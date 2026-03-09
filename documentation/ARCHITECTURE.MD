# ARCHITECTURE - redis-lite

A living document. Update when modules are added, interfaces change, or the design evolves.

---

## System Overview

redis-lite is a single-process, async TCP server. Clients connect over TCP, send commands encoded in the RESP protocol, and receive responses. All data lives in memory with hybrid persistence to disk (RDB snapshots + AOF logging). Graceful shutdown ensures data is flushed before exit.

```
┌─────────────────────────────────────────────┐
│                  Client                     │
│          (redis-cli / any RESP client)      │
└───────────────────┬─────────────────────────┘
                    │ TCP (port 6379)
                    ▼
┌─────────────────────────────────────────────┐
│              server/                        │
│   Tokio TCP listener + connection handler   │
│   One task spawned per client connection    │
│   Graceful shutdown via CancellationToken   │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│              protocol/                      │
│   RESP parser - bytes in, Command enum out  │
│   Encoder - RespValue out to RESP bytes     │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│              storage/                       │
│   DashMap-backed in-memory KV store         │
│   GET, SET, DEL, EXISTS                     │
│   TTL: lazy expiry + active background sweep│
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│             persistence/                    │
│   Hybrid: RDB snapshots + AOF logging       │
│   Replay on startup for crash recovery      │
│   Periodic snapshots with AOF truncation    │
└─────────────────────────────────────────────┘
```

---

## Modules

### `main.rs`
**Responsibility:** Bootstrap and orchestration. Handles the full lifecycle: load snapshot -> replay AOF -> initialize shared state -> spawn background tasks (expiry sweep, periodic snapshots) -> start TCP server -> graceful shutdown with final snapshot + AOF truncation.

**Key orchestration:**
- Hybrid startup: snapshot load (if exists) -> AOF replay on top (if exists) -> fresh AofWriter
- Background tasks: expiry sweep (3s interval), periodic snapshot (300s interval)
- Shutdown: `tokio::select!` races server against `ctrl_c`, then runs final snapshot + AOF truncation
- `CancellationToken` propagated to `server::run()` for clean accept-loop termination

**Status:** complete

---

### `server/`
**Responsibility:** Accept TCP connections. Spawn a Tokio task per client. Read RESP bytes, parse commands, execute against Store, log write commands to AOF, encode and write responses back. Supports graceful shutdown via CancellationToken.

**Key types:**
- `TcpStream`, `TcpListener` (Tokio)
- `bytes::{Buf, Bytes, BytesMut}`
- `Arc<Mutex<AofWriter>>`
- `CancellationToken` (tokio_util)

**Interface:**
- `pub async fn run(listener: TcpListener, store: Store, aof_writer: Arc<Mutex<AofWriter>>, shutdown: CancellationToken)`
    - Accept loop with `tokio::select!` - races `listener.accept()` against `shutdown.cancelled()`
    - Clones `Store` and `Arc<Mutex<AofWriter>>` per spawned client task
- `pub async fn handle_client(socket: TcpStream, store: Store, aof_writer: Arc<Mutex<AofWriter>>) -> Result<(), Box<dyn Error>>`
    - Reads into persistent `BytesMut` buffer, inner loop drains all complete RESP frames per read (supports pipelined commands)
    - Captures raw RESP bytes before buffer advance for AOF logging
    - Logs SET/DEL commands to AOF after successful execution - AOF failures are non-fatal (logged, not propagated)
- `pub fn execute_command(cmd: protocol::Command, store: &Store) -> RespValue`
    - Pure dispatch, no IO - maps Command variants to store operations and returns RESP values

**Status:** complete

---

### `protocol/`
**Responsibility:** Parse raw RESP bytes into typed `Command` variants. Serialize responses back to RESP bytes.

**Key types:**
- `Command` enum - `Ping`, `Get { key }`, `Set { key, value, ttl }`, `Del { key }`, `Exists { key }`
- `RespValue` enum - `SimpleString`, `Error`, `Integer`, `BulkString(Option<Bytes>)`, `Array`
- `ProtocolError` enum - `Incomplete`, `InvalidType(u8)`, `InvalidFormat(String)`, `InvalidCommand(String)`, `WrongArgCount { command, expected, got }`

**Interface:**
- `pub fn parse_value(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>`
    - Cursor-based with rollback-on-incomplete pattern - saves pos before parsing, restores on `Incomplete` so the server can retry when more bytes arrive
    - Recursive for nested arrays
- `pub fn parse_command(value: RespValue) -> Result<Command, ProtocolError>`
    - Converts RespValue (array of bulk strings) into typed Command enum
    - Supports Redis-style `EX`/`PX` flags for SET TTL
- `pub fn encode(value: &RespValue) -> Bytes`
    - Serializes RespValue back to RESP wire format

**Status:** complete - 54 unit tests passing

---

### `storage/`
**Responsibility:** Thread-safe in-memory key-value store. Core data structure. Owns TTL logic (both lazy and active expiry).

**Key types:**
- `Store` struct - wraps `Arc<DashMap<String, Entry>>`, derives Clone
- `Entry` struct - `{ value: Bytes, expires_at: Option<Instant> }`, derives Clone

**Interface:**
- `Store::new() -> Store`
- `store.get(key) -> Option<Bytes>` - lazy expiry: deletes expired keys on access via `remove_if`
- `store.set(key, value, ttl: Option<Duration>)` - converts Duration to Instant internally
- `store.del(key) -> bool`
- `store.exists(key) -> bool` - lazy expiry: same pattern as get
- `store.snapshot_entries() -> Vec<(String, Entry)>` - exposes DashMap contents for snapshot serialization without making the data field public
- `async expiry_sweep(&self, interval: Duration)` - background loop, two-phase collect-then-delete to avoid DashMap deadlocks

