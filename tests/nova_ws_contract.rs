use futures_util::StreamExt;
use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::{AppState, ShotEvent};
use garmingolf_connector::nova::{shot_to_nova_json, spawn_server};

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

    let (mut socket, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws"))
        .await
        .expect("websocket");

    state.publish_shot(ShotEvent::test_shot(55)).await;
    let message = socket
        .next()
        .await
        .expect("message")
        .expect("ok")
        .into_text()
        .unwrap();

    assert!(message.contains(r#""type":"shot""#));
    assert!(message.contains(r#""shot_number":55"#));
}
