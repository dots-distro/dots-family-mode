{
  description = "Family safety and parental controls for dots NixOS desktop distro";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        nativeBuildInputs = with pkgs; [
          rustToolchain
          pkg-config
          makeWrapper
          cargo-tarpaulin
          cargo-deny
        ];

        buildInputs = with pkgs; [
          openssl
          sqlite
          sqlcipher
          dbus
          gtk4
          libadwaita
        ];

        # Runtime dependencies for family mode components
        runtimeDependencies = with pkgs; [
          procps        # Process monitoring
          util-linux    # System utilities
          dbus          # Inter-process communication
        ];

      in
      {
        packages = {
          # Individual crate packages
          dots-family-daemon = pkgs.rustPlatform.buildRustPackage {
            pname = "dots-family-daemon";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            buildAndTestSubdir = "crates/dots-family-daemon";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;

            postInstall = ''
              wrapProgram $out/bin/dots-family-daemon \
                --prefix PATH : ${pkgs.lib.makeBinPath runtimeDependencies}
            '';

            meta = with pkgs.lib; {
              description = "Family safety daemon for dots NixOS";
              homepage = "https://github.com/dots-distro/dots-family-mode";
              license = licenses.agpl3Plus;
              maintainers = [ ];
            };
          };

          dots-family-monitor = pkgs.rustPlatform.buildRustPackage {
            pname = "dots-family-monitor";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            buildAndTestSubdir = "crates/dots-family-monitor";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;

            postInstall = ''
              wrapProgram $out/bin/dots-family-monitor \
                --prefix PATH : ${pkgs.lib.makeBinPath runtimeDependencies}
            '';

            meta = with pkgs.lib; {
              description = "Activity monitor for dots family mode";
              homepage = "https://github.com/dots-distro/dots-family-mode";
              license = licenses.agpl3Plus;
              maintainers = [ ];
            };
          };

          dots-family-ctl = pkgs.rustPlatform.buildRustPackage {
            pname = "dots-family-ctl";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            buildAndTestSubdir = "crates/dots-family-ctl";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;

            meta = with pkgs.lib; {
              description = "CLI control tool for dots family mode";
              homepage = "https://github.com/dots-distro/dots-family-mode";
              license = licenses.agpl3Plus;
              maintainers = [ ];
            };
          };

          dots-family-gui = pkgs.rustPlatform.buildRustPackage {
            pname = "dots-family-gui";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            buildAndTestSubdir = "crates/dots-family-gui";

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;

            meta = with pkgs.lib; {
              description = "GUI dashboard for dots family mode";
              homepage = "https://github.com/dots-distro/dots-family-mode";
              license = licenses.agpl3Plus;
              maintainers = [ ];
            };
          };

          # Default package builds all workspace members
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "dots-family-mode";
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;

            postInstall = ''
              # Wrap all binaries with runtime dependencies
              for bin in $out/bin/*; do
                wrapProgram $bin \
                  --prefix PATH : ${pkgs.lib.makeBinPath runtimeDependencies}
              done
            '';

            meta = with pkgs.lib; {
              description = "Family safety and parental controls for dots NixOS";
              homepage = "https://github.com/dots-distro/dots-family-mode";
              license = licenses.agpl3Plus;
              maintainers = [ ];
            };
          };
        };

        devShells.default = pkgs.mkShell {
          nativeBuildInputs = nativeBuildInputs ++ [ pkgs.pre-commit ];
          buildInputs = buildInputs ++ runtimeDependencies;

          shellHook = ''
            echo "dots-family-mode development environment"
            echo "Multi-crate workspace with 9 crates"
            echo ""
            echo "Common commands:"
            echo "  cargo build                    - Build all workspace members"
            echo "  cargo test                     - Test all workspace members"
            echo "  cargo build -p dots-family-ctl - Build specific crate"
            echo "  cargo run -p dots-family-ctl   - Run specific crate"
            echo ""
            echo "Development tools:"
            echo "  cargo tarpaulin --out Html     - Generate test coverage"
            echo "  cargo deny check               - Audit dependencies"
            echo "  cargo clippy --all-features -- -D warnings"
            echo ""
            echo "Available runtime tools:"
            echo "  - procps (process monitoring)"
            echo "  - util-linux (system utilities)"
            echo "  - dbus (inter-process communication)"
            echo ""
            echo "Workspace structure:"
            echo "  dots-family-common        - Common types and utilities"
            echo "  dots-family-proto         - DBus protocol definitions"
            echo "  dots-family-daemon        - Policy enforcement daemon"
            echo "  dots-family-monitor       - Activity monitoring service"
            echo "  dots-family-filter        - Web content filtering"
            echo "  dots-family-ctl           - CLI administration tool"
            echo "  dots-family-gui           - GTK4 parent dashboard"
            echo "  dots-terminal-filter      - Command filtering for terminals"
            echo "  dots-wm-bridge            - Window manager integration"
            echo ""
            echo "NOTE: This is Phase 0 - foundation work in progress"
            echo "      Not all crates are fully implemented yet"
          '';
        };

        # Nix checks - run with 'nix flake check'
        checks = {
          build = self.packages.${system}.default;
          
          test = pkgs.runCommand "test-dots-family-mode" {
            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
          } ''
            cp -r ${./.} source
            chmod -R +w source
            cd source
            cargo test --workspace
            touch $out
          '';

          clippy = pkgs.runCommand "clippy-dots-family-mode" {
            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
          } ''
            cp -r ${./.} source
            chmod -R +w source
            cd source
            cargo clippy --workspace --all-features -- -D warnings
            touch $out
          '';
        };
      }
    );
}
