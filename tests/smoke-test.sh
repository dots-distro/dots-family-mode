#!/usr/bin/env bash
# DOTS Family Mode - Ultra Simple Smoke Test

set -e

echo "DOTS Family Mode - Smoke Test"
echo "=============================="
echo ""

tests_passed=0
tests_total=0

# Test 1
tests_total=$((tests_total + 1))
if [ -x "/run/current-system/sw/bin/dots-family-daemon" ]; then
    echo "[PASS] daemon binary exists"
    tests_passed=$((tests_passed + 1))
fi

# Test 2  
tests_total=$((tests_total + 1))
if [ -x "/run/current-system/sw/bin/dots-family-ctl" ]; then
    echo "[PASS] ctl binary exists"
    tests_passed=$((tests_passed + 1))
fi

# Test 3
if [ -d "prebuilt-ebpf" ]; then
    tests_total=$((tests_total + 1))
    count=$(ls prebuilt-ebpf/*-monitor 2>/dev/null | wc -l)
    if [ "$count" = "3" ]; then
        echo "[PASS] eBPF programs present"
        tests_passed=$((tests_passed + 1))
    fi
fi

echo ""
echo "Passed: $tests_passed / $tests_total"
