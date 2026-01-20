# Guide: Building Rust Projects with `crane` and Nix

This document explains how to correctly build Rust projects that depend on native C-libraries (like OpenSSL) using the `crane` Nix library.

## The Problem

When running `nix flake check`, the build may fail with an error related to a missing native dependency, such as OpenSSL. The error message might look like this:

```
Could not find directory of OpenSSL installation...
...
It looks like you're compiling on Linux and also targeting Linux. Currently this
requires the `pkg-config` utility to find OpenSSL but unfortunately `pkg-config`
could not be found.
```

This happens even if `openssl` and `pkg-config` are present in your `flake.nix`'s `buildInputs`.

## Root Cause

The `crane` library separates the building of dependencies from the building of the final crate. The `craneLib.buildDepsOnly` function is used to compile and cache all the dependencies listed in `Cargo.lock`. This step runs in a sandboxed Nix environment.

The error occurs because the environment for `buildDepsOnly` does not have access to the necessary native libraries and tools (like `openssl` and `pkg-config`). These need to be explicitly passed to it.

## The Solution

The solution is to pass the `nativeBuildInputs` (which should include `pkg-config`) and `buildInputs` (which should include `openssl`) to the `craneLib.buildDepsOnly` function in your `flake.nix`.

### Example `flake.nix` Fix

**Before:**
```nix
let
  # ...
  src = ...;
  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src;
  };
in
# ...
```

**After:**
```nix
let
  # ...
  src = ...;
  nativeBuildInputs = with pkgs; [ pkg-config ... ];
  buildInputs = with pkgs; [ openssl ... ];

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src nativeBuildInputs buildInputs;
  };
in
# ...
```

By adding `nativeBuildInputs` and `buildInputs` to `cargoArtifacts`, you ensure that when `crane` builds the dependencies, it has everything it needs to compile and link native libraries, resolving the build failure.
