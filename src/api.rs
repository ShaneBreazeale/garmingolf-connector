use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

use crate::config::AppConfig;
use crate::core::{
    AppState, AppStatus, BallMetrics, ClubMetrics, ConnectionStatus, EndpointStatus, GarminStatus,
    ShotEvent,
};

#[derive(Clone)]
struct ApiState {
    app: AppState,
    config: AppConfig,
    test_shot_counter: Arc<AtomicU64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ActionAccepted {
    pub accepted: bool,
}

#[derive(OpenApi)]
#[openapi(
    paths(health, status, get_config, patch_config, test_shot),
    components(schemas(
        HealthResponse,
        ActionAccepted,
        AppConfig,
        AppStatus,
        GarminStatus,
        EndpointStatus,
        ConnectionStatus,
        ShotEvent,
        BallMetrics,
        ClubMetrics
    ))
)]
pub struct ApiDoc;

pub fn router(config: AppConfig, app: AppState) -> Router {
    let state = ApiState {
        app,
        config,
        test_shot_counter: Arc::new(AtomicU64::new(0)),
    };

    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/config", get(get_config).patch(patch_config))
        .route("/shots/test", post(test_shot))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        .with_state(state)
}

pub async fn serve(config: AppConfig, app: AppState) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(config.api_addr())
        .await
        .map_err(|err| format!("failed to bind API server: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("failed to read API server address: {err}"))?;

    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router(config, app)).await {
            tracing::error!(%err, "API server stopped");
        }
    });

    Ok(addr)
}

#[utoipa::path(
    get,
    path = "/health",
    responses((status = 200, description = "API server is healthy", body = HealthResponse))
)]
async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { ok: true })
}

#[utoipa::path(
    get,
    path = "/status",
    responses((status = 200, description = "Current connector status", body = AppStatus))
)]
async fn status(State(state): State<ApiState>) -> Json<AppStatus> {
    Json(state.app.status().await)
}

#[utoipa::path(
    get,
    path = "/config",
    responses((status = 200, description = "Current connector configuration", body = AppConfig))
)]
async fn get_config(State(state): State<ApiState>) -> Json<AppConfig> {
    Json(state.config)
}

#[utoipa::path(
    patch,
    path = "/config",
    responses((status = 202, description = "Configuration update accepted", body = ActionAccepted))
)]
async fn patch_config() -> (StatusCode, Json<ActionAccepted>) {
    (
        StatusCode::ACCEPTED,
        Json(ActionAccepted { accepted: true }),
    )
}

#[utoipa::path(
    post,
    path = "/shots/test",
    responses((status = 202, description = "Test shot accepted", body = ActionAccepted))
)]
async fn test_shot(State(state): State<ApiState>) -> (StatusCode, Json<ActionAccepted>) {
    let shot_number = state.test_shot_counter.fetch_add(1, Ordering::Relaxed) + 1;
    state
        .app
        .publish_shot(ShotEvent::test_shot(shot_number))
        .await;

    (
        StatusCode::ACCEPTED,
        Json(ActionAccepted { accepted: true }),
    )
}
