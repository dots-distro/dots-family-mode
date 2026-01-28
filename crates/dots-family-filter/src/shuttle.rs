use anyhow::{anyhow, Result};
use hyper::upgrade::Upgraded;
use hyper_util::rt::TokioIo;
use tokio::io::DuplexStream;
use tokio::net::TcpStream;

/// Create a tokio DuplexStream endpoint for use by the MITM TLS stack.
pub async fn create_shuttle() -> DuplexStream {
    let (client_side, _server_side) = tokio::io::duplex(64 * 1024);
    client_side
}

/// Bridge an upgraded hyper connection to a remote TCP stream. This wraps the hyper
/// Upgraded object with `hyper_util::rt::TokioIo` so we can use tokio copy utilities.
pub async fn bridge_upgraded_to_tcp(upgraded: Upgraded, mut tcp: TcpStream) -> Result<()> {
    let mut client = TokioIo::new(upgraded);

    match tokio::io::copy_bidirectional(&mut client, &mut tcp).await {
        Ok((_c2s, _s2c)) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}

/// Bridge any tokio AsyncRead+AsyncWrite object to a TcpStream. Useful when the
/// caller already wrapped an upgraded connection in a Tokio-compatible adapter
/// (for example `hyper_util::rt::TokioIo`) or has an async TLS stream.
pub async fn bridge_io_to_tcp<RW>(mut client: RW, mut tcp: TcpStream) -> Result<()>
where
    RW: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    match tokio::io::copy_bidirectional(&mut client, &mut tcp).await {
        Ok((_c2s, _s2c)) => Ok(()),
        Err(e) => Err(anyhow::anyhow!(e)),
    }
}
