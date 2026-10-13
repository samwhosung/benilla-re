# benilla-re

The reverse engineering of the World of Warcraft 1.12.1 client (`WoW.exe`, build 5875), read at
its bytes and cited by address. It is the reference behind
[benilla](https://github.com/samwhosung/benilla), a from-scratch 1.12.1 client in Rust, and it is
written for anyone else who needs to know what the client does: another client, a server, a
client mod, a format reader.

Bring your own client. Nothing here comes from a WoW install. The tools read `WoW/WoW.exe` from
your own copy of 1.12.1.5875 and refuse any other build.

## What is here

- **`symbols/`**: one row per function, global, struct and field read so far, with its name,
  owning system, signature, a one-line meaning, how sure that is (`verified` at the bytes, or
  `inferred`), and the finding that derives it.
- **`systems/`**: what each system does, one entry per mechanism, each with the addresses that
  decide it and its derivation. Written so far:
  [core](systems/core) (main loop, events, window input, clock),
  [net](systems/net) (the wire, dispatch, session, logon),
  [collision](systems/collision) (movement physics),
  [terrain](systems/terrain) (the world map, chunks, liquid, queries, cull and LOD),
  [models](systems/models) (M2 and WMO, the scene, rendering, animation, lights, particles),
  [objectlayer](systems/objectlayer) (the update packet, objects, fields, reaction),
  [spell](systems/spell), [glue](systems/glue) (the login and character screens),
  [ui](systems/ui) (the frame and layout core),
  [camera](systems/camera.md), [input](systems/input.md) and [random](systems/random.md).
  The rest of the client is open work in the issue tracker.
- **`tables/`**: facts a program reads (opcode handlers, events and their fire sites, CVars,
  Lua bindings, the message catalogue, DBC stores and layouts), each rebuilt from your exe by a
  generator in `tools/tables/`.
- **`tools/`**: the listing the findings cite (`tools/disasm.sh`), a Ghidra project with this
  repo's names, signatures and types applied (`tools/ghidra/setup.sh`), and the table generators.

## How to trust it

[METHOD.md](METHOD.md) says what counts as evidence: the binary decides, every claim states its
confidence and proof, and a contradiction is settled at the bytes. `cargo xtask check` holds the
repo to it: the formats, every address a doc cites having a row, every row's finding naming its
address, and every row of a written system having a class; with `--exe`, every function's size
against its body in the listing and every table against its generator. It does not walk a
system's closure, and the bit-exact tests that complete one are not ported yet
([#4](https://github.com/samwhosung/benilla-re/issues/4)).

## Contributing

Names, findings, corrections and checks are welcome: [CONTRIBUTING.md](CONTRIBUTING.md) covers
the setup and how a change is judged.

## License

MIT OR Apache-2.0, at your option.
