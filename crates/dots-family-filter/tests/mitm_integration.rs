use openssl::asn1::Asn1Time;
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::rsa::Rsa;
use openssl::x509::X509Builder;
use std::fs;
use std::net::SocketAddr;
use std::pin::Pin;

use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};

use dots_family_filter::certificate_manager::get_or_generate_acceptor_cached;

#[tokio::test]
async fn integration_mitm_accepts_tls_with_generated_cert() {
    // Create a temporary CA cert and key
    let tmp = std::env::temp_dir();

    let ca_rsa = Rsa::generate(2048).unwrap();
    let ca_pkey = PKey::from_rsa(ca_rsa).unwrap();

    let mut name_builder = openssl::x509::X509NameBuilder::new().unwrap();
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

    let ca_cert_path = tmp.join("test_ca_integration.pem");
    let ca_key_path = tmp.join("test_ca_integration_key.pem");
    fs::write(&ca_cert_path, &ca_cert_pem).unwrap();
    fs::write(&ca_key_path, &ca_key_pem).unwrap();

    // Generate acceptor for host
    let host = "example.local";
    let acceptor = get_or_generate_acceptor_cached(
        host,
        ca_cert_path.to_str().unwrap(),
        ca_key_path.to_str().unwrap(),
    )
    .expect("generate acceptor");

    // Start a TLS server using the acceptor
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr: SocketAddr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        // Build Ssl from acceptor
        let ctx = acceptor.context();
        let mut ssl = openssl::ssl::Ssl::new(ctx).unwrap();
        ssl.set_accept_state();
        let mut tls = tokio_openssl::SslStream::new(ssl, stream).unwrap();
        // Pin for async methods
        let mut tls = Pin::new(&mut tls);
        tls.as_mut().accept().await.unwrap();
        // Read data and echo
        let mut buf = [0u8; 16];
        let n = tls.read(&mut buf).await.unwrap_or(0);
        if n > 0 {
            tls.write_all(&buf[..n]).await.unwrap();
        }
    });

    // Client connects and performs TLS handshake to the acceptor
    let client = tokio::spawn(async move {
        let stream = TcpStream::connect(addr).await.unwrap();
        let mut connector_builder =
            openssl::ssl::SslConnector::builder(openssl::ssl::SslMethod::tls()).unwrap();
        let connector = connector_builder.build();
        let ctx = connector.context();
        let mut ssl = openssl::ssl::Ssl::new(ctx).unwrap();
        ssl.set_connect_state();
        ssl.set_hostname(host).ok();
        let mut tls = tokio_openssl::SslStream::new(ssl, stream).unwrap();
        // Pin for async handshake
        let mut tls = Pin::new(&mut tls);
        tls.as_mut().connect().await.unwrap();
        // Send a short message and expect echo
        tls.write_all(b"ping").await.unwrap();
        let mut buf = [0u8; 8];
        let n = tls.read(&mut buf).await.unwrap();
        assert_eq!(&buf[..n], b"ping");
    });

    let _ = tokio::join!(server, client);

    let _ = fs::remove_file(&ca_cert_path);
    let _ = fs::remove_file(&ca_key_path);
}
