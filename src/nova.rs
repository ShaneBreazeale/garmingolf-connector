use std::net::SocketAddr;

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde_json::json;
use tokio::net::TcpListener;

use crate::config::AppConfig;
use crate::core::{AppState, ConnectionStatus, ShotEvent};

pub fn shot_to_nova_json(shot: &ShotEvent) -> String {
    json!({
        "type": "shot",
        "shot_number": shot.shot_number,
        "ball_speed_miles_per_hour": shot.ball.ball_speed,
        "vertical_launch_angle_degrees": shot.ball.vla,
        "horizontal_launch_angle_degrees": shot.ball.hla,
        "total_spin_rpm": shot.ball.total_spin,
        "spin_axis_degrees": shot.ball.spin_axis
    })
    .to_string()
}

pub async fn spawn_server(config: AppConfig, state: AppState) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(config.nova_ws_addr())
        .await
        .map_err(|err| format!("Nova WebSocket bind failed: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("Nova WebSocket local_addr failed: {err}"))?;
    state
        .update_nova(|status| {
            status.connection_status = ConnectionStatus::Listening;
            status.port = addr.port();
            status.last_error = None;
        })
        .await;

    let router = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state.clone());

    tokio::spawn(async move {
        if let Err(err) = axum::serve(listener, router).await {
            tracing::error!("Nova WebSocket server failed: {err}");
        }
    });
    Ok(addr)
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| pump_shots(socket, state))
}

async fn pump_shots(mut socket: WebSocket, state: AppState) {
    let mut shots = state.subscribe_shots();
    while let Ok(shot) = shots.recv().await {
        if socket
            .send(Message::Text(shot_to_nova_json(&shot).into()))
            .await
            .is_err()
        {
            break;
        }
    }
}
