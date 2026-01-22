# DOTS Family Mode - Web Filtering Test Suite

Comprehensive Playwright-based tests for web content filtering functionality in DOTS Family Mode.

## Overview

This test suite validates the web content filtering capabilities of DOTS Family Mode using Playwright browser automation. It tests:

- **Blocked domain detection** - Verify blocked domains return 403 Forbidden
- **Allowed domain access** - Verify allowed domains work through the proxy
- **HTTPS filtering** - Test CONNECT method for HTTPS traffic
- **Safe search enforcement** - Test safe search URL rewriting
- **Proxy configuration** - Verify proper proxy routing

## Files

- `web-filtering-test.js` - Main Node.js test suite using Playwright
- `run-web-filtering-test.sh` - Shell wrapper script for easy execution
- `package.json` - Node.js package configuration
- `README.md` - This file

## Requirements

- Node.js 18+
- Playwright library
- dots-family-filter proxy running on configured port (default: 127.0.0.1:8080)

### NixOS VM Requirements (Recommended)

When running in the DOTS Family Mode VM, the following packages are automatically available:

```nix
# From nix/vm-simple.nix
environment.systemPackages = with pkgs; [
    nodejs_20
    playwright
    playwright-driver.browsers  # Pre-packaged browser binaries
];
```

The `playwright-driver.browsers` package provides:
- Chromium (chromium-1181, chromium_headless_shell-1181)
- Firefox (firefox-1489)
- WebKit (webkit-2191)

Located at: `/run/current-system/sw/share/playwright-driver/browsers`

## Installation

### In the VM Test Environment

The VM is pre-configured with Node.js and Playwright. Tests can be run directly:

```bash
cd /home/shift/code/endpoint-agent/dots-detection/dots-familt-mode/scripts/web-filtering-test
./run-web-filtering-test.sh
```

### On Development System

Install dependencies:

```bash
cd scripts/web-filtering-test
npm install
npx playwright install chromium
```

## Usage

### Shell Wrapper (Recommended)

```bash
# Run with defaults (127.0.0.1:8080, test-evidence/web-filtering)
./run-web-filtering-test.sh

# Custom proxy settings
./run-web-filtering-test.sh --proxy 192.168.1.100 --port 8080

# Custom evidence directory
./run-web-filtering-test.sh --evidence /tmp/my-tests
```

### Direct Node.js Execution

```bash
# Run with defaults
node web-filtering-test.js

# Custom settings
node web-filtering-test.js --proxy-host=127.0.0.1 --proxy-port=8080 --evidence-dir=my-evidence
```

### npm Scripts

```bash
# Run tests with defaults
npm test

# Test with specific proxy
npm run test:proxy

# Custom evidence directory
npm run test:evidence -- --evidence-dir=/tmp/tests
```

## Test Evidence

All test evidence is saved to the evidence directory (default: `test-evidence/web-filtering/`):

### Directory Structure

```
test-evidence/web-filtering/
├── test_report.md          # Main test report with all results
├── evidence.json           # Machine-readable evidence data
├── screenshots/            # Screenshots of blocked/allowed pages
│   ├── blocked_*.png
│   └── allowed_*.png
├── html/                   # HTML responses from filter
│   ├── blocked_*.html
│   └── allowed_*.html
└── network/                # Network request logs
    └── *.json
```

### Evidence Report Format

The `test_report.md` includes:

- Test configuration (proxy settings, timeouts)
- Summary table (total, passed, failed, skipped tests)
- Detailed test results with status, duration, and errors
- Screenshots and HTML evidence paths
- Filtering levels and domains tested

## Test Configuration

### Default Configuration

```javascript
{
    proxy: {
        host: '127.0.0.1',
        port: 8080
    },
    testTimeout: 30000,  // 30 seconds per test
    evidenceDir: 'test-evidence/web-filtering',
    filterLevels: ['strict', 'moderate', 'minimal', 'disabled']
}
```

### Command Line Options

| Option | Default | Description |
|--------|---------|-------------|
| `--proxy-host` | 127.0.0.1 | Filter proxy host |
| `--proxy-port` | 8080 | Filter proxy port |
| `--evidence-dir` | test-evidence/web-filtering | Evidence output directory |

