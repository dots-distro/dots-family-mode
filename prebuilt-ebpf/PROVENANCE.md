# Provenance of the prebuilt eBPF monitor binaries

This directory contains five compiled eBPF ELF objects that are checked into the
repository as build inputs for the `dots-family-ebpf` Nix package. They are
kernel-side bytecode, not userspace programs: the daemon loads them with `aya`
through the `BPF_*_MONITOR_PATH` environment variables or the compile-time paths
injected by `crates/dots-family-daemon/build.rs`.

## Binary to source mapping

| Binary | Source file | Kernel hook |
| --- | --- | --- |
| `process-monitor` | `crates/dots-family-ebpf/src/process-monitor.rs` | tracepoints |
| `network-monitor` | `crates/dots-family-ebpf/src/network-monitor.rs` | kprobes |
| `filesystem-monitor` | `crates/dots-family-ebpf/src/filesystem-monitor.rs` | kprobes |
| `memory-monitor` | `crates/dots-family-ebpf/src/memory-monitor.rs` | tracepoints |
| `disk-io-monitor` | `crates/dots-family-ebpf/src/disk-io-monitor.rs` | tracepoints |

## Toolchain

The binaries were produced from the crate at `crates/dots-family-ebpf`, which is
a standalone Cargo workspace and is commented out of the root workspace in
`Cargo.toml` (line 14). The crate pins the following:

- `rust-toolchain.toml`: nightly channel with the `rust-src` component and the
  `bpfel-unknown-none` target.
- `aya-ebpf` 0.1.1, `aya-log-ebpf` 0.1, `network-types` 0.0.5 (see
  `crates/dots-family-ebpf/Cargo.lock`).

The exact nightly rustc version and commit that produced these blobs were not
recorded at the time of the build. A rebuild with a different nightly may
produce different bytes; the checksums pin the committed artifacts, not the
compiler.

## How to rebuild

```bash
cd crates/dots-family-ebpf
cargo build --release --target bpfel-unknown-none -Z build-std=core
cp target/bpfel-unknown-none/release/process-monitor \
   target/bpfel-unknown-none/release/network-monitor \
   target/bpfel-unknown-none/release/filesystem-monitor \
   target/bpfel-unknown-none/release/memory-monitor \
   target/bpfel-unknown-none/release/disk-io-monitor \
   ../../prebuilt-ebpf/
cd ../..
sha256sum prebuilt-ebpf/disk-io-monitor prebuilt-ebpf/filesystem-monitor \
  prebuilt-ebpf/memory-monitor prebuilt-ebpf/network-monitor \
  prebuilt-ebpf/process-monitor > prebuilt-ebpf/SHA256SUMS
sha256sum -c prebuilt-ebpf/SHA256SUMS
```

The Nix package `dots-family-ebpf` in `flake.nix` runs the same checksum
verification in its install phase (after stripping the `prebuilt-ebpf/` path
prefix), so a corrupted or silently replaced blob fails the build instead of
being installed.

## Verification limits

The checksums in `SHA256SUMS` were recorded from the committed blobs on
2026-11-08. They prove that a blob has not changed since the checksums were
recorded. They do **not** prove that a blob was produced from the source files
listed above: the repository has no reproducible build or recorded CI artifact
for these objects, and the compiler version is not pinned.

**These binaries cannot be verified from the repository alone.** To trust a
blob, rebuild it from the listed source with a known toolchain and compare the
result, or load it only in an environment where kernel-side monitoring is an
accepted risk.
