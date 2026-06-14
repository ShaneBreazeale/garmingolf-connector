use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;
use garmingolf_connector::garmin::runtime::spawn_listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

fn runtime_config() -> AppConfig {
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

#[tokio::test]
async fn tcp_runtime_responds_to_garmin_messages_and_publishes_shot() {
    let config = runtime_config();
    let state = AppState::new(&config);
    let mut shots = state.subscribe_shots();
    let addr = spawn_listener(config, state).await.expect("listener");
    let mut client = TcpStream::connect(addr).await.expect("client");

    client.write_all(br#"{"Type":"Handshake"}"#).await.unwrap();
    let mut buf = vec![0; 1024];
    let n = timeout(Duration::from_secs(2), client.read(&mut buf))
        .await
        .expect("handshake timeout")
        .expect("handshake response");
    assert!(String::from_utf8_lossy(&buf[..n]).contains(r#""Type":"Handshake""#));

    client.write_all(br#"{"Type":"SetBallData","BallData":{"BallSpeed":151.58,"SpinAxis":353.3982,"TotalSpin":4721.59,"LaunchDirection":-5.0065,"LaunchAngle":17.7736}}"#).await.unwrap();
    client.write_all(br#"{"Type":"SendShot"}"#).await.unwrap();

    let shot = timeout(Duration::from_secs(2), shots.recv())
        .await
        .expect("shot timeout")
        .expect("shot");
    assert_eq!(shot.shot_number, 1);
    assert_eq!(shot.ball.ball_speed, 151.58);
}

#[tokio::test]
async fn tcp_runtime_handles_multiple_json_messages_in_one_read() {
    let config = runtime_config();
    let state = AppState::new(&config);
    let mut shots = state.subscribe_shots();
    let addr = spawn_listener(config, state).await.expect("listener");
    let mut client = TcpStream::connect(addr).await.expect("client");

    client
        .write_all(br#"{"Type":"SetBallData","BallData":{"BallSpeed":120.0,"SpinAxis":10.0,"TotalSpin":3000.0,"LaunchDirection":1.0,"LaunchAngle":12.0}}{"Type":"SendShot"}"#)
        .await
        .unwrap();

    let shot = timeout(Duration::from_secs(2), shots.recv())
        .await
        .expect("shot timeout")
        .expect("shot");
    assert_eq!(shot.shot_number, 1);
    assert_eq!(shot.ball.ball_speed, 120.0);
    assert_eq!(shot.ball.spin_axis, -10.0);
}
