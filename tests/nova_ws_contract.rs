use futures_util::StreamExt;
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::{AppState, ConnectionStatus, ShotEvent};
use garmingolf_connector::nova::{shot_to_nova_json, spawn_server};
use tokio::time::{timeout, Duration, Instant};

#[test]
fn serializes_shot_to_nova_style_json() {
    let json = shot_to_nova_json(&ShotEvent::test_shot(44));
    let value: serde_json::Value = serde_json::from_str(&json).expect("json");

    assert_eq!(value["type"], "shot");
    assert_eq!(value["shot_number"], 44);
    assert_eq!(value["ball_speed_miles_per_hour"], 98.5);
    assert_eq!(value["vertical_launch_angle_degrees"], 13.5);
    assert_eq!(value["horizontal_launch_angle_degrees"], 0.0);
    assert_eq!(value["total_spin_rpm"], 2350.2);
    assert_eq!(value["spin_axis_degrees"], -10.2);
}

#[tokio::test]
async fn websocket_subscriber_receives_published_shot() {
    let config = AppConfig {
        garmin_host: "127.0.0.1".into(),
        garmin_port: 0,
        api_host: "127.0.0.1".into(),
        api_port: 0,
        gspro_enabled: false,
        gspro_host: "127.0.0.1".into(),
        gspro_port: 921,
        nova_ws_enabled: true,
        nova_ws_host: "127.0.0.1".into(),
        nova_ws_port: 0,
    };
    let state = AppState::new(&config);
    let addr = spawn_server(config, state.clone()).await.expect("server");
    let status = state.status().await;

    assert_eq!(
        status.nova_ws.connection_status,
        ConnectionStatus::Listening
    );
    assert_eq!(status.nova_ws.port, addr.port());

    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("websocket");

    let deadline = Instant::now() + Duration::from_secs(2);
    let value = loop {
        state.publish_shot(ShotEvent::test_shot(55)).await;

        let remaining = deadline.saturating_duration_since(Instant::now());
        assert!(
            !remaining.is_zero(),
            "timed out waiting for Nova websocket shot"
        );

        match timeout(Duration::from_millis(100).min(remaining), socket.next()).await {
            Ok(Some(Ok(message))) => {
                let text = message.into_text().expect("text message");
                let value: serde_json::Value = serde_json::from_str(&text).expect("json message");
                if value["shot_number"] == 55 {
                    break value;
                }
            }
            Ok(Some(Err(err))) => panic!("websocket receive failed: {err}"),
            Ok(None) => panic!("websocket closed before receiving shot"),
            Err(_) => {}
        }
    };

    assert_eq!(value["type"], "shot");
    assert_eq!(value["shot_number"], 55);
    assert_eq!(value["ball_speed_miles_per_hour"], 98.5);
    assert_eq!(value["vertical_launch_angle_degrees"], 13.5);
    assert_eq!(value["horizontal_launch_angle_degrees"], 0.0);
    assert_eq!(value["total_spin_rpm"], 2350.2);
    assert_eq!(value["spin_axis_degrees"], -10.2);
}
