# Garmin Golf Connector

A small Rust CLI and library for bridging Garmin Golf launch monitor data into local simulator tooling.

The connector listens for Garmin Golf's E6 Connect / Play on PC TCP stream, normalizes each shot, exposes a local OpenAPI server, and can forward shots to GSPro or a Nova-style WebSocket feed.

## Features

- Garmin Golf TCP listener for E6 Connect / Play on PC messages
- Local OpenAPI server with status, config, test-shot, Swagger UI, and OpenAPI JSON endpoints
- Optional GSPro OpenAPI TCP forwarding
- Optional Nova-style WebSocket shot stream
- Selectable host and port for every listener/output
- Reusable Rust library modules for embedding the connector in another app

## Requirements

- Rust 1.77 or newer
- A Garmin Golf launch monitor mode that can send E6 Connect / Play on PC data
- Optional: GSPro running with its OpenAPI connector enabled

## Quickstart

Run the connector with the default Garmin and API ports:

```sh
cargo run --bin garmingolf-connector
```

The default listeners are:

```text
Garmin TCP: 0.0.0.0:2483
OpenAPI:    http://127.0.0.1:5178
Swagger UI: http://127.0.0.1:5178/swagger-ui
```

Open Garmin Golf on your phone, choose E6 Connect / Play on PC mode, then set the PC address to the machine running this connector and the port to `2483`.

## Common Commands

Run on a custom Garmin port:

```sh
cargo run --bin garmingolf-connector -- \
  --garmin-host 0.0.0.0 \
  --garmin-port 2483
```

Run the API server on a custom port:

```sh
cargo run --bin garmingolf-connector -- \
  --api-host 127.0.0.1 \
  --api-port 5178
```

Ask the OS for any free API port:

```sh
cargo run --bin garmingolf-connector -- --api-port 0
```

The CLI prints the actual bound URL at startup.

## GSPro Forwarding

Enable GSPro forwarding when GSPro is listening for OpenAPI shot JSON:

```sh
cargo run --bin garmingolf-connector -- \
  --enable-gspro \
  --gspro-host 127.0.0.1 \
  --gspro-port 921
```

Forwarded GSPro payloads are newline-delimited JSON and include ball data plus optional club data when Garmin provides it.

## Nova-Style WebSocket

Enable the WebSocket feed:

```sh
cargo run --bin garmingolf-connector -- \
  --enable-nova-ws \
  --nova-ws-host 127.0.0.1 \
  --nova-ws-port 8765
```

Subscribers connect to:

```text
ws://127.0.0.1:8765/ws
```

Shot messages use this shape:

```json
{
  "type": "shot",
  "shot_number": 1,
  "ball_speed_miles_per_hour": 98.5,
  "vertical_launch_angle_degrees": 13.5,
  "horizontal_launch_angle_degrees": 0.0,
  "total_spin_rpm": 2350.2,
  "spin_axis_degrees": -10.2
}
```

## OpenAPI Server

The API server is local by default and is intended for inspection, automation, and lightweight control.

| Endpoint | Method | Purpose |
| --- | --- | --- |
| `/health` | `GET` | Health check |
| `/status` | `GET` | Current connector, Garmin, GSPro, Nova, and last-shot status |
| `/config` | `GET` | Effective runtime configuration |
| `/config` | `PATCH` | Placeholder configuration update endpoint |
| `/shots/test` | `POST` | Publish a synthetic shot for testing outputs |
| `/api-docs/openapi.json` | `GET` | OpenAPI document |
| `/swagger-ui` | `GET` | Swagger UI |

Try a synthetic shot:

```sh
curl -X POST http://127.0.0.1:5178/shots/test
```

Check connector status:

```sh
curl http://127.0.0.1:5178/status
```

## Configuration

CLI flags override environment variables. Environment variables override built-in defaults.

| CLI flag | Environment variable | Default |
| --- | --- | --- |
| `--garmin-host` | `GARMINGOLF_GARMIN_HOST` | `0.0.0.0` |
| `--garmin-port` | `GARMINGOLF_GARMIN_PORT` | `2483` |
| `--api-host` | `GARMINGOLF_API_HOST` | `127.0.0.1` |
| `--api-port` | `GARMINGOLF_API_PORT` | `5178` |
| `--enable-gspro` | `GARMINGOLF_ENABLE_GSPRO` | `false` |
| `--gspro-host` | `GARMINGOLF_GSPRO_HOST` | `127.0.0.1` |
| `--gspro-port` | `GARMINGOLF_GSPRO_PORT` | `921` |
| `--enable-nova-ws` | `GARMINGOLF_ENABLE_NOVA_WS` | `false` |
| `--nova-ws-host` | `GARMINGOLF_NOVA_WS_HOST` | `127.0.0.1` |
| `--nova-ws-port` | `GARMINGOLF_NOVA_WS_PORT` | `8765` |

Boolean flags can be passed without a value or with an explicit value:

```sh
cargo run --bin garmingolf-connector -- --enable-gspro
cargo run --bin garmingolf-connector -- --enable-gspro=false
```

## Library Use

The package exposes the `garmingolf_connector` library for embedding the same pieces in another Rust application.

Public modules:

- `api`: Axum router and OpenAPI server
- `config`: CLI/env configuration model
- `core`: shared status and shot event state
- `garmin`: Garmin protocol parsing and TCP runtime
- `gspro`: GSPro payload conversion and forwarding runtime
- `nova`: WebSocket shot stream

## Development

Run the test suite:

```sh
cargo test
```

Check the CLI binary:

```sh
cargo check --bin garmingolf-connector
```

Check formatting:

```sh
cargo fmt --check
```

## Notes

- Garmin status is modeled around the expected single Garmin app client. Multiple simultaneous Garmin clients can connect, but the top-level connection status is not a per-client view.
- GSPro and Nova outputs are optional. If neither is enabled, the connector still exposes OpenAPI status and test-shot support.

## License

MIT
