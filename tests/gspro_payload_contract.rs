use garmingolf_connector::core::ShotEvent;
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
