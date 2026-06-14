use clap::Parser;
use garmingolf_connector::config::{AppConfig, CliArgs};

#[test]
fn default_config_matches_existing_connector_ports() {
    let config = AppConfig::from_cli_and_env(
        CliArgs::try_parse_from(["garmingolf-connector"]).expect("valid defaults"),
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("config");

    assert_eq!(config.garmin_host, "0.0.0.0");
    assert_eq!(config.garmin_port, 2483);
    assert_eq!(config.api_host, "127.0.0.1");
    assert_eq!(config.api_port, 5178);
    assert!(!config.gspro_enabled);
    assert_eq!(config.gspro_host, "127.0.0.1");
    assert_eq!(config.gspro_port, 921);
    assert!(!config.nova_ws_enabled);
    assert_eq!(config.nova_ws_host, "127.0.0.1");
    assert_eq!(config.nova_ws_port, 8765);
}

#[test]
fn env_values_override_defaults_and_cli_values_override_env() {
    let env = [
        ("GARMINGOLF_API_PORT", "6000"),
        ("GARMINGOLF_GARMIN_PORT", "2500"),
        ("GARMINGOLF_ENABLE_NOVA_WS", "true"),
        ("GARMINGOLF_NOVA_WS_PORT", "9900"),
    ];
    let args = CliArgs::try_parse_from([
        "garmingolf-connector",
        "--api-port",
        "7000",
        "--enable-gspro",
        "--gspro-host",
        "192.0.2.10",
    ])
    .expect("valid cli");

    let config = AppConfig::from_cli_and_env(args, env).expect("config");

    assert_eq!(config.api_port, 7000);
    assert_eq!(config.garmin_port, 2500);
    assert!(config.gspro_enabled);
    assert_eq!(config.gspro_host, "192.0.2.10");
    assert!(config.nova_ws_enabled);
    assert_eq!(config.nova_ws_port, 9900);
}

#[test]
fn socket_addresses_are_formed_from_host_and_port() {
    let config = AppConfig::from_cli_and_env(
        CliArgs::try_parse_from([
            "garmingolf-connector",
            "--garmin-host",
            "127.0.0.1",
            "--garmin-port",
            "0",
            "--api-port",
            "0",
        ])
        .expect("valid cli"),
        std::iter::empty::<(&str, &str)>(),
    )
    .expect("config");

    assert_eq!(config.garmin_addr().to_string(), "127.0.0.1:0");
    assert_eq!(config.api_addr().to_string(), "127.0.0.1:0");
}
