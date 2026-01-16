#!/usr/bin/env bash

set -e

echo "Starting DOTS Family Mode Integration Test"

pkill -f dots-family-daemon || true
pkill -f dots-family-monitor || true
sleep 2

echo "Starting daemon..."
RUST_LOG=debug cargo run -p dots-family-daemon &
DAEMON_PID=$!
echo "Daemon started with PID: $DAEMON_PID"

sleep 3

echo "Testing CLI connectivity..."
cargo run -p dots-family-ctl -- status || true

echo "Creating test profile..."
cargo run -p dots-family-ctl -- profile create "test-child" "8-12" || true

echo "Listing profiles..."
cargo run -p dots-family-ctl -- profile list || true

echo "Testing application check..."
cargo run -p dots-family-ctl -- check firefox || true

echo "Starting monitor for 10 seconds..."
timeout 10 cargo run -p dots-family-monitor || true

echo "Integration test completed"

echo "Cleaning up..."
kill $DAEMON_PID 2>/dev/null || true
wait $DAEMON_PID 2>/dev/null || true

echo "Test finished"