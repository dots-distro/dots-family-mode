# DOTS Family Mode

DOTS Family Mode is a comprehensive parental control and child safety system designed for Linux desktop environments. Built natively in Rust, it provides robust content filtering, application controls, time management, and activity monitoring while maintaining privacy through local-only operation.

## Quick Start

To get started with DOTS Family Mode, you need to have Nix installed.

1.  **Enter the development environment:**
    ```bash
    nix develop
    ```

2.  **Build all components:**
    ```bash
    nix build .#default
    ```

3.  **Run the test suite:**
    ```bash
    nix run .#test
    ```

## Documentation

For detailed information about the architecture, features, and development, please refer to the [documentation](./docs/INDEX.md).
