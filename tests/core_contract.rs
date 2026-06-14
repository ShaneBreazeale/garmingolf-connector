use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::{AppState, ClubMetrics, ConnectionStatus, ShotEvent};

#[tokio::test]
async fn app_state_records_last_shot_and_broadcasts_events() {
    let config = AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: true,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: true,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 0,
    };
    let state = AppState::new(&config);
    let mut rx = state.subscribe_shots();

    let shot = ShotEvent::test_shot(7);
    state.publish_shot(shot.clone()).await;

    assert_eq!(
        state.status().await.last_shot.as_ref().unwrap().shot_number,
        7
    );
    assert_eq!(rx.recv().await.expect("shot").shot_number, 7);
}

#[test]
fn test_shot_contains_ball_and_optional_club_metrics() {
    let shot = ShotEvent::test_shot(3);

    assert_eq!(shot.shot_number, 3);
    assert_eq!(shot.device_id, "Garmin R10");
    assert_eq!(shot.units, "Yards");
    assert_eq!(shot.club_type, "7Iron");
    assert_eq!(shot.ball.ball_speed, 98.5);
    assert!(matches!(
        shot.club,
        Some(ClubMetrics {
            speed: Some(110.0),
            ..
        })
    ));
}

#[tokio::test]
async fn status_starts_disconnected() {
    let state = AppState::new(&AppConfig {
        garmin_host: "0.0.0.0".into(),
        garmin_port: 2483,
        api_host: "127.0.0.1".into(),
        api_port: 5178,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: false,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 8765,
    });

    let status = state.status().await;
    assert_eq!(
        status.garmin.connection_status,
        ConnectionStatus::Disconnected
    );
    assert_eq!(status.api_port, 5178);
    assert!(!status.gspro.enabled);
    assert!(!status.nova_ws.enabled);
}
