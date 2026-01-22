# DOTS Family Mode - HTTPS Filtering Implementation

This document describes the HTTPS/SSL filtering implementation for DOTS Family Mode web content filtering.

## Overview

HTTPS filtering requires SSL/TLS certificate interception to inspect encrypted traffic. This implementation provides:

1. **Custom CA Certificate Generation** - Creates a private Certificate Authority
2. **System-wide Certificate Installation** - Installs CA to system trust stores
3. **Browser Configuration** - Configures Firefox to trust the CA
4. **CONNECT Tunnel Handling** - Proper HTTPS proxy tunneling support
5. **Certificate Generation** - Per-site certificate generation for interception

## Architecture

### Components

```
User Browser
     |
     v
+------------------------+
|  dots-family-filter    |  <-- Web proxy with CONNECT support
|  (127.0.0.1:8080)      |
+------------------------+
     |
     |  CONNECT tunnel for HTTPS
     v
+------------------------+
|  SSL Interception      |  <-- Generates forged certificates
|  - CA Certificate      |
|  - Site Certificates   |
+------------------------+
     |
     v
Target HTTPS Server
```

### File Structure

```
nixos-modules/dots-family/
├── default.nix              # Main module with SSL imports
├── ssl-intercept.nix        # SSL CA certificate module (NEW)
└── ...other modules...

nix/
└── vm-simple.nix            # VM config with SSL enabled (UPDATED)

crates/dots-family-filter/
├── src/
│   ├── main.rs              # Entry point
│   ├── proxy.rs             # Proxy with CONNECT handling (UPDATED)
│   └── ...
└── Cargo.toml               # Dependencies

scripts/
└── test-https-filtering.sh  # Comprehensive test script (NEW)
```

## SSL CA Certificate Module

### Features

The `ssl-intercept.nix` module provides:

- **Automatic CA Generation**: Creates 4096-bit RSA CA certificate
- **Certificate Storage**: `/var/lib/dots-family/ssl/`
  - `ca.crt` - CA certificate
  - `ca.key` - CA private key
  - `ca.pem` - PEM format
  - `ca.p12` - PKCS12 bundle
- **System Installation**: Installs to `/etc/ssl/certs/`
- **Helper Scripts**:
  - `generate-dots-family-ca` - Generate CA certificate
  - `generate-dots-family-site-cert` - Generate site certificates
  - `install-dots-family-ca` - Install to system trust stores

### Configuration Options

```nix
services.dots-family.sslIntercept = {
  enable = true;
  certPath = "/var/lib/dots-family/ssl";
  caCertPath = "/var/lib/dots-family/ssl/ca.crt";
  caKeyPath = "/var/lib/dots-family/ssl/ca.key";
  countryCode = "US";
  state = "California";
  locality = "San Francisco";
  organization = "DOTS Family Mode";
  pkcs12Password = "dots-family";
};
```

### Systemd Services

1. **dots-family-ssl-ca.service**: Generates CA certificate on boot
2. **dots-family-ssl-ca-install.service**: Installs CA to system trust store

## Browser Configuration

### Firefox

The module configures Firefox via policies to automatically install the CA certificate:

```nix
programs.firefox = {
  enable = true;
  policies = {
    Certificates = {
      Install = [
        "/var/lib/dots-family/ssl/ca.crt"
      ];
    };
  };
};
```

### Chrome/Chromium

For Chrome, the CA is installed system-wide via the trust store. Users can also import manually:

1. Open `chrome://settings/certificates`
2. Click "Authorities" tab
3. Click "Import" and select `/var/lib/dots-family/ssl/ca.crt`

## Proxy CONNECT Handling

The proxy (`proxy.rs`) properly handles HTTPS traffic:

1. **CONNECT Request**: Browser sends `CONNECT host:port HTTP/1.1`
2. **Filter Evaluation**: Check if domain is allowed
3. **Tunnel Establishment**: 
   - Allow: Connect to target server, return 200 to browser
   - Block: Return 403 Forbidden with block page
4. **Bidirectional Tunnel**: Forward traffic between client and server

### Supported Operations

- HTTP requests through proxy
- HTTPS CONNECT tunneling
- Safe search URL rewriting
- Block page generation
- Content filtering decisions

## Certificate Generation

### Generate CA Certificate

```bash
# Using the helper script
generate-dots-family-ca

# Or manually
openssl genrsa -out /var/lib/dots-family/ssl/ca.key 4096
openssl req -new -x509 -days 3650 -key /var/lib/dots-family/ssl/ca.key \
  -out /var/lib/dots-family/ssl/ca.crt \
  -subj "/C=US/ST=California/L=San Francisco/O=DOTS Family Mode/OU=Parental Controls/CN=DOTS Family Mode CA"
```

### Generate Site Certificate

For SSL interception, generate certificates for each site:

