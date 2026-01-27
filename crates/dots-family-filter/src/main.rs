use anyhow::Result;
use clap::Parser;
use tracing::info;

mod config;
mod filter_engine;
mod proxy;
mod rules;
mod shuttle;

#[derive(Parser, Debug)]
#[command(name = "dots-family-filter")]
#[command(about = "DOTS Family Mode content filtering and web proxy")]
struct Args {
    #[arg(short, long, default_value = "8080")]
    port: u16,

    #[arg(short, long, default_value = "127.0.0.1")]
    bind_address: String,

    #[arg(short, long)]
    config_path: Option<String>,

    #[arg(long)]
    daemon_mode: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .init();

    let args = Args::parse();

    info!("Starting DOTS Family Filter");
    info!("Proxy server will bind to {}:{}", args.bind_address, args.port);

    let config = config::FilterConfig::load(args.config_path)?;
    let filter_engine = filter_engine::FilterEngine::new(config).await?;

    let proxy = proxy::WebProxy::new(filter_engine);
    proxy.start(&args.bind_address, args.port).await?;

    Ok(())
}
