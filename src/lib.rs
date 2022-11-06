use tracing_subscriber::{fmt::format::FmtSpan, prelude::*};

#[tarpc::service]
pub trait World {
    async fn hello(name: String) -> String;
    async fn healthcheck() -> &'static str;
}

pub fn init_tracing(service_name: &str) -> anyhow::Result<()> {
    tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG")
            .unwrap_or_else(|_| "splinter=debug,tower_http=debug".into()),
    ))
    .with(tracing_subscriber::fmt::layer())
    .init();
    Ok(())
}