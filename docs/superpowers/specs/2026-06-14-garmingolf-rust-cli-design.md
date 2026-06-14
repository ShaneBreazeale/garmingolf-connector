# Garmin Golf Rust CLI/Library Design

## Goal

Turn this repository into a smaller Rust CLI/library interface for Garmin Golf launch monitor bridging. The new implementation should replace the current JavaScript/Electron runtime while preserving the useful behavior: listen for Garmin Golf/E6 TCP messages, keep the Garmin app connected, assemble shot data, and forward shots to simulator integrations.

The project will use the backend architecture style from `squaregolf-connector`, but it will not include a Tauri shell, static frontend, desktop packaging, or Electron-equivalent UI.

## Scope

In scope:

- A Rust library crate exposing Garmin protocol, normalized launch data, simulator clients, runtime state, and API server modules.
- A Rust CLI daemon that starts the Garmin listener, OpenAPI server, optional GSPro forwarding, and optional Nova-style WebSocket source.
- Selectable OpenAPI port through CLI flags and environment variables.
- Optional Nova-style WebSocket launch monitor feed.
- Contract tests for protocol parsing, command responses, API behavior, and outbound simulator payloads.

Out of scope:

- Tauri or Electron desktop UI.
- Release installers.
- Bluetooth access to Garmin hardware. The existing flow uses Garmin Golf's E6 TCP mode, and this design keeps that model.
- Reverse engineering Garmin account/cloud APIs.

## Architecture

The Rust project should follow the SquareGolf connector's separation of concerns, trimmed for CLI/library use:

- `src/core`: shared domain models, runtime status, normalized ball/club metrics, and shot events.
- `src/garmin`: Garmin Golf/E6 TCP protocol handling. This owns handshake/challenge responses, ping/pong, club type updates, ball data, club data, and `SendShot` assembly.
- `src/gspro`: GSPro Open API TCP client, connection lifecycle, and shot payload conversion.
- `src/api`: Axum/utoipa OpenAPI server for health, status, configuration, controls, and test-shot injection.
- `src/nova`: optional WebSocket server that broadcasts normalized shot events in a Nova-style launch monitor format.
- `src/bin/garmingolf-connector.rs`: CLI entrypoint that parses configuration, starts runtimes, and handles shutdown.

The library should own the behavior. The CLI should mostly translate flags/env into `AppConfig`, start runtimes, and wait for shutdown.

## Runtime Flow

The normal data flow is:

1. Garmin Golf mobile app enters E6 Connect / Play on PC mode.
2. The user points the app at the computer IP and configured Garmin TCP port, default `2483`.
3. `garmin::runtime` accepts the connection and responds to Garmin/E6 `Handshake`, `Challenge`, `Ping`, and disconnect messages.
4. Incoming `SetClubType`, `SetBallData`, and `SetClubData` messages update per-connection shot state.
5. `SendShot` converts the current shot state into a normalized `core::ShotEvent`.
6. Runtime publishes the shot to enabled sinks:
   - GSPro Open API TCP client when enabled.
   - Nova WebSocket subscribers when enabled.
   - API-visible in-memory status/history.

## CLI Configuration

The primary CLI should support:

```sh
garmingolf-connector \
  --garmin-host 0.0.0.0 \
  --garmin-port 2483 \
  --api-host 127.0.0.1 \
  --api-port 5178 \
  --enable-gspro \
  --gspro-host 127.0.0.1 \
  --gspro-port 921 \
  --enable-nova-ws \
  --nova-ws-host 127.0.0.1 \
  --nova-ws-port 8765
```

Environment variables should mirror the CLI names with a `GARMINGOLF_` prefix, for example `GARMINGOLF_API_PORT`. CLI arguments override environment values.

Defaults:

- Garmin listener host: `0.0.0.0`
- Garmin listener port: `2483`
- OpenAPI host: `127.0.0.1`
- OpenAPI port: `5178`
- GSPro: disabled unless `--enable-gspro` is set
- GSPro host: `127.0.0.1`
- GSPro port: `921`
- Nova WebSocket: disabled unless `--enable-nova-ws` is set
- Nova WebSocket host: `127.0.0.1`
- Nova WebSocket port: `8765`

## OpenAPI Surface

The API server should expose JSON endpoints and generated Swagger UI, matching the SquareGolf backend style.

Initial endpoints:

- `GET /health`: returns process health.
- `GET /status`: returns Garmin, GSPro, Nova WebSocket, last-shot, and configuration status.
- `GET /config`: returns current runtime configuration.
- `PATCH /config`: updates runtime configuration where safe without restart. Port and host changes may be accepted but reported as requiring restart unless runtime rebinding is implemented.
- `POST /garmin/listen`: starts the Garmin listener if stopped.
- `POST /garmin/disconnect`: disconnects the active Garmin client and stops listening.
- `POST /gspro/connect`: connects or reconnects the GSPro client.
- `POST /gspro/disconnect`: disconnects GSPro forwarding.
- `POST /shots/test`: injects a deterministic test shot through the same sink pipeline used by real Garmin shots.

The selectable OpenAPI port is a startup concern. If `--api-port 0` is provided, the server may bind an OS-selected port and print the final address to stdout.

## Garmin/E6 Protocol Compatibility

The Rust Garmin protocol module should be behavior-compatible with the existing JavaScript implementation:

- Respond to `Handshake` with E6 handshake metadata.
- Respond to `Challenge` with authentication success.
- Respond to `Ping` with `Pong`.
- Track `SetClubType`.
- Track `SetBallData`.
- Track `SetClubData`.
- On `SendShot`, emit one normalized shot event.
- On `Disconnect` or socket close, clear connection state and report disconnected status.

Incoming JSON should be parsed with typed structures where possible, while preserving unknown fields when useful for diagnostics.

## Shot Model

`core::ShotEvent` should normalize the fields needed by GSPro and Nova:

- Shot number.
- Device name, default `Garmin R10`.
- Units, default `Yards`.
- Club type.
- Ball speed.
- Launch angle.
- Side angle or horizontal launch.
- Back spin.
- Side spin or spin axis, based on available Garmin/E6 fields.
- Carry distance when present.
- Total distance when present.
- Club speed, club path, face angle, attack angle, and smash factor when present.
- Raw Garmin/E6 payload snapshots for diagnostics.

Missing optional metrics should remain optional. The connector should not invent measurements except in the deterministic test shot.

## GSPro Integration

`gspro` should implement the Open API TCP payload currently produced by `src/gsProConnect.js`:

- Connect to configured host/port.
- Reconnect after disconnect or connection refusal when enabled.
- Send shot payloads with device ID, units, API version, shot number, ball data, and optional club data.
- Track connected/disconnected/connecting status.

The library should expose conversion functions from `ShotEvent` to GSPro payloads so the contract can be tested without sockets.

## Nova WebSocket

When `--enable-nova-ws` is set, the connector should start a WebSocket server on the configured host/port.

Behavior:

- Accept multiple subscribers.
- Broadcast each normalized `ShotEvent` to all connected subscribers.
- Include a compact status/heartbeat message if the SquareGolf Nova-style protocol expects it.
- Keep Nova formatting isolated in `src/nova` so it can evolve without changing Garmin parsing or GSPro output.

The first implementation can support broadcast-only shot output plus a status endpoint in OpenAPI. Command/control messages from WebSocket clients are not required initially.

## Error Handling

Errors should be surfaced through typed runtime status rather than only logs:

- Garmin bind failure, active client disconnect, malformed JSON, and unsupported message type.
- GSPro connection refused, timeout, disconnect, and payload send failure.
- API bind failure.
- Nova WebSocket bind failure and subscriber send failures.

Malformed Garmin messages should not crash the runtime. They should be logged, counted, and exposed in status.

## Testing

The implementation should be driven by contract tests before production code:

- Config parsing: defaults, environment values, CLI override behavior, and selectable API port.
- Garmin protocol: handshake response, challenge response, ping response, typed parsing, shot assembly from ball/club data, and malformed JSON handling.
- GSPro payload conversion: normalized shot to expected Open API payload.
- API contract: health, status, config, and test-shot endpoint behavior.
- Nova WebSocket: a subscriber receives a test shot when enabled.
- Runtime TCP smoke test: local Garmin client sends E6-style messages and the runtime emits one shot event.

Tests should avoid real Garmin hardware and real GSPro. Socket tests should use local loopback ports and short timeouts.

## Migration Notes

The current JavaScript files are useful references during the Rust port:

- `src/garminConnect.js`: Garmin/E6 TCP connection lifecycle and message handling.
- `src/gsProConnect.js`: GSPro TCP lifecycle and outbound payload shape.
- `src/helpers/simMessages.js`: E6 handshake/challenge/ping and sample shot payloads.
- `src/env.js`: current defaults for device ID, units, GSPro host/port, and Garmin port.

The old Electron files can remain during the first Rust implementation if needed, but the new Rust crate should be independently buildable and testable.