## Test Cases

### 1. Proxy Connectivity
Verifies the filter proxy is accessible before running tests.

### 2. Blocked Domain Tests
Tests that configured blocked domains return 403 Forbidden with the block page.

**Test Domains:**
- Blocked test URLs (example.com variants)

**Expected Behavior:**
- HTTP 403 status code
- Block page with "Content Blocked - DOTS Family Mode"

### 3. Allowed Domain Tests
Tests that allowed domains are accessible through the proxy.

**Test Domains:**
- example.com (reference site)
- wikipedia.org (educational)
- google.com (search engine)

**Expected Behavior:**
- HTTP 200 status code
- Normal page content (no block page)

### 4. HTTPS Filtering Tests
Tests HTTPS traffic filtering via CONNECT method.

**Test:**
- https://example.com through proxy

**Expected Behavior:**
- CONNECT request processed
- Appropriate allow/block response

### 5. Safe Search Enforcement Tests
Tests that safe search parameters are enforced.

**Test Domains:**
- Google (safe search)
- DuckDuckGo (safe search)
- Bing (safe search)

### 6. Proxy Configuration Tests
Verifies proxy routing works correctly.

**Test:**
- Compare direct connection vs proxy connection
- Verify proxy properly intercepts requests

## Integration with VM Testing

### Automated VM Tests

Run web filtering tests as part of the full VM test suite:

```bash
./run-all-vm-tests.sh --web-filtering-only
```

Or with all tests:

```bash
./run-all-vm-tests.sh
```

### Nix Build

Build the test package:

```bash
nix build .#dots-family-web-filtering-test
```

### VM Configuration

The VM configuration includes:

- Node.js 20
- Playwright
- Test scripts

See `nix/vm-simple.nix` for the full configuration.

## Troubleshooting

### Playwright Browsers Not Found

**Error:** `Executable doesn't exist at .../chromium_headless_shell-1200/...`

**Solution:** Ensure `playwright-driver.browsers` is in your system packages:

```nix
# In your VM configuration (nix/vm-simple.nix)
environment.systemPackages = with pkgs; [
    playwright
    playwright-driver.browsers  # Required for browser binaries
];
```

Then rebuild and restart the VM:
```bash
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
./result/bin/run-dots-family-test-vm
```

### In VM Environment

The test runner automatically detects Nix-packaged browsers at:
```
/run/current-system/sw/share/playwright-driver/browsers
```

### Proxy Not Available

If the filter proxy is not running, tests will be skipped:

```
[INFO] Proxy not available - running integration tests only
[SKIP] Block Test: Blocked Test URL
```

**Solution:** Start the filter proxy before running tests:

```bash
dots-family-filter --bind-address 127.0.0.1 --port 8080 &
./run-web-filtering-test.sh --start-filter
```

### Node.js Version

Ensure Node.js 18+ is installed:

```bash
node --version  # Should show v18.x.x or higher
```

## Extending Tests

### Adding New Test Domains

Edit `web-filtering-test.js` and add to the `CONFIG.domains` object:

```javascript
domains: {
    allowed: [
        // Add new allowed domain
        { url: 'https://newsite.com', name: 'New Site', category: 'reference' }
    ],
    blocked: [
        // Add new blocked domain
        { url: 'http://badsite.com', name: 'Bad Site', category: 'adult' }
    ]
}
```

### Adding New Test Types

Add new test methods to the `WebFilterTestRunner` class:

```javascript
async testNewFeature() {
    const testName = 'New Feature Test';
    const description = 'Test new filtering feature';
    // Implementation...
}
```

Then call from `runAllTests()`:

```javascript
await this.testNewFeature();
```

## Architecture

```
web-filtering-test.js
├── EvidenceCollector    - Collects and saves test evidence
├── WebFilterTestRunner  - Main test orchestration
│   ├── startProxy()     - Start filter proxy
│   ├── setupBrowser()   - Initialize Playwright
│   ├── testBlockedDomain()
│   ├── testAllowedDomain()
│   ├── testHTTPSFiltering()
│   ├── testSafeSearch()
│   └── testProxyAuth()
└── main()               - Entry point
```

## License

MIT - See project root for full license information.
