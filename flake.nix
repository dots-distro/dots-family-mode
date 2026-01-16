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
    let
      # NixOS modules for cross-system support
      nixosModules = {
        dots-family = import ./nixos-modules/dots-family/default.nix;
        
        default = nixosModules.dots-family;
      };
    in
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
          libnotify     # Desktop notifications
          polkit        # Authentication framework
        ];

        # Helper function to build individual crate packages
        buildCrate = { pname, subdir ? "crates/${pname}", doCheck ? false }: 
          pkgs.rustPlatform.buildRustPackage {
            inherit pname doCheck;
            version = "0.1.0";

            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };

            buildAndTestSubdir = subdir;

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;
            
            # Disable SQLx compile-time checks for Nix build
            SQLX_OFFLINE = "true";

            postInstall = ''
              # Wrap binaries with runtime dependencies
              for bin in $out/bin/*; do
                wrapProgram $bin \
                  --prefix PATH : ${pkgs.lib.makeBinPath runtimeDependencies}
              done
            '';

            meta = with pkgs.lib; {
              description = "${pname} component for DOTS Family Mode";
              homepage = "https://github.com/dots-distro/dots-family-mode";
              license = licenses.agpl3Plus;
              maintainers = [ ];
            };
          };

      in
      {
        packages = {
          # Individual crate packages using helper function
          dots-family-daemon = buildCrate { pname = "dots-family-daemon"; };
          dots-family-monitor = buildCrate { pname = "dots-family-monitor"; };
          dots-family-ctl = buildCrate { pname = "dots-family-ctl"; doCheck = true; };
          dots-family-gui = buildCrate { pname = "dots-family-gui"; };
          dots-family-filter = buildCrate { pname = "dots-family-filter"; };
          dots-terminal-filter = buildCrate { pname = "dots-terminal-filter"; };

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
            
            # Disable SQLx compile-time checks
            SQLX_OFFLINE = "true";
            
            # Skip tests for full workspace build (some are integration tests)
            doCheck = false;

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
    ) // {
      # Export NixOS modules for system integration
      inherit nixosModules;
      
      # NixOS VM configurations for testing
      nixosConfigurations = {
        dots-family-test-vm = nixpkgs.lib.nixosSystem {
          system = "x86_64-linux";
          modules = [
            ./vm-simple.nix
            nixosModules.dots-family
            {
              # Enable DOTS Family Mode with test configuration
              services.dots-family = {
                enable = true;
                parentUsers = [ "parent" ];
                childUsers = [ "child" ];
                reportingOnly = true;  # Safe mode for testing
                
                profiles.child = {
                  name = "Test Child";
                  ageGroup = "8-12";
                  dailyScreenTimeLimit = "2h";
                  timeWindows = [{
                    start = "09:00";
                    end = "17:00";
                    days = [ "mon" "tue" "wed" "thu" "fri" ];
                  }];
                  allowedApplications = [ "firefox" "calculator" ];
                  webFilteringLevel = "moderate";
                };
              };
            }
          ];
        };
      };
    };
}
