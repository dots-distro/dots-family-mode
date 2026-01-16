{
  description = "Family safety and parental controls for dots NixOS desktop distro";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, rust-overlay, crane }:
    let
      # Create overlay for our packages
      dotsOverlay = final: prev: {
        dots-family-daemon = self.packages.${final.system}.dots-family-daemon;
        dots-family-monitor = self.packages.${final.system}.dots-family-monitor;
        dots-family-ctl = self.packages.${final.system}.dots-family-ctl;
        dots-terminal-filter = self.packages.${final.system}.dots-terminal-filter;
      };
    
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

        # Multi-stage eBPF build setup
        # Use actual nightly Rust for eBPF compilation with -Z build-std
        rustToolchainNightly = pkgs.rust-bin.nightly.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" "llvm-tools-preview" ];
        };

        # Stage 2: User-space applications (stable Rust)
        rustToolchainStable = pkgs.rust-bin.stable.latest.default.override {
          extensions = [ "rust-src" "rust-analyzer" ];
        };

        # Crane for eBPF build (nightly)
        craneLibEbpf = (crane.mkLib pkgs).overrideToolchain rustToolchainNightly;
        
        # Crane for user-space build (stable)
        craneLib = (crane.mkLib pkgs).overrideToolchain rustToolchainStable;

        # Common source filter
        src = craneLib.cleanCargoSource (craneLib.path ./.);

        # Common build inputs
        nativeBuildInputs = with pkgs; [
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
          elfutils
          zlib
        ];

        # Runtime dependencies for family mode components
        runtimeDependencies = with pkgs; [
          procps        # Process monitoring
          util-linux    # System utilities
          dbus          # Inter-process communication
          libnotify     # Desktop notifications
          polkit        # Authentication framework
        ];

        # Stage 1: eBPF Programs (kernel-space)
        # Build eBPF programs using crane with proper dependency management
        dots-family-ebpf = let
          # Create the eBPF source filter to include Cargo.toml and source files
          ebpfSrc = pkgs.lib.cleanSourceWith {
            src = ./dots-family-ebpf;
            filter = path: type:
              (pkgs.lib.hasSuffix "Cargo.toml" path) ||
              (pkgs.lib.hasSuffix "Cargo.lock" path) ||
              (pkgs.lib.hasSuffix ".rs" path) ||
              (type == "directory");
          };
        in craneLibEbpf.buildPackage {
          pname = "dots-family-ebpf";
          version = "0.1.0";
          src = ebpfSrc;
          
          # Set up for eBPF target compilation
          CARGO_BUILD_TARGET = "bpfel-unknown-none";
          CARGO_BUILD_RUSTFLAGS = "-C link-arg=--no-rosegment";
          
          # Use build-std for the eBPF target
          cargoExtraArgs = "--target bpfel-unknown-none --release -Z build-std=core";
          
          # Add LLVM tools for eBPF
          nativeBuildInputs = with pkgs; [ 
            llvmPackages.clang 
            llvmPackages.llvm 
          ];
          
          # Don't run tests for eBPF programs
          doCheck = false;
          
          # Install eBPF binaries to expected locations
          installPhase = ''
            mkdir -p $out/target/bpfel-unknown-none/release
            cp target/bpfel-unknown-none/release/process-monitor $out/target/bpfel-unknown-none/release/
            cp target/bpfel-unknown-none/release/network-monitor $out/target/bpfel-unknown-none/release/
            cp target/bpfel-unknown-none/release/filesystem-monitor $out/target/bpfel-unknown-none/release/
          '';
        };

        # Helper function to build user-space crate packages with eBPF support
        buildCrateWithEbpf = { pname, subdir ? "crates/${pname}", doCheck ? false, hasEbpf ? false }: 
          craneLib.buildPackage {
            inherit pname doCheck;
            version = "0.1.0";

            src = src;

            buildAndTestSubdir = subdir;

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;
            
            # Disable SQLx compile-time checks for Nix build
            SQLX_OFFLINE = "true";
            
            # eBPF compilation environment for user-space
            KERNEL_HEADERS = "${pkgs.linuxHeaders}/include";
            LIBBPF_INCLUDE_PATH = "${pkgs.libbpf}/include";
            LIBBPF_LIB_PATH = "${pkgs.libbpf}/lib";
            BPF_CLANG_PATH = "${pkgs.clang}/bin/clang";

            # Inject eBPF ELF paths for daemon
            BPF_PROCESS_MONITOR_PATH = if hasEbpf then "${dots-family-ebpf}/target/bpfel-unknown-none/release/process-monitor" else "";
            BPF_NETWORK_MONITOR_PATH = if hasEbpf then "${dots-family-ebpf}/target/bpfel-unknown-none/release/network-monitor" else "";
            BPF_FILESYSTEM_MONITOR_PATH = if hasEbpf then "${dots-family-ebpf}/target/bpfel-unknown-none/release/filesystem-monitor" else "";

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
          # eBPF programs (Stage 1)
          inherit dots-family-ebpf;
          
          # Core packages with eBPF support (Stage 2)
          dots-family-daemon = buildCrateWithEbpf { pname = "dots-family-daemon"; hasEbpf = true; };
          dots-family-monitor = buildCrateWithEbpf { pname = "dots-family-monitor"; };
          dots-family-ctl = buildCrateWithEbpf { pname = "dots-family-ctl"; doCheck = true; };
          dots-terminal-filter = buildCrateWithEbpf { pname = "dots-terminal-filter"; };
          
          # Default package builds all workspace members
          default = craneLib.buildPackage {
            pname = "dots-family-mode";
            version = "0.1.0";

            src = src;

            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs ++ runtimeDependencies;
            
            # Disable SQLx compile-time checks
            SQLX_OFFLINE = "true";
            
            # eBPF compilation environment
            KERNEL_HEADERS = "${pkgs.linuxHeaders}/include";
            LIBBPF_INCLUDE_PATH = "${pkgs.libbpf}/include";
            LIBBPF_LIB_PATH = "${pkgs.libbpf}/lib";
            BPF_CLANG_PATH = "${pkgs.clang}/bin/clang";
            
            # Inject eBPF ELF paths
            BPF_PROCESS_MONITOR_PATH = "${dots-family-ebpf}/target/bpfel-unknown-none/release/process-monitor";
            BPF_NETWORK_MONITOR_PATH = "${dots-family-ebpf}/target/bpfel-unknown-none/release/network-monitor";
            BPF_FILESYSTEM_MONITOR_PATH = "${dots-family-ebpf}/target/bpfel-unknown-none/release/filesystem-monitor";
            
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
          nativeBuildInputs = nativeBuildInputs ++ [ 
            pkgs.pre-commit
            # Both toolchains for development
            rustToolchainStable
            rustToolchainNightly
            # eBPF development tools
            pkgs.bpf-linker
            pkgs.cargo-generate
          ];
          buildInputs = buildInputs ++ runtimeDependencies ++ [
            pkgs.linuxHeaders
            pkgs.libbpf
            pkgs.llvm
            pkgs.clang
          ];

          shellHook = ''
            echo "dots-family-mode development environment"
            echo "Multi-crate workspace with eBPF support"
            echo ""
            
            # eBPF environment setup
            export KERNEL_HEADERS="${pkgs.linuxHeaders}/include"
            export LIBBPF_INCLUDE_PATH="${pkgs.libbpf}/include"
            export LIBBPF_LIB_PATH="${pkgs.libbpf}/lib"
            export BPF_CLANG_PATH="${pkgs.clang}/bin/clang"
            export RUST_SRC_PATH="${rustToolchainNightly}/lib/rustlib/src/rust/library"
            
            echo "eBPF compilation environment configured:"
            echo "  Rust stable: $(rustc --version)"
            echo "  Rust nightly: $(rustup run nightly rustc --version 2>/dev/null || echo 'Use rustup for nightly')"
            echo "  Target: bpfel-unknown-unknown available in nightly"
            echo "  Clang: ${pkgs.clang}/bin/clang"
            echo "  LLVM: ${pkgs.llvm}"
            echo "  bpf-linker: ${pkgs.bpf-linker}/bin/bpf-linker"
            echo ""
            
            echo "Common commands:"
            echo "  cargo build                    - Build all workspace members (user-space)"
            echo "  nix build .#dots-family-ebpf   - Build eBPF programs (kernel-space)"
            echo "  nix build .#dots-family-daemon - Build daemon with embedded eBPF"
            echo "  cargo test                     - Test user-space code"
            echo ""
            echo "Development tools:"
            echo "  cargo tarpaulin --out Html     - Generate test coverage"
            echo "  cargo deny check               - Audit dependencies"
            echo "  cargo clippy --all-features -- -D warnings"
            echo ""
            echo "eBPF Development:"
            echo "  cd dots-family-ebpf && cargo build --target bpfel-unknown-none -Z build-std=core"
            echo ""
            echo "Workspace structure:"
            echo "  dots-family-common        - Common types and utilities"
            echo "  dots-family-proto         - DBus protocol definitions"
            echo "  dots-family-daemon        - Policy enforcement daemon (uses eBPF)"
            echo "  dots-family-monitor       - Activity monitoring service"
            echo "  dots-family-filter        - Web content filtering"
            echo "  dots-family-ctl           - CLI administration tool"
            echo "  dots-family-gui           - GTK4 parent dashboard"
            echo "  dots-terminal-filter      - Command filtering for terminals"
            echo "  dots-wm-bridge            - Window manager integration"
            echo "  dots-family-ebpf          - eBPF programs for kernel monitoring"
          '';
        };

        # Nix checks - run with 'nix flake check'
        checks = {
          build = self.packages.${system}.default;
          ebpf-build = self.packages.${system}.dots-family-ebpf;
          
          test = pkgs.runCommand "test-dots-family-mode" {
            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
            KERNEL_HEADERS = "${pkgs.linuxHeaders}/include";
            LIBBPF_INCLUDE_PATH = "${pkgs.libbpf}/include";
            LIBBPF_LIB_PATH = "${pkgs.libbpf}/lib";
            BPF_CLANG_PATH = "${pkgs.clang}/bin/clang";
            SQLX_OFFLINE = "true";
          } ''
            cp -r ${src} source
            chmod -R +w source
            cd source
            cargo test --workspace
            touch $out
          '';

          clippy = pkgs.runCommand "clippy-dots-family-mode" {
            nativeBuildInputs = nativeBuildInputs;
            buildInputs = buildInputs;
            KERNEL_HEADERS = "${pkgs.linuxHeaders}/include";
            LIBBPF_INCLUDE_PATH = "${pkgs.libbpf}/include";
            LIBBPF_LIB_PATH = "${pkgs.libbpf}/lib";
            BPF_CLANG_PATH = "${pkgs.clang}/bin/clang";
            SQLX_OFFLINE = "true";
          } ''
            cp -r ${src} source
            chmod -R +w source
            cd source
            cargo clippy --workspace --all-features -- -D warnings
            touch $out
          '';
        };
      }
    ) // {
      # Export overlays
      overlays.default = dotsOverlay;
      
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
              # Override the package set to include our DOTS Family packages
              nixpkgs.overlays = [
                (final: prev: {
                  dots-family-daemon = self.packages.x86_64-linux.dots-family-daemon;
                  dots-family-monitor = self.packages.x86_64-linux.dots-family-monitor;
                  dots-family-ctl = self.packages.x86_64-linux.dots-family-ctl;
                  dots-terminal-filter = self.packages.x86_64-linux.dots-terminal-filter;
                })
              ];

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