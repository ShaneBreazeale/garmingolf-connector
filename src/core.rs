use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, RwLock};
use utoipa::ToSchema;

use crate::config::AppConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionStatus {
    Disconnected,
    Listening,
    Connecting,
    Connected,
    Error,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BallMetrics {
    pub ball_speed: f64,
    pub spin_axis: f64,
    pub total_spin: f64,
    pub hla: f64,
    pub vla: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClubMetrics {
    pub speed: Option<f64>,
    pub angle_of_attack: Option<f64>,
    pub face_to_target: Option<f64>,
    pub lie: Option<f64>,
    pub loft: Option<f64>,
    pub path: Option<f64>,
    pub speed_at_impact: Option<f64>,
    pub vertical_face_impact: Option<f64>,
    pub horizontal_face_impact: Option<f64>,
    pub closure_rate: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShotEvent {
    pub shot_number: u64,
    pub device_id: String,
    pub units: String,
    pub club_type: String,
    pub ball: BallMetrics,
    pub club: Option<ClubMetrics>,
    pub raw_ball_data: Option<serde_json::Value>,
    pub raw_club_data: Option<serde_json::Value>,
}

impl ShotEvent {
    pub fn test_shot(shot_number: u64) -> Self {
        Self {
            shot_number,
            device_id: "Garmin R10".into(),
            units: "Yards".into(),
            club_type: "7Iron".into(),
            ball: BallMetrics {
                ball_speed: 98.5,
                spin_axis: -10.2,
                total_spin: 2350.2,
                hla: 0.0,
                vla: 13.5,
            },
            club: Some(ClubMetrics {
                speed: Some(110.0),
                angle_of_attack: Some(0.0),
                face_to_target: Some(-2.4),
                lie: Some(0.0),
                loft: Some(0.0),
                path: Some(-10.2),
                speed_at_impact: Some(110.0),
                vertical_face_impact: Some(0.0),
                horizontal_face_impact: Some(0.0),
                closure_rate: Some(0.0),
            }),
            raw_ball_data: None,
            raw_club_data: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct EndpointStatus {
    pub enabled: bool,
    pub connection_status: ConnectionStatus,
    pub host: String,
    pub port: u16,
    pub last_error: Option<String>,
    pub last_shot_number: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GarminStatus {
    pub connection_status: ConnectionStatus,
    pub host: String,
    pub port: u16,
    pub active_client: Option<String>,
    pub last_error: Option<String>,
    pub malformed_message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppStatus {
    pub api_port: u16,
    pub garmin: GarminStatus,
    pub gspro: EndpointStatus,
    pub nova_ws: EndpointStatus,
    pub last_shot: Option<ShotEvent>,
}

#[derive(Clone)]
pub struct AppState {
    inner: Arc<RwLock<AppStatus>>,
    shots: broadcast::Sender<ShotEvent>,
}

impl AppState {
    pub fn new(config: &AppConfig) -> Self {
        let (shots, _) = broadcast::channel(128);
        Self {
            inner: Arc::new(RwLock::new(AppStatus {
                api_port: config.api_port,
                garmin: GarminStatus {
                    connection_status: ConnectionStatus::Disconnected,
                    host: config.garmin_host.clone(),
                    port: config.garmin_port,
                    active_client: None,
                    last_error: None,
                    malformed_message_count: 0,
                },
                gspro: EndpointStatus {
                    enabled: config.gspro_enabled,
                    connection_status: ConnectionStatus::Disconnected,
                    host: config.gspro_host.clone(),
                    port: config.gspro_port,
                    last_error: None,
                    last_shot_number: None,
                },
                nova_ws: EndpointStatus {
                    enabled: config.nova_ws_enabled,
                    connection_status: ConnectionStatus::Disconnected,
                    host: config.nova_ws_host.clone(),
                    port: config.nova_ws_port,
                    last_error: None,
                    last_shot_number: None,
                },
                last_shot: None,
            })),
            shots,
        }
    }

    pub async fn status(&self) -> AppStatus {
        self.inner.read().await.clone()
    }

    pub async fn set_api_port(&self, port: u16) {
        self.inner.write().await.api_port = port;
    }

    pub fn subscribe_shots(&self) -> broadcast::Receiver<ShotEvent> {
        self.shots.subscribe()
    }

    pub async fn publish_shot(&self, shot: ShotEvent) {
        {
            let mut status = self.inner.write().await;
            status.last_shot = Some(shot.clone());
        }
        let _ = self.shots.send(shot);
    }

    pub async fn update_garmin<F>(&self, update: F)
    where
        F: FnOnce(&mut GarminStatus),
    {
        let mut status = self.inner.write().await;
        update(&mut status.garmin);
    }

    pub async fn update_gspro<F>(&self, update: F)
    where
        F: FnOnce(&mut EndpointStatus),
    {
        let mut status = self.inner.write().await;
        update(&mut status.gspro);
    }

    pub async fn update_nova<F>(&self, update: F)
    where
        F: FnOnce(&mut EndpointStatus),
    {
        let mut status = self.inner.write().await;
        update(&mut status.nova_ws);
    }
}
