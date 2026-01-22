#!/usr/bin/env bash

set -e

echo "Running DOTS Family Mode Test Suite"
echo "==================================="

TEST_DIR="$(dirname "$0")"
cd "$TEST_DIR/.."

export DATABASE_URL="sqlite:///tmp/test-family.db"
export RUST_LOG=info
export RUST_BACKTRACE=1

echo "1. Testing compilation..."
cargo build --workspace --release

echo "2. Running unit tests..."
cargo test --workspace

echo "3. Running integration tests..."
if [ "$1" = "--integration" ] || [ "$1" = "--all" ]; then
    chmod +x scripts/tests/integration-test.sh
    ./scripts/tests/integration-test.sh
fi

echo "4. Running performance tests..."
if [ "$1" = "--performance" ] || [ "$1" = "--all" ]; then
    chmod +x scripts/tests/performance-test.sh  
    ./scripts/tests/performance-test.sh
fi

echo ""
echo "🎉 All requested tests completed successfully!"
echo ""
echo "Usage:"
echo "  ./scripts/tests/run-all-tests.sh                    # Compilation + unit tests only"
echo "  ./scripts/tests/run-all-tests.sh --integration      # Add integration tests"
echo "  ./scripts/tests/run-all-tests.sh --performance      # Add performance tests" 
echo "  ./scripts/tests/run-all-tests.sh --all              # Run everything"