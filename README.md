# DOTS Family Mode

DOTS Family Mode is a parental control and child safety system for Linux
desktops. It is written in Rust and stores its data locally. It provides
content filtering, application controls, time management and activity
monitoring.

## Status

Early beta. The workspace builds, and the daemon starts and registers
`org.dots.FamilyDaemon` on the D-Bus system bus. It is not production ready.
CI is red for formatting, clippy, build and tests: of 27 recorded runs between
2026-01-26 and 2026-02-07, all 27 failed, and the last run failed Build, Clippy,
Test Suite, Rustfmt and Coverage. No test result is reported here as passing.
The eBPF monitors are prebuilt binaries in `prebuilt-ebpf/`; CI does not build
them from source.

## Architecture

The repository is a Cargo workspace. The main components are:

- **dots-family-daemon** — core service. Owns the policy engine, serves
  `org.dots.FamilyDaemon` on the system bus and writes to SQLite.
- **dots-family-monitor** — per-user activity and window tracking.
- **dots-family-filter** — HTTP/HTTPS content filtering proxy.
- **dots-terminal-filter** — shell command filtering.
- **dots-family-ctl** — command-line administration tool.
- **dots-family-gui** — GTK4 parent dashboard (present in the workspace, not
  enabled by the NixOS module).
- **dots-wm-bridge** — window manager integration for Niri, Sway and Hyprland.

Supporting crates hold the protocol definitions (`dots-family-proto`), shared
types (`dots-family-common`) and the SQLite layer (`dots-family-db`).

The eBPF monitors (`process-monitor`, `filesystem-monitor`, `network-monitor`,
`memory-monitor`, `disk-io-monitor`) are separate binaries. They are shipped
prebuilt and are not compiled by CI.

## Installation

### NixOS flake

Add the repository as a flake input:

```nix
{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    dots-family-mode.url = "github:dots-distro/dots-family-mode";
  };

  outputs = { self, nixpkgs, dots-family-mode, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ./configuration.nix
        dots-family-mode.nixosModules.default
      ];
    };
  };
}
```

The module is `nixos-modules/dots-family/default.nix`. Its option namespace is
`services.dots-family`; there is no `services.dots-family-mode` namespace.
Minimal configuration:

```nix
{ ... }:
{
  services.dots-family = {
    enable = true;

    # Accounts that already exist in this configuration.
    parentUsers = [ "mom" "dad" ];
    childUsers = [ "alice" ];

    databasePath = "/var/lib/dots-family/family.db";
    reportingOnly = true;       # log activity, do not enforce limits yet
    enableWebFiltering = true;

    profiles.alice = {
      name = "Alice";
      ageGroup = "8-12";
      dailyScreenTimeLimit = "2h30m";
      timeWindows = [
        {
          start = "15:00";
          end = "17:30";
          days = [ "mon" "tue" "wed" "thu" "fri" ];
        }
      ];
    };
  };
}
```

Apply it with:

```bash
sudo nixos-rebuild switch
```

The module creates the `dots-family-daemon` systemd service and the D-Bus
service files itself, so no manual `systemctl enable` is needed.

If the module directory is vendored into your configuration rather than used
as a flake input, import it by path:

```nix
{ imports = [ ./nixos-modules/dots-family ]; }
```

### Development build

```bash
git clone https://github.com/dots-distro/dots-family-mode.git
cd dots-family-mode
nix develop
cargo build --workspace --release
```

The binaries are written to `target/release/`, for example
`target/release/dots-family-daemon` and `target/release/dots-family-ctl`.

## Quick Start

1. Clone the repository and enter the development shell: `nix develop`.
2. Build: `cargo build --workspace --release`.
3. Create a profile and time windows with `dots-family-ctl`.
4. Start the daemon and begin monitoring.

## Getting Help

- **Documentation**: [docs/INDEX.md](docs/INDEX.md)
- **Issues**: https://github.com/dots-distro/dots-family-mode/issues
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md)
- **Security**: [SECURITY.md](SECURITY.md)

## Licensing

The workspace declares `AGPL-3.0-or-later` (`Cargo.toml`). The full licence
text is in [LICENSE](LICENSE). The dual-licence summary and the commercial
contact are in [LICENSING.md](LICENSING.md).

---

**Repository**: https://github.com/dots-distro/dots-family-mode
**License**: AGPL-3.0-or-later; commercial licensing available
**Status**: early beta — see [Status](#status)
