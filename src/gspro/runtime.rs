use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tokio::time::{sleep, Duration};

use crate::config::AppConfig;
use crate::core::{AppState, ConnectionStatus};

use super::payload::GsProShotPayload;

pub async fn spawn_forwarder(config: AppConfig, state: AppState) {
    if !config.gspro_enabled {
        return;
    }

    tokio::spawn(async move {
        let mut shots = state.subscribe_shots();
        loop {
            let addr = format!("{}:{}", config.gspro_host, config.gspro_port);
            state
                .update_gspro(|status| {
                    status.connection_status = ConnectionStatus::Connecting;
                    status.last_error = None;
                })
                .await;

            match TcpStream::connect(&addr).await {
                Ok(mut socket) => {
                    state
                        .update_gspro(|status| {
                            status.connection_status = ConnectionStatus::Connected;
                            status.last_error = None;
                        })
                        .await;

                    while let Ok(shot) = shots.recv().await {
                        let payload = GsProShotPayload::from_shot(&shot, true);
                        match serde_json::to_vec(&payload) {
                            Ok(bytes) => {
                                if let Err(err) = socket.write_all(&bytes).await {
                                    state
                                        .update_gspro(|status| {
                                            status.connection_status = ConnectionStatus::Error;
                                            status.last_error =
                                                Some(format!("GSPro send failed: {err}"));
                                        })
                                        .await;
                                    break;
                                }
                                state
                                    .update_gspro(|status| {
                                        status.last_shot_number = Some(shot.shot_number);
                                    })
                                    .await;
                            }
                            Err(err) => {
                                state
                                    .update_gspro(|status| {
                                        status.last_error =
                                            Some(format!("GSPro payload serialize failed: {err}"));
                                    })
                                    .await;
                            }
                        }
                    }
                }
                Err(err) => {
                    state
                        .update_gspro(|status| {
                            status.connection_status = ConnectionStatus::Error;
                            status.last_error = Some(format!("GSPro connect failed: {err}"));
                        })
                        .await;
                    sleep(Duration::from_millis(250)).await;
                }
            }
        }
    });
}
