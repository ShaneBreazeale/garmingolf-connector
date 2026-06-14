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
    #[arg(long, env = "GARMINGOLF_GARMIN_HOST", default_value = DEFAULT_GARMIN_HOST)]
    pub garmin_host: String,
    #[arg(long, env = "GARMINGOLF_GARMIN_PORT", default_value_t = DEFAULT_GARMIN_PORT)]
    pub garmin_port: u16,
    #[arg(long, env = "GARMINGOLF_API_HOST", default_value = DEFAULT_API_HOST)]
    pub api_host: String,
    #[arg(long, env = "GARMINGOLF_API_PORT", default_value_t = DEFAULT_API_PORT)]
    pub api_port: u16,
    #[arg(long, env = "GARMINGOLF_ENABLE_GSPRO", default_value_t = DEFAULT_GSPRO_ENABLED)]
    pub enable_gspro: bool,
    #[arg(long, env = "GARMINGOLF_GSPRO_HOST", default_value = DEFAULT_GSPRO_HOST)]
    pub gspro_host: String,
    #[arg(long, env = "GARMINGOLF_GSPRO_PORT", default_value_t = DEFAULT_GSPRO_PORT)]
    pub gspro_port: u16,
    #[arg(long, env = "GARMINGOLF_ENABLE_NOVA_WS", default_value_t = DEFAULT_NOVA_WS_ENABLED)]
    pub enable_nova_ws: bool,
    #[arg(long, env = "GARMINGOLF_NOVA_WS_HOST", default_value = DEFAULT_NOVA_WS_HOST)]
    pub nova_ws_host: String,
    #[arg(long, env = "GARMINGOLF_NOVA_WS_PORT", default_value_t = DEFAULT_NOVA_WS_PORT)]
    pub nova_ws_port: u16,
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
            garmin_host: args.garmin_host,
            garmin_port: args.garmin_port,
            api_host: args.api_host,
            api_port: args.api_port,
            gspro_enabled: args.enable_gspro,
            gspro_host: args.gspro_host,
            gspro_port: args.gspro_port,
            nova_ws_enabled: args.enable_nova_ws,
            nova_ws_host: args.nova_ws_host,
            nova_ws_port: args.nova_ws_port,
        };

        for (key, value) in env {
            let key = key.as_ref();
            let value = value.as_ref();
            match key {
                "GARMINGOLF_GARMIN_HOST" if config.garmin_host == DEFAULT_GARMIN_HOST => {
                    config.garmin_host = value.to_string();
                }
                "GARMINGOLF_GARMIN_PORT" if config.garmin_port == DEFAULT_GARMIN_PORT => {
                    config.garmin_port = parse_port(key, value)?;
                }
                "GARMINGOLF_API_HOST" if config.api_host == DEFAULT_API_HOST => {
                    config.api_host = value.to_string();
                }
                "GARMINGOLF_API_PORT" if config.api_port == DEFAULT_API_PORT => {
                    config.api_port = parse_port(key, value)?;
                }
                "GARMINGOLF_ENABLE_GSPRO" if config.gspro_enabled == DEFAULT_GSPRO_ENABLED => {
                    config.gspro_enabled = parse_bool(key, value)?;
                }
                "GARMINGOLF_GSPRO_HOST" if config.gspro_host == DEFAULT_GSPRO_HOST => {
                    config.gspro_host = value.to_string();
                }
                "GARMINGOLF_GSPRO_PORT" if config.gspro_port == DEFAULT_GSPRO_PORT => {
                    config.gspro_port = parse_port(key, value)?;
                }
                "GARMINGOLF_ENABLE_NOVA_WS"
                    if config.nova_ws_enabled == DEFAULT_NOVA_WS_ENABLED =>
                {
                    config.nova_ws_enabled = parse_bool(key, value)?;
                }
                "GARMINGOLF_NOVA_WS_HOST" if config.nova_ws_host == DEFAULT_NOVA_WS_HOST => {
                    config.nova_ws_host = value.to_string();
                }
                "GARMINGOLF_NOVA_WS_PORT" if config.nova_ws_port == DEFAULT_NOVA_WS_PORT => {
                    config.nova_ws_port = parse_port(key, value)?;
                }
                _ => {}
            }
        }

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
