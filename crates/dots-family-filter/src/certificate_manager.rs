use anyhow::{Context, Result};
use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::nid::Nid;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
use openssl::x509::{X509Builder, X509Extension, X509NameBuilder, X509};

/// Generate an SslAcceptor for a given host using the provided CA cert and key paths.
/// This creates a short-lived certificate for `host` signed by the CA and returns
/// an SslAcceptor that can be used to accept TLS from clients.
pub fn generate_acceptor_for_host(
    host: &str,
    ca_cert_path: &str,
    ca_key_path: &str,
) -> Result<SslAcceptor> {
    // Read CA cert and key
    let ca_cert_pem = std::fs::read(ca_cert_path).context("reading CA cert file")?;
    let ca_key_pem = std::fs::read(ca_key_path).context("reading CA key file")?;

    let ca_cert = X509::from_pem(&ca_cert_pem).context("parsing CA certificate PEM")?;
    let ca_key = PKey::private_key_from_pem(&ca_key_pem).context("parsing CA private key PEM")?;

    // Generate a new RSA key for the site cert
    let rsa = Rsa::generate(2048).context("generating RSA key")?;
    let pkey = PKey::from_rsa(rsa).context("creating PKey from RSA")?;

    // Build subject name
    let mut name_builder = X509NameBuilder::new().context("creating X509NameBuilder")?;
    name_builder.append_entry_by_nid(Nid::COMMONNAME, host).context("setting common name")?;
    let name = name_builder.build();

    // Build X509 certificate
    let mut builder = X509Builder::new().context("creating X509Builder")?;
    builder.set_version(2).context("setting X509 version")?;
    builder.set_subject_name(&name).context("setting subject name")?;
    builder.set_issuer_name(ca_cert.subject_name()).context("setting issuer name")?;
    builder.set_pubkey(&pkey).context("setting public key")?;

    // Validity: now -> now + 365 days
    let not_before = Asn1Time::days_from_now(0).context("setting not_before")?;
    let not_after = Asn1Time::days_from_now(365).context("setting not_after")?;
    builder.set_not_before(&not_before).context("applying not_before")?;
    builder.set_not_after(&not_after).context("applying not_after")?;

    // Optionally add subjectAltName
    if let Ok(ext) =
        X509Extension::new_nid(None, None, Nid::SUBJECT_ALT_NAME, &format!("DNS:{}", host))
    {
        builder.append_extension(ext).ok();
    }

    // Sign with CA key
    builder.sign(&ca_key, MessageDigest::sha256()).context("signing certificate with CA key")?;

    let cert = builder.build();

    // Build SslAcceptor
    let mut acceptor_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .context("creating SslAcceptorBuilder")?;
    acceptor_builder.set_private_key(&pkey).context("setting private key")?;
    acceptor_builder.set_certificate(&cert).context("setting certificate")?;
    // Also set CA as chain cert so clients see issuer
    acceptor_builder.add_extra_chain_cert(ca_cert).ok();
    acceptor_builder.check_private_key().context("checking private key")?;

    Ok(acceptor_builder.build())
}
