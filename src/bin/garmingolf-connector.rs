use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;

#[tokio::main]
async fn main() -> Result<(), String> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "garmingolf_connector=info,tower_http=info".into()),
        )
        .try_init();

    let config = AppConfig::from_env()?;
    let state = AppState::new(&config);

    let garmin_addr =
        garmingolf_connector::garmin::runtime::spawn_listener(config.clone(), state.clone())
            .await?;
    let api_addr = garmingolf_connector::api::serve(config.clone(), state.clone()).await?;

    if config.gspro_enabled {
        garmingolf_connector::gspro::runtime::spawn_forwarder(config.clone(), state.clone()).await;
    }
    if config.nova_ws_enabled {
        let nova_addr =
            garmingolf_connector::nova::spawn_server(config.clone(), state.clone()).await?;
        println!("Nova WebSocket listening on ws://{nova_addr}/ws");
    }

    println!("Garmin listener running on {garmin_addr}");
    println!("OpenAPI server running on http://{api_addr}");
    println!("Swagger UI available at http://{api_addr}/swagger-ui");

    tokio::signal::ctrl_c()
        .await
        .map_err(|err| format!("failed waiting for ctrl-c: {err}"))?;
    Ok(())
}
