# Garmin Golf Connector

Rust CLI/library bridge for Garmin Golf launch monitor data. The connector listens for Garmin Golf's E6 Connect / Play on PC TCP messages, normalizes shot data, and forwards shots to enabled simulator integrations.

## Run

```sh
cargo run --bin garmingolf-connector -- \
  --garmin-host 0.0.0.0 \
  --garmin-port 2483 \
  --api-host 127.0.0.1 \
  --api-port 5178
```

OpenAPI is available at:

```text
http://127.0.0.1:5178/swagger-ui
```

## GSPro

Enable GSPro forwarding:

```sh
cargo run --bin garmingolf-connector -- \
  --enable-gspro \
  --gspro-host 127.0.0.1 \
  --gspro-port 921
```

## Nova-Style WebSocket

Enable the WebSocket shot feed:

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

## Environment Variables

Every CLI option has a `GARMINGOLF_` environment equivalent, for example:

- `GARMINGOLF_API_PORT`
- `GARMINGOLF_GARMIN_PORT`
- `GARMINGOLF_ENABLE_GSPRO`
- `GARMINGOLF_ENABLE_NOVA_WS`

CLI flags override environment values.

## Garmin Golf Setup

Open Garmin Golf on the phone, choose E6 Connect / Play on PC mode, and set the PC address and port to the machine running this connector. The default Garmin listener port is `2483`.
