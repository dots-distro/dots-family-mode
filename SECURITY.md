# Security Policy

## Supported versions

Only `0.1.x` is supported. Older versions do not receive fixes.

## Reporting a vulnerability

Email **shift@someone.section.me** with the subject `DOTS Family Mode security`.
Do not open a public issue or put exploit details in a pull request.

Include the affected component and version or commit, reproduction steps, the
impact you observed, proof-of-concept code or logs with secrets removed, and how
you want to be credited (or a request to stay anonymous).

## Disclosure

We aim to acknowledge within 7 days and to fix or publish an advisory within 90
days. That window is an expectation, not a guarantee: no advisory feed, CVE
process or automated disclosure tooling is implemented yet.

## Scope

- **eBPF monitors** — binaries in `prebuilt-ebpf/`. They ship prebuilt; a
  reproducible from-source build and signed releases are not implemented yet.
  Loading them needs `cap_bpf` and `cap_sys_admin`.
- **HTTPS interception CA** — the CA from
  `nixos-modules/dots-family/ssl-intercept.nix`. Its private key is created on
  the host. Hardware-backed storage, rotation and revocation are not
  implemented yet.
- **Local database** — the SQLite file at `databasePath`. Access is limited by
  file permissions only. At-rest encryption is not implemented yet: the daemon
  opens the database with no key, and the SQLCipher pragmas in
  `dots-family-db` are not enabled against a SQLCipher-linked SQLite build.
- **D-Bus surface** — `org.dots.FamilyDaemon` on the system bus, the monitor's
  session-bus service and the polkit rules in `nixos-modules/dots-family/`. No
  fuzzing or independent audit of this interface has been published.

Out of scope: bugs in the kernel, Nixpkgs or upstream Rust crates; report those
to their maintainers.
