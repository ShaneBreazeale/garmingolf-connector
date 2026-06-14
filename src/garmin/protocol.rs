use serde::Deserialize;
use serde_json::{json, Value};

use crate::core::{BallMetrics, ClubMetrics, ShotEvent};

const HANDSHAKE_CHALLENGE: &str =
    "gQW3om37uK4OOU4FXQH9GWgljxOrNcL5MvubVHAtQC0x6Z1AwJTgAIKyamJJMzm9";

#[derive(Debug, Clone, PartialEq)]
pub enum GarminIncoming {
    Handshake,
    Challenge,
    Disconnect,
    Pong,
    SetClubType { club_type: String },
    SetBallData { data: GarminBallData, raw: Value },
    SetClubData { data: GarminClubData, raw: Value },
    SendShot,
    Unknown(String),
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GarminBallData {
    pub ball_speed: f64,
    pub spin_axis: f64,
    pub total_spin: f64,
    pub launch_direction: f64,
    pub launch_angle: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GarminClubData {
    pub club_head_speed: Option<f64>,
    pub club_angle_face: Option<f64>,
    pub club_angle_path: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct Envelope {
    #[serde(rename = "Type")]
    message_type: String,
}

pub fn parse_incoming(text: &str) -> Result<GarminIncoming, String> {
    let value: Value =
        serde_json::from_str(text).map_err(|err| format!("invalid Garmin JSON: {err}"))?;
    let envelope: Envelope = serde_json::from_value(value.clone())
        .map_err(|err| format!("invalid Garmin message: {err}"))?;

    match envelope.message_type.as_str() {
        "Handshake" => Ok(GarminIncoming::Handshake),
        "Challenge" => Ok(GarminIncoming::Challenge),
        "Disconnect" => Ok(GarminIncoming::Disconnect),
        "Pong" => Ok(GarminIncoming::Pong),
        "SetClubType" => Ok(GarminIncoming::SetClubType {
            club_type: required_string(&value, "ClubType")?,
        }),
        "SetBallData" => {
            let raw = value
                .get("BallData")
                .cloned()
                .ok_or_else(|| "SetBallData missing BallData".to_string())?;
            let data = serde_json::from_value(raw.clone())
                .map_err(|err| format!("invalid BallData: {err}"))?;
            Ok(GarminIncoming::SetBallData { data, raw })
        }
        "SetClubData" => {
            let raw = value
                .get("ClubData")
                .cloned()
                .ok_or_else(|| "SetClubData missing ClubData".to_string())?;
            let data = serde_json::from_value(raw.clone())
                .map_err(|err| format!("invalid ClubData: {err}"))?;
            Ok(GarminIncoming::SetClubData { data, raw })
        }
        "SendShot" => Ok(GarminIncoming::SendShot),
        other => Ok(GarminIncoming::Unknown(other.to_string())),
    }
}

pub fn handshake_json() -> String {
    json!({
        "Challenge": HANDSHAKE_CHALLENGE,
        "E6Version": "2, 0, 0, 0",
        "ProtocolVersion": "1.0.0.5",
        "RequiredProtocolVersion": "1.0.0.0",
        "Type": "Handshake"
    })
    .to_string()
}

pub fn authentication_json() -> String {
    json!({ "Success": "true", "Type": "Authentication" }).to_string()
}

pub fn ack_json(subtype: &str) -> String {
    json!({ "Details": "Success.", "SubType": subtype, "Type": "ACK" }).to_string()
}

pub fn sim_command_json(subtype: &str) -> String {
    json!({ "SubType": subtype, "Type": "SimCommand" }).to_string()
}

pub fn ping_json() -> String {
    sim_command_json("Ping")
}

#[derive(Debug, Clone)]
pub struct ShotAssembler {
    club_type: String,
    ball: Option<(GarminBallData, Value)>,
    club: Option<(GarminClubData, Value)>,
}

impl Default for ShotAssembler {
    fn default() -> Self {
        Self {
            club_type: "7Iron".into(),
            ball: None,
            club: None,
        }
    }
}

impl ShotAssembler {
    pub fn apply(&mut self, incoming: GarminIncoming) {
        match incoming {
            GarminIncoming::SetClubType { club_type } => self.club_type = club_type,
            GarminIncoming::SetBallData { data, raw } => self.ball = Some((data, raw)),
            GarminIncoming::SetClubData { data, raw } => self.club = Some((data, raw)),
            _ => {}
        }
    }

    pub fn build_shot(&self, shot_number: u64) -> Result<ShotEvent, String> {
        let (ball, raw_ball) = self
            .ball
            .clone()
            .ok_or_else(|| "cannot build shot without ball data".to_string())?;
        let (club, raw_club) = self.club.clone().unzip();

        Ok(ShotEvent {
            shot_number,
            device_id: "Garmin R10".into(),
            units: "Yards".into(),
            club_type: self.club_type.clone(),
            ball: BallMetrics {
                ball_speed: ball.ball_speed,
                spin_axis: convert_spin_axis(ball.spin_axis),
                total_spin: ball.total_spin,
                hla: ball.launch_direction,
                vla: ball.launch_angle,
            },
            club: club.map(|club| ClubMetrics {
                speed: club.club_head_speed,
                angle_of_attack: Some(0.0),
                face_to_target: club.club_angle_face,
                lie: Some(0.0),
                loft: Some(0.0),
                path: club.club_angle_path,
                speed_at_impact: club.club_head_speed,
                vertical_face_impact: Some(0.0),
                horizontal_face_impact: Some(0.0),
                closure_rate: Some(0.0),
            }),
            raw_ball_data: Some(raw_ball),
            raw_club_data: raw_club,
        })
    }

    pub fn clear_ball_data_after_publish(&mut self) {
        self.ball = None;
    }
}

fn convert_spin_axis(mut spin_axis: f64) -> f64 {
    if spin_axis > 90.0 {
        spin_axis -= 360.0;
    }
    spin_axis * -1.0
}

fn required_string(value: &Value, field: &str) -> Result<String, String> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or_else(|| format!("missing string field {field}"))
}
