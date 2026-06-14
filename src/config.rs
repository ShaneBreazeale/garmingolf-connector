use std::net::SocketAddr;

use clap::Parser;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Parser)]
#[command(version, about = "Garmin Golf launch monitor bridge", long_about = None)]
pub struct CliArgs {
    #[arg(long, env = "GARMINGOLF_GARMIN_HOST", default_value = "0.0.0.0")]
    pub garmin_host: String,
    #[arg(long, env = "GARMINGOLF_GARMIN_PORT", default_value_t = 2483)]
    pub garmin_port: u16,
    #[arg(long, env = "GARMINGOLF_API_HOST", default_value = "127.0.0.1")]
    pub api_host: String,
    #[arg(long, env = "GARMINGOLF_API_PORT", default_value_t = 5178)]
    pub api_port: u16,
    #[arg(long, env = "GARMINGOLF_ENABLE_GSPRO", default_value_t = false)]
    pub enable_gspro: bool,
    #[arg(long, env = "GARMINGOLF_GSPRO_HOST", default_value = "127.0.0.1")]
    pub gspro_host: String,
    #[arg(long, env = "GARMINGOLF_GSPRO_PORT", default_value_t = 921)]
    pub gspro_port: u16,
    #[arg(long, env = "GARMINGOLF_ENABLE_NOVA_WS", default_value_t = false)]
    pub enable_nova_ws: bool,
    #[arg(long, env = "GARMINGOLF_NOVA_WS_HOST", default_value = "127.0.0.1")]
    pub nova_ws_host: String,
    #[arg(long, env = "GARMINGOLF_NOVA_WS_PORT", default_value_t = 8765)]
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
                "GARMINGOLF_GARMIN_PORT" if config.garmin_port == 2483 => {
                    config.garmin_port = parse_port(key, value)?;
                }
                "GARMINGOLF_API_PORT" if config.api_port == 5178 => {
                    config.api_port = parse_port(key, value)?;
                }
                "GARMINGOLF_ENABLE_NOVA_WS" if !config.nova_ws_enabled => {
                    config.nova_ws_enabled = parse_bool(key, value)?;
                }
                "GARMINGOLF_NOVA_WS_PORT" if config.nova_ws_port == 8765 => {
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
