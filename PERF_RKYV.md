# perf-tmp branch: rkyv + memmap cache instructions

This branch adds a zero-copy cache implementation for CMake builtin completions using rkyv + memmap2.

To build and test locally, add the following dependencies to your Cargo.toml:

[dependencies]
rkyv = "0.7"
memmap2 = "0.5"
# optional: sha2 = "0.10" if you plan to add content-hash checks

Notes:
- The implementation validates rkyv archived roots before deserializing; it uses `unsafe` archived_root only after validation.
- The cache file is written atomically to `<cache_dir>/neocmakelsp_cmake_rkyv_cache.bin` where `cache_dir()` prefers `/dev/shm` on unix systems and falls back to the OS temp dir.
- If you want me to also update Cargo.toml in this branch automatically, confirm and I'll update it directly; otherwise please add the deps manually to avoid accidental merge conflicts.