```bash
generate-dots-family-site-cert site.key site.csr example.com

# Or manually
openssl genrsa -out site.key 2048
openssl req -new -key site.key -out site.csr -subj "/CN=example.com"
openssl x509 -req -in site.csr -CA ca.crt -CAkey ca.key -CAcreateserial \
  -out site.crt -days 1
```

### Import CA to Browser

```bash
# Firefox
# 1. Open Preferences > Privacy & Security > Certificates > View Certificates
# 2. Click Import, select /var/lib/dots-family/ssl/ca.crt
# 3. Trust this CA for identifying websites

# Chrome
# 1. Open chrome://settings/certificates
# 2. Click Authorities tab > Import
# 3. Select /var/lib/dots-family/ssl/ca.crt
```

## Testing

### Run HTTPS Filtering Tests

```bash
# Run the comprehensive test suite
./scripts/test-https-filtering.sh --evidence /tmp/https-test

# Test with custom proxy
PROXY_HOST=127.0.0.1 PROXY_PORT=8080 ./scripts/test-https-filtering.sh
```

### Manual Testing

```bash
# Start the proxy
dots-family-filter --bind-address 127.0.0.1 --port 8080 &

# Test HTTP
curl -x http://127.0.0.1:8080 http://example.com

# Test HTTPS (requires CONNECT)
curl -x http://127.0.0.1:8080 https://example.com

# Test with browser
chromium-browser --proxy-server=http://127.0.0.1:8080 https://example.com
```

### Test Checklist

- [ ] CA certificate generated at `/var/lib/dots-family/ssl/ca.crt`
- [ ] CA certificate installed to `/etc/ssl/certs/dots-family-ca.crt`
- [ ] Firefox policies configured
- [ ] Proxy accepts CONNECT requests
- [ ] HTTP requests forwarded correctly
- [ ] HTTPS CONNECT tunneling works
- [ ] Block pages displayed for blocked domains
- [ ] Browser trusts CA and accepts forged certificates

## VM Testing

### Build and Run VM

```bash
# Build VM with HTTPS filtering
nix build .#nixosConfigurations.dots-family-test-vm.config.system.build.vm
./result/bin/run-dots-family-test-vm

# Inside VM, run tests
ssh -p 10022 parent@localhost
# Password: parent123

# Run HTTPS tests
./scripts/test-https-filtering.sh --evidence /var/log/dots-family-https-test
```

### VM Configuration

The VM (`nix/vm-simple.nix`) is pre-configured with:

```nix
services.dots-family.sslIntercept = {
  enable = true;
  countryCode = "US";
  state = "California";
  locality = "San Francisco";
  organization = "DOTS Family Mode";
};

programs.firefox = {
  enable = true;
  policies = {
    Certificates.Install = [ "/var/lib/dots-family/ssl/ca.crt" ];
  };
};

environment.systemPackages = [
  openssl
  # ... other packages
];
```

## Security Considerations

### CA Private Key Protection

The CA private key (`ca.key`) is protected with:
- 4096-bit RSA encryption
- File permissions 600 (owner read/write only)
- Stored in `/var/lib/dots-family/ssl/` (not in world-readable locations)

### Certificate Validity

- CA certificate: 10 years (3650 days)
- Site certificates: 1 day (automatic rotation)
- Short-lived site certificates minimize risk if compromised

### Browser Trust

Only browsers configured to trust the CA will accept intercepted certificates. Users must explicitly:
1. Install the CA certificate
2. Trust it for website identification

## Troubleshooting

### Certificate Not Trusted

```bash
# Check if CA exists
ls -la /var/lib/dots-family/ssl/ca.crt

# Check system installation
ls -la /etc/ssl/certs/dots-family-ca.crt

# Reinstall CA
install-dots-family-ca
```

### HTTPS Not Working

```bash
# Check proxy is running
pgrep -f dots-family-filter

# Test CONNECT method
curl -v -x http://127.0.0.1:8080 https://example.com

# Check proxy logs
journalctl -u dots-family-daemon.service
```

### Firefox Certificate Error

```bash
# Check Firefox policy
cat /etc/firefox/policies/policies.json

# Manually import certificate
firefox preferences > Certificates > View Certificates > Import
```

## Future Enhancements

1. **Dynamic Certificate Generation**: Generate site certificates on-the-fly
2. **OCSP Support**: Online Certificate Status Protocol
3. **Certificate Pinning**: For critical domains
4. **Multiple CA Support**: Different CAs for different profiles
5. **CRL Management**: Certificate Revocation Lists

## References

- [RFC 7230](https://tools.ietf.org/html/rfc7230) - HTTP/1.1 Message Syntax and Routing
- [RFC 7231](https://tools.ietf.org/html/rfc7231) - HTTP/1.1 Semantics and Content
- [RFC 7540](https://tools.ietf.org/html/rfc7540) - HTTP/2
- [Mozilla CA Program](https://wiki.mozilla.org/CA) - Certificate Authority requirements
