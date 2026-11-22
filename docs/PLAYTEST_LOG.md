# Browser playtest log

The browser client (`engine/web/`) records one session in bounded memory
(`SessionTelemetry`); nothing leaves the device until the operator downloads it.

## What it records
- Snapshot arrival gaps (20 Hz snapshots), applied-ack latency (input send time
  to arrival of the covering ack), and frame durations (~60 Hz draw loop).
- Discrete events `join`, `resume`, `reconnect`, `disconnect`, `level_change`,
  `death`, `rescue`, `complete`, each with `at` (ms since session start) and
  a small detail (`level`, `deaths`, `code`, `rescued_at`).

## What it does NOT record
No message contents, no chat text, no credentials, no resume tokens, no account
identity, nothing transmitted — page memory only until the download button.

## Memory bounds
`SAMPLE_CAP` = 4096 entries per sample buffer (`gaps`, `ackLatencies`,
`frames`), drop-oldest once full; `EVENT_CAP` = 512 events, same rule. That is
~3.4 min of snapshot gaps (4096 / 20 Hz), ~1 min of frame times (4096 /
60 Hz), and at most one ack sample per snapshot.

## How to export
Press **Download playtest log** to save `summary()` as JSON named
`coop-playtest-<ISO-8601 timestamp with ':' and '.' replaced by '-'>.json`:

- `started_at`, `ended_at` — `performance.now()` values (ms).
- `counts` — `snapshots`, `acks`, `frames`, `events` (entries currently held).
- `snapshot_gap`, `ack_latency`, `frame_time` — each `{ median, p95 }` in ms
  (nearest-rank p95; `null` when empty); `events` — a copy of the event array.

## Reading the numbers
- High `snapshot_gap` p95: the state path stalled (network/server) — remote
  players freeze or jump.
- High `ack_latency` p95: inputs acknowledged late (round trip plus server
  scheduling); input feels laggy even when frames are smooth.
- High `frame_time` p95: the browser dropped frames (rendering or main-thread
  load), independent of the network.
- A `rescue` event means a level allowed an unrecoverable split — report it
  as a level bug.

## Known limits
- Browser only; the native client has no equivalent log yet.
- Recent-window only: oldest samples are dropped on long sessions.
- A hidden tab suspends frame sampling (gaps over 1 s are discarded).
