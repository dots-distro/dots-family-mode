# DOTS Family Mode SSL CA Certificate Module
#
# This module generates a custom CA certificate for SSL/TLS interception
# in the web filtering proxy. The CA certificate is installed system-wide
# so that browsers can trust the proxy's forged certificates.
#
# Usage:
#   services.dots-family.sslIntercept.enable = true;
#

{ config, lib, pkgs, ... }:

let
  cfg = config.services.dots-family.sslIntercept;

  # Generate SSL certificate and key using openssl
  certGenerationScript = pkgs.writeScriptBin "generate-dots-family-ca" ''
    #!/bin/bash
    set -euo pipefail

    CERT_DIR="${cfg.certPath}"
    CA_KEY="$CERT_DIR/ca.key"
    CA_CERT="$CERT_DIR/ca.crt"
    CA_PEM="$CERT_DIR/ca.pem"

    echo "Generating DOTS Family Mode CA certificate..."

    # Create certificate directory
    mkdir -p "$CERT_DIR"
    chmod 700 "$CERT_DIR"

    # Generate CA private key
    ${pkgs.openssl}/bin/openssl genrsa -out "$CA_KEY" 4096
    chmod 600 "$CA_KEY"

    # Generate CA certificate
    ${pkgs.openssl}/bin/openssl req -new -x509 -days 3650 -key "$CA_KEY" \
      -out "$CA_CERT" \
      -subj "/C=${cfg.countryCode}/ST=${cfg.state}/L=${cfg.locality}/O=DOTS Family Mode/OU=Parental Controls/CN=DOTS Family Mode CA"

    # Create PEM format for some applications
    ${pkgs.openssl}/bin/openssl x509 -in "$CA_CERT" -out "$CA_PEM" -outform PEM

    # Create PKCS12 format for importing into some browsers
    ${pkgs.openssl}/bin/openssl pkcs12 -export \
      -out "$CERT_DIR/ca.p12" \
      -inkey "$CA_KEY" \
      -in "$CA_CERT" \
      -passout pass:${cfg.pkcs12Password}

    echo "CA certificate generated successfully!"
    echo "Certificate: $CA_CERT"
    echo "Private Key: $CA_KEY"
    echo "PKCS12: $CERT_DIR/ca.p12"
    echo ""
    echo "To install in Firefox:"
    echo "  1. Open Firefox Preferences"
    echo "  2. Search for 'Certificates'"
    echo "  3. Click 'View Certificates'"
    echo "  4. Click 'Import' and select $CA_PEM"
    echo ""
    echo "To install in Chrome/Chromium:"
    echo "  1. Open chrome://settings/certificates"
    echo "  2. Click 'Authorities' tab"
    echo "  3. Click 'Import' and select $CA_CERT"
  '';

  # Script to generate per-site certificates for SSL interception
  siteCertGenerationScript = pkgs.writeScriptBin "generate-dots-family-site-cert" ''
    #!/bin/bash
    set -euo pipefail

    SITE_KEY="$1"
    SITE_CERT="$2"
    SITE_CSR="$3"
    COMMON_NAME="$4"
    CA_KEY="${cfg.caKeyPath}"
    CA_CERT="${cfg.caCertPath}"

    if [ $# -lt 4 ]; then
      echo "Usage: $0 <key-out> <cert-out> <csr-out> <common-name>"
      exit 1
    fi

    echo "Generating certificate for: $COMMON_NAME"

    # Generate site private key
    ${pkgs.openssl}/bin/openssl genrsa -out "$SITE_KEY" 2048

    # Generate CSR
    ${pkgs.openssl}/bin/openssl req -new \
      -key "$SITE_KEY" \
      -out "$SITE_CSR" \
      -subj "/C=US/ST=State/L=City/O=DOTS Family Mode/OU=Parental Controls/CN=$COMMON_NAME"

    # Create extensions file for site certificate
    cat > /tmp/site_cert_ext.cnf << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = $COMMON_NAME
DNS.2 = *.$COMMON_NAME
EOF

    # Sign the CSR with the CA
    ${pkgs.openssl}/bin/openssl x509 -req \
      -in "$SITE_CSR" \
      -CA "$CA_CERT" \
      -CAkey "$CA_KEY" \
      -CAcreateserial \
      -out "$SITE_CERT" \
      -days 1 \
      -extfile /tmp/site_cert_ext.cnf

    # Clean up
    rm -f /tmp/site_cert_ext.cnf "$SITE_CSR"

    echo "Site certificate generated: $SITE_CERT"
  '';

  # Install CA certificate to system trust store
  installCaScript = pkgs.writeScriptBin "install-dots-family-ca" ''
    #!/bin/bash
    set -euo pipefail

    CA_CERT="${cfg.caCertPath}"
    CERT_DIR="${cfg.certPath}"

    echo "Installing DOTS Family Mode CA certificate..."

    # Install to system-wide certificate store (NixOS specific)
    if [ -d "/etc/ssl/certs" ]; then
      cp "$CA_CERT" "/etc/ssl/certs/dots-family-ca.crt"
      chmod 644 "/etc/ssl/certs/dots-family-ca.crt"
      ${pkgs.openssl}/bin/openssl rehash /etc/ssl/certs 2>/dev/null || true
      echo "Installed to /etc/ssl/certs/"
    fi

    # Install to Java certificate store (for Java-based applications)
    if [ -d "/etc/pki/java" ]; then
      cp "$CA_CERT" "/etc/pki/java/dots-family-ca.crt"
      chmod 644 "/etc/pki/java/dots-family-ca.crt"
    fi

    # Update GTK certificate store (for applications using GLib)
    if command -v trust 2>/dev/null; then
      trust anchor "$CA_CERT"
      echo "Added to system trust anchor"
    elif command -v update-ca-certificates 2>/dev/null; then
      cp "$CA_CERT" "/usr/local/share/ca-certificates/dots-family-ca.crt"
      update-ca-certificates
      echo "Updated system CA certificates"
    else
      echo "Warning: Could not update system certificate store"
      echo "Please manually install $CA_CERT"
    fi

    echo "CA certificate installation complete!"
  '';

in {
  options.services.dots-family.sslIntercept = {
    enable = lib.mkEnableOption "Enable SSL/TLS interception for HTTPS filtering";

    certPath = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/dots-family/ssl";
      description = "Directory to store SSL certificates and keys";
    };

    caCertPath = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/dots-family/ssl/ca.crt";
      description = "Path to the CA certificate";
    };

    caKeyPath = lib.mkOption {
      type = lib.types.str;
      default = "/var/lib/dots-family/ssl/ca.key";
      description = "Path to the CA private key";
    };

    countryCode = lib.mkOption {
      type = lib.types.str;
      default = "US";
      description = "Country code for CA certificate";
    };

    state = lib.mkOption {
      type = lib.types.str;
      default = "California";
      description = "State for CA certificate";
    };

    locality = lib.mkOption {
      type = lib.types.str;
      default = "San Francisco";
      description = "Locality (city) for CA certificate";
    };

    organization = lib.mkOption {
      type = lib.types.str;
      default = "DOTS Family Mode";
      description = "Organization for CA certificate";
    };

    pkcs12Password = lib.mkOption {
      type = lib.types.str;
      default = "dots-family";
      description = "Password for PKCS12 certificate bundle";
    };
  };

  config = lib.mkIf cfg.enable {
    # Create certificate directory
    systemd.tmpfiles.rules = [
      "d ${cfg.certPath} 700 root root"
    ];

    # Generate CA certificate on activation
    systemd.services.dots-family-ssl-ca = {
      description = "Generate DOTS Family Mode SSL CA Certificate";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = "yes";
        ExecStart = "${certGenerationScript}/bin/generate-dots-family-ca";
      };
    };

    # Install CA certificate to system trust store
    systemd.services.dots-family-ssl-ca-install = {
      description = "Install DOTS Family Mode CA Certificate";
      after = [ "dots-family-ssl-ca.service" ];
      wantedBy = [ "multi-user.target" ];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = "yes";
        ExecStart = "${installCaScript}/bin/install-dots-family-ca";
      };
    };

    # Provide helper scripts for generating site certificates
    environment.systemPackages = [
      certGenerationScript
      siteCertGenerationScript
      installCaScript
    ];

    # Add CA certificate to system-wide trusted certificates
    # Note: We don't include it in security.pki.certificateFiles because that would
    # try to build it during the Nix build phase. Instead, it's installed at runtime
    # by the systemd service.
  };
}
