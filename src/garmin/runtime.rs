use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

use crate::{
    config::AppConfig,
    core::{AppState, ConnectionStatus},
    garmin::protocol::{
        ack_json, authentication_json, handshake_json, parse_incoming, GarminIncoming,
        ShotAssembler,
    },
};

const GARMIN_MAX_MESSAGE_BUFFER_BYTES: usize = 64 * 1024;

pub async fn spawn_listener(config: AppConfig, state: AppState) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind(config.garmin_addr())
        .await
        .map_err(|err| format!("failed to bind Garmin listener: {err}"))?;
    let addr = listener
        .local_addr()
        .map_err(|err| format!("failed to read Garmin listener address: {err}"))?;

    state
        .update_garmin(|garmin| {
            garmin.connection_status = ConnectionStatus::Listening;
            garmin.host = addr.ip().to_string();
            garmin.port = addr.port();
            garmin.active_client = None;
            garmin.last_error = None;
        })
        .await;

    let shot_counter = Arc::new(AtomicU64::new(1));
    tokio::spawn(accept_loop(listener, state, shot_counter));

    Ok(addr)
}

async fn accept_loop(listener: TcpListener, state: AppState, shot_counter: Arc<AtomicU64>) {
    loop {
        match listener.accept().await {
            Ok((stream, peer_addr)) => {
                let client = peer_addr.to_string();
                state
                    .update_garmin(|garmin| {
                        garmin.connection_status = ConnectionStatus::Connected;
                        garmin.active_client = Some(client.clone());
                        garmin.last_error = None;
                    })
                    .await;

                let client_state = state.clone();
                let client_counter = shot_counter.clone();
                tokio::spawn(async move {
                    handle_client(stream, client_state.clone(), client_counter).await;
                    client_state
                        .update_garmin(|garmin| {
                            garmin.connection_status = ConnectionStatus::Listening;
                            garmin.active_client = None;
                        })
                        .await;
                });
            }
            Err(err) => {
                state
                    .update_garmin(|garmin| {
                        garmin.connection_status = ConnectionStatus::Error;
                        garmin.last_error = Some(format!("failed to accept Garmin client: {err}"));
                    })
                    .await;
            }
        }
    }
}

async fn handle_client(mut stream: TcpStream, state: AppState, shot_counter: Arc<AtomicU64>) {
    let mut read_buf = [0_u8; 4096];
    let mut message_buf = String::new();
    let mut assembler = ShotAssembler::default();

    loop {
        let bytes_read = match stream.read(&mut read_buf).await {
            Ok(0) => break,
            Ok(bytes_read) => bytes_read,
            Err(err) => {
                record_error(&state, format!("failed to read Garmin client: {err}")).await;
                break;
            }
        };

        message_buf.push_str(&String::from_utf8_lossy(&read_buf[..bytes_read]));

        while let Some(message) = extract_json_object(&mut message_buf) {
            match message {
                Ok(message) => match parse_incoming(&message) {
                    Ok(incoming) => {
                        if let Err(err) = handle_message(
                            &mut stream,
                            &state,
                            &shot_counter,
                            &mut assembler,
                            incoming,
                        )
                        .await
                        {
                            record_error(&state, err).await;
                        }
                    }
                    Err(err) => record_malformed(&state, err).await,
                },
                Err(err) => record_malformed(&state, err).await,
            }
        }

        if message_buf.len() > GARMIN_MAX_MESSAGE_BUFFER_BYTES {
            record_malformed(
                &state,
                format!(
                    "Garmin message buffer too large: {} bytes exceeds {} byte limit",
                    message_buf.len(),
                    GARMIN_MAX_MESSAGE_BUFFER_BYTES
                ),
            )
            .await;
            break;
        }
    }
}

async fn handle_message(
    stream: &mut TcpStream,
    state: &AppState,
    shot_counter: &AtomicU64,
    assembler: &mut ShotAssembler,
    incoming: GarminIncoming,
) -> Result<(), String> {
    match incoming {
        GarminIncoming::Handshake => write_response(stream, handshake_json()).await,
        GarminIncoming::Challenge => write_response(stream, authentication_json()).await,
        GarminIncoming::SetClubType { .. }
        | GarminIncoming::SetBallData { .. }
        | GarminIncoming::SetClubData { .. } => {
            let subtype = subtype(&incoming);
            assembler.apply(incoming);
            write_response(stream, ack_json(subtype)).await
        }
        GarminIncoming::SendShot => {
            assembler.build_shot(0)?;
            let shot_number = shot_counter.fetch_add(1, Ordering::SeqCst);
            let shot = assembler.build_shot(shot_number)?;
            state.publish_shot(shot).await;
            assembler.clear_ball_data_after_publish();
            write_response(stream, ack_json("SendShot")).await
        }
        GarminIncoming::Disconnect | GarminIncoming::Pong | GarminIncoming::Unknown(_) => Ok(()),
    }
}

async fn write_response(stream: &mut TcpStream, response: String) -> Result<(), String> {
    stream
        .write_all(response.as_bytes())
        .await
        .map_err(|err| format!("failed to write Garmin response: {err}"))
}

async fn record_malformed(state: &AppState, error: String) {
    state
        .update_garmin(|garmin| {
            garmin.malformed_message_count += 1;
            garmin.last_error = Some(error);
        })
        .await;
}

async fn record_error(state: &AppState, error: String) {
    state
        .update_garmin(|garmin| {
            garmin.last_error = Some(error);
        })
        .await;
}

fn subtype(incoming: &GarminIncoming) -> &'static str {
    match incoming {
        GarminIncoming::SetClubType { .. } => "SetClubType",
        GarminIncoming::SetBallData { .. } => "SetBallData",
        GarminIncoming::SetClubData { .. } => "SetClubData",
        _ => unreachable!("subtype only applies to acknowledged Garmin data messages"),
    }
}

fn extract_json_object(buffer: &mut String) -> Option<Result<String, String>> {
    let leading_whitespace = buffer
        .char_indices()
        .find(|(_, ch)| !ch.is_whitespace())
        .map(|(idx, _)| idx)
        .unwrap_or(buffer.len());
    if leading_whitespace > 0 {
        buffer.drain(..leading_whitespace);
    }

    if buffer.is_empty() {
        return None;
    }

    if !buffer.starts_with('{') {
        let next_object = buffer.find('{').unwrap_or(buffer.len());
        let malformed = buffer.drain(..next_object).collect::<String>();
        return Some(Err(format!(
            "invalid Garmin JSON framing before object: {malformed}"
        )));
    }

    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in buffer.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let end = idx + ch.len_utf8();
                    return Some(Ok(buffer.drain(..end).collect()));
                }
            }
            _ => {}
        }
    }

    None
}
