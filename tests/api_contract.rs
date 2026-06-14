use axum::body::Body;
use axum::http::{Request, StatusCode};
use garmingolf_connector::api::router;
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;
use tower::ServiceExt;

fn test_config() -> AppConfig {
    AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: false,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 8765,
    }
}

fn test_state() -> AppState {
    AppState::new(&test_config())
}

#[tokio::test]
async fn health_returns_ok() {
    let response = router(test_config(), test_state())
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn status_returns_json_status() {
    let response = router(test_config(), test_state())
        .oneshot(
            Request::builder()
                .uri("/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn config_returns_json_config() {
    let response = router(test_config(), test_state())
        .oneshot(
            Request::builder()
                .uri("/config")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn swagger_ui_is_mounted() {
    let response = router(test_config(), test_state())
        .oneshot(
            Request::builder()
                .uri("/api-docs/openapi.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_shot_updates_status() {
    let state = test_state();
    let app = router(test_config(), state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/shots/test")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert_eq!(state.status().await.last_shot.unwrap().shot_number, 1);
}