**Status:** complete

---

### `persistence/`
**Responsibility:** Durability. Hybrid persistence: RDB-style snapshots for periodic full-state dumps, AOF for write-ahead logging between snapshots. Replay on startup for crash recovery.

#### `persistence/snapshot.rs`

**Key types:**
- `SnapshotEntry` struct - `{ key: String, value: Vec<u8>, ttl_remaining: Option<Duration> }` - serde-serializable. Converts `Instant` to remaining `Duration` for serialization.
- `SnapshotError` enum - `Io(io::Error)`, `Bincode(Box<bincode::ErrorKind>)` - uses thiserror with `#[from]`

**Interface:**
- `pub fn collect_snapshot(store: &Store) -> Vec<SnapshotEntry>` - iterates DashMap, skips expired keys, converts Instant to remaining Duration. Fast (no disk IO).
- `pub fn save(entries: Vec<SnapshotEntry>, path: &Path) -> Result<(), SnapshotError>` - serializes with bincode, writes atomically (temp file + rename). Runs in `spawn_blocking`.
- `pub fn load(path: &Path) -> Result<Store, SnapshotError>` - reads file, deserializes, converts Duration back to Instant, populates and returns a new Store. Crashes on corruption (strict mode).

**Dependencies:** bincode 1.3.3, serde (with derive feature)

**Status:** complete

#### `persistence/aof.rs`

**Key types:**
- `AofWriter` struct - wraps `std::fs::File` opened in append+create mode. Shared across tasks via `Arc<Mutex<AofWriter>>`.

**Interface:**
- `AofWriter::new(path: &Path) -> Result<Self, io::Error>` - opens/creates AOF file in append mode via OpenOptions
- `pub fn append(&mut self, bytes: &[u8]) -> Result<(), io::Error>` - writes raw RESP bytes to the file
- `pub fn replay(path: &Path, store: &Store) -> Result<(), io::Error>` - reads entire AOF file, parses RESP frames with same cursor pattern as handle_client, replays write commands via `execute_command`. Tolerates partial frames at EOF (crash mid-write). Hard parse errors stop replay with a warning.
- `pub fn truncate(&mut self) -> Result<(), io::Error>` - wipes file via `set_len(0)` + `seek(0)` on existing handle. Called after successful snapshot.

**Status:** complete

---

## Data Flow - Example: `SET foo bar`

```
1. Client sends: "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
2. server/ receives raw bytes from TCP stream into BytesMut buffer
3. server/ captures raw RESP bytes before advancing buffer (for AOF)
4. protocol/ parses -> Command::Set { key: "foo", value: "bar", ttl: None }
5. storage/ executes -> inserts into DashMap
6. server/ checks: is_write? Yes -> aof_writer.append(raw RESP bytes) (non-fatal on failure)
7. protocol/ encodes -> "+OK\r\n"
8. server/ writes response back to TCP stream
9. Client receives: "OK"
```

## Startup Flow

```
1. Check if dump.rdb exists
   - Yes -> snapshot::load(path) -> populated Store (crash on corruption)
   - No  -> Store::new()
2. Check if appendonly.aof exists -> replay(path, store) on top of snapshot
3. Create AofWriter (opens/creates appendonly.aof in append mode)
4. Spawn background expiry sweep task (every 3s)
5. Spawn periodic snapshot save task (every 300s)
   - collect_snapshot on async thread (fast)
   - save() via spawn_blocking (disk IO off async pool)
   - On success: truncate AOF
6. Start TCP server with CancellationToken (run listener + client handling)
```

## Shutdown Flow

```
1. Ctrl+C received -> CancellationToken cancelled
2. server::run() breaks accept loop (in-flight clients finish naturally)
3. Final snapshot: collect_snapshot -> save via spawn_blocking
4. On success: truncate AOF
5. Process exits
```

---

## Concurrency Model

- One Tokio task per client connection
- `Store` is wrapped in `Arc<DashMap>` internally, cloned across tasks (Arc refcount bump, not deep copy)
- `DashMap` handles internal locking per key-shard - no global lock
- `AofWriter` is shared via `Arc<Mutex<AofWriter>>` - `std::sync::Mutex` (not tokio) since lock is held only for synchronous file writes, no `.await` while holding. Lock failures are non-fatal.
- TTL expiration runs as a background Tokio task (lazy expiry on GET/EXISTS + active sweep every 3s)
- Graceful shutdown via `CancellationToken` - one signal, many listeners

---

## Benchmarks

Tested with `redis-benchmark` against the running server.

**50 clients, 3-byte payload:**
- SET: ~86k ops/sec, p50 0.33ms, p99 0.44ms
- GET: ~106k ops/sec, p50 0.24ms, p99 0.42ms
- 99.9%+ of requests under 1ms

**200 clients, 1KB payload:**
- SET: ~81k ops/sec, p50 1.28ms, p99 1.63ms
- GET: ~96k ops/sec, p50 1.03ms, p99 1.67ms
- Throughput held within 10% of 50-client run despite 4x concurrency and 340x payload

**200 clients, 1KB payload, 16-pipeline:**
- SET: ~76k ops/sec, p50 0.38ms, p99 27ms
- GET: ~78k ops/sec, p50 0.26ms, p99 6.97ms
- Tail latency spike on SET caused by AOF mutex contention under burst pipelining

**Known bottleneck:** AOF mutex contention under heavy pipelined writes. The single `Arc<Mutex<AofWriter>>` becomes a serialization point when many clients burst write commands simultaneously. p50 stays fast, but p99 suffers. Mitigation path: batch AOF writes or move to a dedicated writer task with a channel.
