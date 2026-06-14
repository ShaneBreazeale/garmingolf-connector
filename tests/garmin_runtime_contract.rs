use std::sync::Arc;

use garmingolf_connector::config::AppConfig;
use garmingolf_connector::core::AppState;
use garmingolf_connector::garmin::runtime::spawn_listener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Barrier;
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

#[tokio::test]
async fn tcp_runtime_only_numbers_successfully_published_shots() {
    let config = runtime_config();
    let state = AppState::new(&config);
    let mut shots = state.subscribe_shots();
    let addr = spawn_listener(config, state).await.expect("listener");
    let mut client = TcpStream::connect(addr).await.expect("client");

    client.write_all(br#"{"Type":"SendShot"}"#).await.unwrap();
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
}

#[tokio::test]
async fn tcp_runtime_assigns_unique_shot_numbers_across_clients() {
    let config = runtime_config();
    let state = AppState::new(&config);
    let mut shots = state.subscribe_shots();
    let addr = spawn_listener(config, state).await.expect("listener");
    let client_count = 24;
    let barrier = Arc::new(Barrier::new(client_count));
    let mut clients = Vec::new();

    for index in 0..client_count {
        let barrier = barrier.clone();
        clients.push(tokio::spawn(async move {
            let mut client = TcpStream::connect(addr).await.expect("client");
            let ball_speed = 100.0 + index as f64;
            let set_ball_data = format!(
                r#"{{"Type":"SetBallData","BallData":{{"BallSpeed":{ball_speed},"SpinAxis":10.0,"TotalSpin":3000.0,"LaunchDirection":1.0,"LaunchAngle":12.0}}}}"#
            );
            client.write_all(set_ball_data.as_bytes()).await.unwrap();
            barrier.wait().await;
            client.write_all(br#"{"Type":"SendShot"}"#).await.unwrap();
        }));
    }

    let mut shot_numbers = Vec::new();
    for _ in 0..client_count {
        let shot = timeout(Duration::from_secs(2), shots.recv())
            .await
            .expect("shot timeout")
            .expect("shot");
        shot_numbers.push(shot.shot_number);
    }
    shot_numbers.sort_unstable();

    for client in clients {
        client.await.expect("client task");
    }

    let expected = (1..=client_count as u64).collect::<Vec<_>>();
    assert_eq!(shot_numbers, expected);
}
