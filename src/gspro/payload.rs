use serde::Serialize;

use crate::core::{ClubMetrics, ShotEvent};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GsProShotPayload {
    #[serde(rename = "DeviceID")]
    pub device_id: String,
    pub units: String,
    pub shot_number: u64,
    #[serde(rename = "APIversion")]
    pub api_version: String,
    pub ball_data: GsProBallData,
    pub shot_data_options: ShotDataOptions,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub club_data: Option<GsProClubData>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GsProBallData {
    pub speed: f64,
    pub spin_axis: f64,
    pub total_spin: f64,
    #[serde(rename = "HLA")]
    pub hla: f64,
    #[serde(rename = "VLA")]
    pub vla: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct ShotDataOptions {
    pub contains_ball_data: bool,
    pub contains_club_data: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
pub struct GsProClubData {
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

impl GsProShotPayload {
    pub fn from_shot(shot: &ShotEvent, include_club_data: bool) -> Self {
        let club_data = include_club_data
            .then(|| shot.club.as_ref().map(GsProClubData::from))
            .flatten();

        Self {
            device_id: shot.device_id.clone(),
            units: shot.units.clone(),
            shot_number: shot.shot_number,
            api_version: "1".into(),
            ball_data: GsProBallData {
                speed: shot.ball.ball_speed,
                spin_axis: shot.ball.spin_axis,
                total_spin: shot.ball.total_spin,
                hla: shot.ball.hla,
                vla: shot.ball.vla,
            },
            shot_data_options: ShotDataOptions {
                contains_ball_data: true,
                contains_club_data: club_data.is_some(),
            },
            club_data,
        }
    }
}

impl From<&ClubMetrics> for GsProClubData {
    fn from(club: &ClubMetrics) -> Self {
        Self {
            speed: club.speed,
            angle_of_attack: club.angle_of_attack,
            face_to_target: club.face_to_target,
            lie: club.lie,
            loft: club.loft,
            path: club.path,
            speed_at_impact: club.speed_at_impact,
            vertical_face_impact: club.vertical_face_impact,
            horizontal_face_impact: club.horizontal_face_impact,
            closure_rate: club.closure_rate,
        }
    }
}
