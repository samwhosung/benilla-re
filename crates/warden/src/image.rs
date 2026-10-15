//! The module image loader `0x7a79d0`, its unloader `0x7a7d80` and the entry lookup `0x7a7de0`
//! (`systems/warden/modules.md`).
//!
//! The loader runs over the decompressed module and makes no bounds check. It allocates `size`
//! bytes, which Windows rounds up to whole pages, so an access past `size` inside the last page
//! succeeds and the model gives it. An access outside the committed pages is a
//! [`LoadError::Outside`]: the client reads or writes other process memory, or faults; the
//! loader runs inside an exception handler (`0x7a79d3` to `0x7a79e6`, scope table `0x81d760`,
//! filter `0x7a7d42` always handles) and a fault ends in the unloader and a return of 0.

/// What the loader asks of the process: `VirtualAlloc`, `LoadLibraryA` and `GetProcAddress`.
pub trait Host {
    /// `VirtualAlloc(0, size, MEM_COMMIT, PAGE_READWRITE)`: the base of `size` zeroed bytes.
    fn alloc(&mut self, size: u32) -> Option<u32>;
    /// `LoadLibraryA(name)`: a module handle, or `None`.
    fn load_library(&mut self, name: &[u8]) -> Option<u32>;
    /// `GetProcAddress(library, name)`.
    fn proc_by_name(&mut self, library: u32, name: &[u8]) -> u32;
    /// `GetProcAddress(library, ordinal)`.
    fn proc_by_ordinal(&mut self, library: u32, ordinal: u32) -> u32;
}

/// A `VirtualProtect` the loader makes after the imports, in section order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Protect {
    pub address: u32,
    pub size: u32,
    pub protection: u32,
    /// The protection has a bit of `0xf0` (an execute right): `FlushInstructionCache` follows.
    pub flush: bool,
}

