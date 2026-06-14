use std::net::SocketAddr;

use clap::Parser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

const DEFAULT_GARMIN_HOST: &str = "0.0.0.0";
const DEFAULT_GARMIN_PORT: u16 = 2483;
const DEFAULT_API_HOST: &str = "127.0.0.1";
const DEFAULT_API_PORT: u16 = 5178;
const DEFAULT_GSPRO_ENABLED: bool = false;
const DEFAULT_GSPRO_HOST: &str = "127.0.0.1";
const DEFAULT_GSPRO_PORT: u16 = 921;
const DEFAULT_NOVA_WS_ENABLED: bool = false;
const DEFAULT_NOVA_WS_HOST: &str = "127.0.0.1";
const DEFAULT_NOVA_WS_PORT: u16 = 8765;

#[derive(Debug, Clone, Parser)]
#[command(version, about = "Garmin Golf launch monitor bridge", long_about = None)]
pub struct CliArgs {
    #[arg(long)]
    pub garmin_host: Option<String>,
    #[arg(long)]
    pub garmin_port: Option<u16>,
    #[arg(long)]
    pub api_host: Option<String>,
    #[arg(long)]
    pub api_port: Option<u16>,
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub enable_gspro: Option<bool>,
    #[arg(long)]
    pub gspro_host: Option<String>,
    #[arg(long)]
    pub gspro_port: Option<u16>,
    #[arg(long, num_args = 0..=1, default_missing_value = "true", require_equals = true)]
    pub enable_nova_ws: Option<bool>,
    #[arg(long)]
    pub nova_ws_host: Option<String>,
    #[arg(long)]
    pub nova_ws_port: Option<u16>,
}

impl CliArgs {
    pub fn parse_cli() -> Self {
        Self::parse()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub garmin_host: String,
    pub garmin_port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub gspro_enabled: bool,
    pub gspro_host: String,
    pub gspro_port: u16,
    pub nova_ws_enabled: bool,
    pub nova_ws_host: String,
    pub nova_ws_port: u16,
}

impl AppConfig {
    pub fn from_env() -> Result<Self, String> {
        Self::from_cli_and_env(CliArgs::parse_cli(), std::env::vars())
    }

    pub fn from_cli_and_env<I, K, V>(args: CliArgs, env: I) -> Result<Self, String>
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let mut config = Self {
            garmin_host: DEFAULT_GARMIN_HOST.to_string(),
            garmin_port: DEFAULT_GARMIN_PORT,
            api_host: DEFAULT_API_HOST.to_string(),
            api_port: DEFAULT_API_PORT,
            gspro_enabled: DEFAULT_GSPRO_ENABLED,
            gspro_host: DEFAULT_GSPRO_HOST.to_string(),
            gspro_port: DEFAULT_GSPRO_PORT,
            nova_ws_enabled: DEFAULT_NOVA_WS_ENABLED,
            nova_ws_host: DEFAULT_NOVA_WS_HOST.to_string(),
            nova_ws_port: DEFAULT_NOVA_WS_PORT,
        };

        for (key, value) in env {
            let key = key.as_ref();
            let value = value.as_ref();
            match key {
                "GARMINGOLF_GARMIN_HOST" => {
                    config.garmin_host = value.to_string();
                }
                "GARMINGOLF_GARMIN_PORT" => {
                    config.garmin_port = parse_port(key, value)?;
                }
                "GARMINGOLF_API_HOST" => {
                    config.api_host = value.to_string();
                }
                "GARMINGOLF_API_PORT" => {
                    config.api_port = parse_port(key, value)?;
                }
                "GARMINGOLF_ENABLE_GSPRO" => {
                    config.gspro_enabled = parse_bool(key, value)?;
                }
                "GARMINGOLF_GSPRO_HOST" => {
                    config.gspro_host = value.to_string();
                }
                "GARMINGOLF_GSPRO_PORT" => {
                    config.gspro_port = parse_port(key, value)?;
                }
                "GARMINGOLF_ENABLE_NOVA_WS" => {
                    config.nova_ws_enabled = parse_bool(key, value)?;
                }
                "GARMINGOLF_NOVA_WS_HOST" => {
                    config.nova_ws_host = value.to_string();
                }
                "GARMINGOLF_NOVA_WS_PORT" => {
                    config.nova_ws_port = parse_port(key, value)?;
                }
                _ => {}
            }
        }

        if let Some(value) = args.garmin_host {
            config.garmin_host = value;
        }
        if let Some(value) = args.garmin_port {
            config.garmin_port = value;
        }
        if let Some(value) = args.api_host {
            config.api_host = value;
        }
        if let Some(value) = args.api_port {
            config.api_port = value;
        }
        if let Some(value) = args.enable_gspro {
            config.gspro_enabled = value;
        }
        if let Some(value) = args.gspro_host {
            config.gspro_host = value;
        }
        if let Some(value) = args.gspro_port {
            config.gspro_port = value;
        }
        if let Some(value) = args.enable_nova_ws {
            config.nova_ws_enabled = value;
        }
        if let Some(value) = args.nova_ws_host {
            config.nova_ws_host = value;
        }
        if let Some(value) = args.nova_ws_port {
            config.nova_ws_port = value;
        }

        config.validate_socket_addrs()?;

        Ok(config)
    }

    pub fn garmin_addr(&self) -> SocketAddr {
        format!("{}:{}", self.garmin_host, self.garmin_port)
            .parse()
            .expect("validated garmin socket address")
    }

    pub fn api_addr(&self) -> SocketAddr {
        format!("{}:{}", self.api_host, self.api_port)
            .parse()
            .expect("validated api socket address")
    }

    pub fn nova_ws_addr(&self) -> SocketAddr {
        format!("{}:{}", self.nova_ws_host, self.nova_ws_port)
            .parse()
            .expect("validated nova websocket socket address")
    }

    fn validate_socket_addrs(&self) -> Result<(), String> {
        parse_socket_addr("garmin address", &self.garmin_host, self.garmin_port)?;
        parse_socket_addr("api address", &self.api_host, self.api_port)?;
        parse_socket_addr(
            "nova websocket address",
            &self.nova_ws_host,
            self.nova_ws_port,
        )?;
        Ok(())
    }
}

fn parse_socket_addr(name: &str, host: &str, port: u16) -> Result<SocketAddr, String> {
    format!("{host}:{port}")
        .parse()
        .map_err(|err| format!("{name} must be a valid socket address: {err}"))
}

fn parse_port(name: &str, value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .map_err(|err| format!("{name} must be a valid u16 port: {err}"))
}

fn parse_bool(name: &str, value: &str) -> Result<bool, String> {
    value
        .parse::<bool>()
        .map_err(|err| format!("{name} must be true or false: {err}"))
}
