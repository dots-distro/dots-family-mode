#!/bin/bash
set -e

echo "Building NixOS VM..."
nix build --no-link .#nixosConfigurations.dots-family-test.config.system.build.vm

echo "Starting VM..."
./result/bin/run-dots-family-test-vm -m 4096 -smp 4