/// A loaded module: the state of the 12-byte module object and what the loader did to the image.
#[derive(Debug, PartialEq, Eq)]
pub struct Loaded {
    pub base: u32,
    /// The committed pages as they stand when the loader returns: `size` rounded up to a multiple
    /// of `0x1000`.
    pub image: Vec<u8>,
    /// The module object's library count, `+8`.
    pub libraries: u32,
    pub protects: Vec<Protect>,
    /// The `VirtualFree(address, size, MEM_RELEASE)` over the relocation and import data, when the
    /// relocation table starts below the image size.
    pub release: Option<(u32, u32)>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum LoadError {
    /// The first dword is `0x80000000` or more: the client builds `message` and calls
    /// `SErrDisplayAppFatal` with it.
    HeaderTooLarge { message: String },
    /// `VirtualAlloc` returned null.
    AllocFailed,
    /// `LoadLibraryA` returned null; `libraries` libraries had been loaded and are freed by the
    /// unloader.
    LibraryMissing { libraries: u32 },
    /// A read or write of the client's lands outside the blob or the committed pages. What the
    /// client does there depends on the process: memory it can reach is read or written, memory it
    /// cannot faults and the load fails.
    Outside,
}

fn u32_at(b: &[u8], at: u32) -> Result<u32, LoadError> {
    let at = at as usize;
    b.get(at..at + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(LoadError::Outside)
}

fn put_u32(b: &mut [u8], at: u32, v: u32) -> Result<(), LoadError> {
    let at = at as usize;
    b.get_mut(at..at + 4)
        .ok_or(LoadError::Outside)?
        .copy_from_slice(&v.to_le_bytes());
    Ok(())
}

fn cstr(b: &[u8], at: u32) -> Result<&[u8], LoadError> {
    let tail = b.get(at as usize..).ok_or(LoadError::Outside)?;
    let n = tail
        .iter()
        .position(|&c| c == 0)
        .ok_or(LoadError::Outside)?;
    Ok(&tail[..n])
}

/// `0x7a79d0` over `blob`, the decompressed module.
pub fn load(blob: &[u8], host: &mut dyn Host) -> Result<Loaded, LoadError> {
    let size = u32_at(blob, 0)?;
    if size >= 0x8000_0000 {
        let mut message = String::from("Improper header received: [ ");
        for i in 0..0x20 {
            message.push_str(&format!("{:02X} ", blob.get(i).copied().unwrap_or(0)));
        }
        return Err(LoadError::HeaderTooLarge { message });
    }
    let base = host.alloc(size).ok_or(LoadError::AllocFailed)?;
    let mut image = vec![0u8; (size as usize).next_multiple_of(0x1000)];
    let header = blob.get(..0x28).ok_or(LoadError::Outside)?;
    image
        .get_mut(..0x28)
        .ok_or(LoadError::Outside)?
        .copy_from_slice(header);
    let mut libraries = 0u32;

    // Sections: runs, alternately copied and skipped, from the first entry's address to the end.
    let section_count = u32_at(&image, 0x24)?;
    let table = 0x28u32;
    let mut src = table
        .checked_add(section_count.wrapping_mul(12))
        .ok_or(LoadError::Outside)?;
    let mut dst = u32_at(blob, table)?;
    let mut copy = true;
    while dst < size {
        let len = {
            let s = blob
                .get(src as usize..src as usize + 2)
                .ok_or(LoadError::Outside)?;
            u32::from(u16::from_le_bytes([s[0], s[1]]))
        };
        src += 2;
        if copy {
            let from = blob
                .get(src as usize..(src + len) as usize)
                .ok_or(LoadError::Outside)?;
            image
                .get_mut(dst as usize..(dst + len) as usize)
                .ok_or(LoadError::Outside)?
                .copy_from_slice(from);
            src += len;
        }
        dst += len;
        if dst >= size {
            break;
        }
        copy = !copy;
    }

    // Relocations: the stream lives in the image; each entry adds the base to a dword.
    let mut p = u32_at(&image, 8)?;
    let mut offset = 0u32;
    let mut i = 0u32;
    while i < u32_at(&image, 0xc)? {
        let b0 = *image.get(p as usize).ok_or(LoadError::Outside)?;
        if b0 & 0x80 != 0 {
            let be = image
                .get(p as usize..p as usize + 4)
                .ok_or(LoadError::Outside)?;
            offset = u32::from_be_bytes([be[0], be[1], be[2], be[3]]);
            p += 4;
        } else {
            let b1 = *image.get(p as usize + 1).ok_or(LoadError::Outside)?;
            offset = offset
                .wrapping_add(u32::from(b0) << 8)
                .wrapping_add(u32::from(b1));
            p += 2;
        }
        let v = u32_at(&image, offset)?;
        put_u32(&mut image, offset, v.wrapping_add(base))?;
        i += 1;
    }

    // Imports: one entry per library, 8 bytes: the name's address and the slot list's.
    let import_table = u32_at(&image, 0x1c)?;
    let mut n = 0u32;
    while n < u32_at(&image, 0x20)? {
        let entry = import_table.wrapping_add(n.wrapping_mul(8));
        let name = cstr(&image, u32_at(&image, entry)?)?.to_vec();
        let slots = u32_at(&image, entry + 4)?;
        let library = host
            .load_library(&name)
            .ok_or(LoadError::LibraryMissing { libraries })?;
        let mut slot = slots;
        loop {
            let v = u32_at(&image, slot)?;
            if v == 0 {
                break;
            }
            let address = if v & 0x8000_0000 != 0 {
                host.proc_by_ordinal(library, v & 0x7fff_ffff)
            } else {
                let proc_name = cstr(&image, v)?.to_vec();
                host.proc_by_name(library, &proc_name)
            };
            put_u32(&mut image, slot, address)?;
            slot += 4;
        }
        libraries += 1;
        n += 1;
    }

    // Protections, from the section table in the blob.
    let mut protects = Vec::new();
    let mut s = 0u32;
    while s < u32_at(&image, 0x24)? {
        let e = table + s * 12;
        let (rva, len, prot) = (u32_at(blob, e)?, u32_at(blob, e + 4)?, u32_at(blob, e + 8)?);
        protects.push(Protect {
            address: base.wrapping_add(rva),
            size: len,
            protection: prot,
            flush: prot & 0xf0 != 0,
        });
        s += 1;
    }

    // The release reads the size and the relocation address from the image as it stands.
    let reloc = u32_at(&image, 8)?;
    let size_now = u32_at(&image, 0)?;
    let release = (reloc < size_now).then(|| (base.wrapping_add(reloc), size_now - reloc));
    Ok(Loaded {
        base,
        image,
        libraries,
        protects,
        release,
    })
}

/// `0x7a7d80`: the names of the libraries it frees, in the order it frees them (the last loaded
/// first), for a module with `libraries` loaded.
pub fn unload_names(image: &[u8], libraries: u32) -> Result<Vec<Vec<u8>>, LoadError> {
    let table = u32_at(image, 0x1c)?;
    let mut out = Vec::new();
    for i in (0..libraries).rev() {
        let entry = table.wrapping_add(i.wrapping_mul(8));
        out.push(cstr(image, u32_at(image, entry)?)?.to_vec());
    }
    Ok(out)
}

/// `0x7a7de0`: the address of export `ordinal`, or `None` where the client returns 0: the
/// ordinal is below the table's base or past its count.
pub fn export_address(image: &[u8], base: u32, ordinal: u32) -> Result<Option<u32>, LoadError> {
    let index = ordinal.wrapping_sub(u32_at(image, 0x18)?);
    if index >= u32_at(image, 0x14)? {
        return Ok(None);
    }
    let table = u32_at(image, 0x10)?;
    let rva = u32_at(image, table.wrapping_add(index.wrapping_mul(4)))?;
    Ok(Some(base.wrapping_add(rva)))
}
