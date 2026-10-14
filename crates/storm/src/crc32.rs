/// The reflected CRC-32 (polynomial `0xedb88320`) of `data`, as Storm's `0x662860` computes it with
/// its flags 1 (start from `0xffffffff`), 2 (fold the buffer in) and 4 (invert the result):
/// `crc = (crc >> 8) ^ table[(crc ^ byte) & 0xff]` per byte, with the table at `0x866bd8`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc = (crc >> 8) ^ CRC32_TABLE[((crc ^ u32::from(byte)) & 0xff) as usize];
    }
    !crc
}

/// The lookup table, generated from the polynomial; the exe's table at `0x866bd8` holds the same
/// 256 entries (`table[1]` is `0x77073096`).
static CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut n = 0usize;
    while n < 256 {
        let mut c = n as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        table[n] = c;
        n += 1;
    }
    table
};
