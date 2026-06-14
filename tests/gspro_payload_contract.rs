use garmingolf_connector::core::{ClubMetrics, ShotEvent};
use garmingolf_connector::gspro::payload::GsProShotPayload;

#[test]
fn converts_test_shot_to_gspro_open_api_payload() {
    let shot = ShotEvent::test_shot(12);
    let payload = GsProShotPayload::from_shot(&shot, true);
    let json = serde_json::to_value(payload).expect("json");

    assert_eq!(json["DeviceID"], "Garmin R10");
    assert_eq!(json["Units"], "Yards");
    assert_eq!(json["ShotNumber"], 12);
    assert_eq!(json["APIversion"], "1");
    assert_eq!(json["BallData"]["Speed"], 98.5);
    assert_eq!(json["BallData"]["SpinAxis"], -10.2);
    assert_eq!(json["BallData"]["TotalSpin"], 2350.2);
    assert_eq!(json["BallData"]["HLA"], 0.0);
    assert_eq!(json["BallData"]["VLA"], 13.5);
    assert_eq!(json["ShotDataOptions"]["ContainsBallData"], true);
    assert_eq!(json["ShotDataOptions"]["ContainsClubData"], true);
    assert_eq!(json["ClubData"]["Speed"], 110.0);
}

#[test]
fn can_omit_club_data_even_when_shot_has_club_metrics() {
    let shot = ShotEvent::test_shot(1);
    let json = serde_json::to_value(GsProShotPayload::from_shot(&shot, false)).unwrap();

    assert_eq!(json["ShotDataOptions"]["ContainsClubData"], false);
    assert!(json.get("ClubData").is_none());
}

#[test]
fn omits_missing_optional_club_metrics() {
    let mut shot = ShotEvent::test_shot(2);
    shot.club = Some(ClubMetrics {
        speed: Some(101.5),
        angle_of_attack: None,
        face_to_target: Some(-1.2),
        lie: None,
        loft: Some(18.0),
        path: None,
        speed_at_impact: None,
        vertical_face_impact: Some(0.1),
        horizontal_face_impact: None,
        closure_rate: Some(3.0),
    });

    let json = serde_json::to_value(GsProShotPayload::from_shot(&shot, true)).unwrap();
    let club_data = json["ClubData"].as_object().expect("club data object");

    assert_eq!(club_data["Speed"], 101.5);
    assert_eq!(club_data["FaceToTarget"], -1.2);
    assert_eq!(club_data["Loft"], 18.0);
    assert_eq!(club_data["VerticalFaceImpact"], 0.1);
    assert_eq!(club_data["ClosureRate"], 3.0);
    assert!(!club_data.contains_key("AngleOfAttack"));
    assert!(!club_data.contains_key("Lie"));
    assert!(!club_data.contains_key("Path"));
    assert!(!club_data.contains_key("SpeedAtImpact"));
    assert!(!club_data.contains_key("HorizontalFaceImpact"));
}
