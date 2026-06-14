use garmingolf_connector::config::AppConfig;

#[tokio::main]
async fn main() -> Result<(), String> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "garmingolf_connector=info,tower_http=info".into()),
        )
        .try_init();

    let config = AppConfig::from_env()?;
    println!("garmingolf-connector configured: api={}", config.api_addr());
    Ok(())
}
