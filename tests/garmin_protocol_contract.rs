use garmingolf_connector::garmin::protocol::{
    ack_json, authentication_json, handshake_json, parse_incoming, ping_json, sim_command_json,
    GarminIncoming, ShotAssembler,
};

#[test]
fn handshake_and_authentication_match_existing_e6_bridge() {
    assert_eq!(
        handshake_json(),
        r#"{"Challenge":"gQW3om37uK4OOU4FXQH9GWgljxOrNcL5MvubVHAtQC0x6Z1AwJTgAIKyamJJMzm9","E6Version":"2, 0, 0, 0","ProtocolVersion":"1.0.0.5","RequiredProtocolVersion":"1.0.0.0","Type":"Handshake"}"#
    );
    assert_eq!(
        authentication_json(),
        r#"{"Success":"true","Type":"Authentication"}"#
    );
    assert_eq!(
        ack_json("SetBallData"),
        r#"{"Details":"Success.","SubType":"SetBallData","Type":"ACK"}"#
    );
    assert_eq!(
        sim_command_json("Arm"),
        r#"{"SubType":"Arm","Type":"SimCommand"}"#
    );
    assert_eq!(ping_json(), r#"{"SubType":"Ping","Type":"SimCommand"}"#);
}

#[test]
fn parses_known_incoming_message_types() {
    assert!(matches!(
        parse_incoming(r#"{"Type":"Handshake"}"#).expect("message"),
        GarminIncoming::Handshake
    ));
    assert!(matches!(
        parse_incoming(r#"{"Type":"SetClubType","ClubType":"Driver"}"#).expect("message"),
        GarminIncoming::SetClubType { club_type } if club_type == "Driver"
    ));
}

#[test]
fn assembles_shot_with_spin_axis_conversion_and_club_metrics() {
    let mut assembler = ShotAssembler::default();
    assembler.apply(parse_incoming(r#"{"Type":"SetClubType","ClubType":"7Iron"}"#).unwrap());
    assembler.apply(parse_incoming(r#"{"Type":"SetBallData","BallData":{"BallSpeed":151.58,"SpinAxis":353.3982,"TotalSpin":4721.59,"LaunchDirection":-5.0065,"LaunchAngle":17.7736}}"#).unwrap());
    assembler.apply(parse_incoming(r#"{"Type":"SetClubData","ClubData":{"ClubHeadSpeed":110.31,"ClubAngleFace":-2.42,"ClubAnglePath":-10.28}}"#).unwrap());

    let shot = assembler.build_shot(9).expect("shot");

    assert_eq!(shot.shot_number, 9);
    assert_eq!(shot.club_type, "7Iron");
    assert!((shot.ball.spin_axis - 6.6018).abs() < 0.001);
    assert_eq!(shot.ball.total_spin, 4721.59);
    assert_eq!(shot.club.unwrap().face_to_target, Some(-2.42));
}

#[test]
fn malformed_json_returns_error_instead_of_panicking() {
    let err = parse_incoming("{").expect_err("invalid json");
    assert!(err.contains("invalid Garmin JSON"));
}
