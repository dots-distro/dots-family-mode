use anyhow::{Context, Result};
use lru::LruCache;
use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::ssl::{SslAcceptor, SslMethod};
use openssl::x509::{X509Builder, X509NameBuilder, X509};
use parking_lot::Mutex;
use std::num::NonZeroUsize;
use std::sync::{Arc, OnceLock};

/// Generate an SslAcceptor for a given host using the provided CA cert and key paths.
/// This creates a short-lived certificate for `host` signed by the CA and returns
/// an SslAcceptor that can be used to accept TLS from clients.
pub fn generate_acceptor_for_host(
    host: &str,
    ca_cert_path: &str,
    ca_key_path: &str,
) -> Result<SslAcceptor> {
    // Generate fresh cert/key and build acceptor
    let (cert_pem, key_pem, ca_cert_pem) =
        generate_cert_and_key_pem(host, ca_cert_path, ca_key_path)?;
    build_acceptor_from_pems(&cert_pem, &key_pem, &ca_cert_pem)
}

/// Generate certificate PEM and private key PEM for a host, signed by the CA
fn generate_cert_and_key_pem(
    host: &str,
    ca_cert_path: &str,
    ca_key_path: &str,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>)> {
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
    name_builder.append_entry_by_text("CN", host).context("setting common name")?;
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

    // Add subjectAltName using modern builder API
    {
        let mut san = openssl::x509::extension::SubjectAlternativeName::new();
        san.dns(host);
        if let Ok(ext) = san.build(&builder.x509v3_context(Some(&ca_cert), None)) {
            builder.append_extension(ext).ok();
        }
    }

    // Sign with CA key
    builder.sign(&ca_key, MessageDigest::sha256()).context("signing certificate with CA key")?;

    let cert = builder.build();

    // Convert to PEMs
    let cert_pem = cert.to_pem().context("serializing cert to PEM")?;
    let key_pem = pkey.private_key_to_pem_pkcs8().context("serializing key to PEM")?;

    Ok((cert_pem, key_pem, ca_cert_pem))
}

/// Build an SslAcceptor from provided PEM blobs (leaf cert, key, and CA cert)
fn build_acceptor_from_pems(
    cert_pem: &[u8],
    key_pem: &[u8],
    ca_cert_pem: &[u8],
) -> Result<SslAcceptor> {
    let cert = X509::from_pem(cert_pem).context("parsing site certificate PEM")?;
    let pkey = PKey::private_key_from_pem(key_pem).context("parsing site private key PEM")?;
    let ca_cert = X509::from_pem(ca_cert_pem).context("parsing CA cert PEM")?;

    let mut acceptor_builder = SslAcceptor::mozilla_intermediate(SslMethod::tls())
        .context("creating SslAcceptorBuilder")?;
    acceptor_builder.set_private_key(&pkey).context("setting private key")?;
    acceptor_builder.set_certificate(&cert).context("setting certificate")?;
    acceptor_builder.add_extra_chain_cert(ca_cert).ok();
    acceptor_builder.check_private_key().context("checking private key")?;
    Ok(acceptor_builder.build())
}

// Cache stores the generated cert and key PEMs for hosts. Building SslAcceptor from PEM is cheap
// and allows us to avoid storing non-cloneable OpenSSL objects directly.
pub type AcceptorCache = Arc<Mutex<LruCache<String, (Vec<u8>, Vec<u8>)>>>;

pub fn new_acceptor_cache(capacity: usize) -> AcceptorCache {
    Arc::new(Mutex::new(LruCache::new(NonZeroUsize::new(capacity).unwrap())))
}

static DEFAULT_ACCEPTOR_CACHE: OnceLock<AcceptorCache> = OnceLock::new();

fn default_acceptor_cache() -> AcceptorCache {
    DEFAULT_ACCEPTOR_CACHE.get_or_init(|| new_acceptor_cache(1024)).clone()
}

/// Get an SslAcceptor from the cache or generate and cache the PEMs, then build the acceptor.
pub fn get_or_generate_acceptor(
    cache: &AcceptorCache,
    host: &str,
    ca_cert_path: &str,
    ca_key_path: &str,
) -> Result<SslAcceptor> {
    // If present in cache, build acceptor from cached PEMs
    if let Some((cert_pem, key_pem)) = cache.lock().get(host).cloned() {
        return build_acceptor_from_pems(
            &cert_pem,
            &key_pem,
            &std::fs::read(ca_cert_path).context("reading CA cert file")?,
        );
    }

    // Not in cache: generate new cert/key PEMs and insert
    let (cert_pem, key_pem, ca_cert_pem) =
        generate_cert_and_key_pem(host, ca_cert_path, ca_key_path)?;
    cache.lock().put(host.to_string(), (cert_pem.clone(), key_pem.clone()));
    build_acceptor_from_pems(&cert_pem, &key_pem, &ca_cert_pem)
}

/// Convenience wrapper using the default global cache
pub fn get_or_generate_acceptor_cached(
    host: &str,
    ca_cert_path: &str,
    ca_key_path: &str,
) -> Result<SslAcceptor> {
    let cache = default_acceptor_cache();
    get_or_generate_acceptor(&cache, host, ca_cert_path, ca_key_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openssl::asn1::Asn1Time;
    use openssl::hash::MessageDigest;
    use openssl::pkey::PKey;
    use openssl::rsa::Rsa;
    use openssl::x509::{X509Builder, X509NameBuilder};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn test_generate_acceptor_for_host_with_generated_ca() {
        let tmp = std::env::temp_dir();

        // Create a self-signed CA cert and key
        let ca_rsa = Rsa::generate(2048).unwrap();
        let ca_pkey = PKey::from_rsa(ca_rsa).unwrap();

        let mut name_builder = X509NameBuilder::new().unwrap();
        name_builder.append_entry_by_text("CN", "Test CA").unwrap();
        let name = name_builder.build();

        let mut builder = X509Builder::new().unwrap();
        builder.set_version(2).unwrap();
        builder.set_subject_name(&name).unwrap();
        builder.set_issuer_name(&name).unwrap();
        builder.set_pubkey(&ca_pkey).unwrap();
        let not_before = Asn1Time::days_from_now(0).unwrap();
        let not_after = Asn1Time::days_from_now(365).unwrap();
        builder.set_not_before(&not_before).unwrap();
        builder.set_not_after(&not_after).unwrap();
        builder.sign(&ca_pkey, MessageDigest::sha256()).unwrap();
        let ca_cert = builder.build();

        let ca_cert_pem = ca_cert.to_pem().unwrap();
        let ca_key_pem = ca_pkey.private_key_to_pem_pkcs8().unwrap();

        let id = Uuid::new_v4().to_string();
        let ca_cert_path = tmp.join(format!("ca-{}.pem", id));
        let ca_key_path = tmp.join(format!("cakey-{}.pem", id));

        fs::write(&ca_cert_path, &ca_cert_pem).unwrap();
        fs::write(&ca_key_path, &ca_key_pem).unwrap();

        let res = generate_acceptor_for_host(
            "example.local",
            ca_cert_path.to_str().unwrap(),
            ca_key_path.to_str().unwrap(),
        );
        assert!(res.is_ok());

        let _ = fs::remove_file(ca_cert_path);
        let _ = fs::remove_file(ca_key_path);
    }
}
