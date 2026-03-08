# ARCHITECTURE - redis-lite

A living document. Update when modules are added, interfaces change, or the design evolves.

---

## System Overview

redis-lite is a single-process, async TCP server. Clients connect over TCP, send commands encoded in the RESP protocol, and receive responses. All data lives in memory with optional persistence to disk.

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
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│              protocol/                      │
│   RESP parser - bytes in, Command enum out  │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│              storage/                       │
│   DashMap-backed in-memory KV store         │
│   Handles GET, SET, DEL, TTL                │
└───────────────────┬─────────────────────────┘
                    │
                    ▼
┌─────────────────────────────────────────────┐
│             persistence/                    │
│   AOF or snapshot - writes to disk          │
│   Replays on startup for recovery           │
└─────────────────────────────────────────────┘
```

---

## Modules

### `main.rs`
**Responsibility:** Bootstrap. Load snapshot if `dump.rdb` exists, initialize `Store`, create `AofWriter` wrapped in `Arc<Mutex>`, spawn background expiry sweep task, start the TCP server.

**Status:** implemented

---

### `server/`
**Responsibility:** Accept TCP connections. Spawn a Tokio task per client. Read RESP bytes, parse commands, execute against Store, log write commands to AOF, encode and write responses back.

**Key types:**
- Tokio::TcpStream
- Tokio::TcpListener
- bytes::{Buf, Bytes, BytesMut}
- Arc<Mutex<AofWriter>>

**Interface:**
- `pub async fn handle_client(socket: TcpStream, store: Store, aof_writer: Arc<Mutex<AofWriter>>) -> Result<(), Box<dyn Error>>`
    - Reads into persistent `BytesMut` buffer, inner loop drains all complete RESP frames per read
    - Captures raw RESP bytes before buffer advance for AOF logging
    - Logs SET/DEL commands to AOF after successful execution via `matches!()` check
    - helper-function for the above:
        - `fn execute_command(cmd: protocol::Command, store: &Store) -> RespValue` (pure dispatch, no IO)
- `pub async fn run(listener: TcpListener, store: Store, aof_writer: Arc<Mutex<AofWriter>>)`
    - Clones `Store` and `Arc<Mutex<AofWriter>>` per spawned client task

**Status:** implemented — tested with redis-cli (interactive and one-shot)

---

### `protocol/`
**Responsibility:** Parse raw RESP bytes into typed `Command` variants. Serialize responses back to RESP bytes.

**Key types:**
- `Command` enum -      `Get(key)`, `Set(key, value)`, `Del(key)`, etc.
- `RespValue` enum -    `SimpleString`, `Error`, `Integer`, `BulkString`, `Array`
- `ProtocolError`enum - `Incomplete`, `InvalidType(u8)`, `InvalidFormat(String)`, `InvalidCommand(String)`, `WrongArgCount { command: String, expected: usize, got: usize }`

**Interface:**
- `fn parse_value(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>`
    - cursor + rollback pattern    
- `fn parse_command(value: RespValue) -> Result<Command, ProtocolError>`
- `encode(value: RespValue) -> Bytes`

**Status:** finished

---

### `storage/`
**Responsibility:** Thread-safe in-memory key-value store. Core data structure. Owns TTL logic.

**Key types:**
- `Store` struct - wraps `Arc<DashMap<String, Entry>>`, derives Clone
- `Entry` struct - `{ value: Bytes, expires_at: Option<Instant> }`, derives Clone

**Interface:**
- `Store::new() -> Store`
- `store.get(key) -> Option<Bytes>`
- `store.set(key, value, ttl: Option<Duration>)`
- `store.del(key) -> bool`
- `store.exists(key) -> bool`
- `store.snapshot_entries() -> Vec<(String, Entry)>` — exposes DashMap contents for snapshot serialization without making data field public
- `async expiry_sweep(&self, interval: Duration)` — background loop, two-phase collect-then-delete to avoid DashMap deadlocks

**Status:** implemented

---

### `persistence/`
**Responsibility:** Durability. Hybrid persistence: RDB-style snapshots for periodic full-state dumps, AOF for write-ahead logging between snapshots. Replay on startup for recovery.

#### `persistence/snapshot.rs`

**Key types:**
- `SnapshotEntry` struct - `{ key: String, value: Vec<u8>, ttl_remaining: Option<Duration> }` — serde-serializable representation of one entry. Converts `Instant` to remaining `Duration` for serialization.
- `SnapshotError` enum - `Io(io::Error)`, `Bincode(Box<bincode::ErrorKind>)` — uses thiserror with `#[from]`

**Interface:**
- `pub fn collect_snapshot(store: &Store) -> Vec<SnapshotEntry>` — iterates DashMap, skips expired keys, converts Instant to remaining Duration. Fast (no disk IO).
- `pub fn save(entries: Vec<SnapshotEntry>, path: &Path) -> Result<(), SnapshotError>` — serializes with bincode, writes atomically (temp file + rename). Designed to run in a background task.
- `pub fn load(path: &Path) -> Result<Store, SnapshotError>` — reads file, deserializes, converts Duration back to Instant, populates and returns a new Store.

**Dependencies:** bincode 1.3.3, serde (with derive feature)

**Status:** implemented — save/load tested

#### `persistence/aof.rs`

**Key types:**
- `AofWriter` struct - wraps `std::fs::File` opened in append+create mode. Shared across tasks via `Arc<Mutex<AofWriter>>`.

**Interface:**
- `AofWriter::new(path: &Path) -> Result<Self, io::Error>` — opens/creates AOF file in append mode via OpenOptions
- `pub fn append(&mut self, bytes: &[u8]) -> Result<(), io::Error>` — writes raw RESP bytes to the file
- `pub fn replay(path: &Path, store: &Store) -> Result<(), io::Error>` — 🔲 TODO: reads AOF file, parses RESP frames, replays write commands against store
- `pub fn truncate(&mut self) -> Result<(), io::Error>` — wipes file via `set_len(0)` + `seek(0)` on existing handle. Called after successful snapshot.
**Status:** ⏳ implemented

---

## Data Flow - Example: `SET foo bar`

```
1. Client sends: "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
2. server/ receives raw bytes from TCP stream into BytesMut buffer
3. server/ captures raw RESP bytes before advancing buffer (for AOF)
4. protocol/ parses → Command::Set("foo", "bar", None)
5. storage/ executes → inserts into DashMap
6. server/ checks: is_write? Yes → aof_writer.append(raw RESP bytes)
7. protocol/ encodes → "+OK\r\n"
8. server/ writes response back to TCP stream
9. Client receives: "OK"
```

## Startup Flow
```
1. Check if dump.rdb exists
   - Yes → snapshot::load(path) → populated Store (crash on corruption)
   - No  → Store::new()
2. Check if appendonly.aof exists → replay(path, store) on top of snapshot
3. Create AofWriter (opens/creates appendonly.aof in append mode)
4. Spawn background expiry sweep task (every 3s)
5. Spawn periodic snapshot save task
   - collect_snapshot on async thread (fast)
   - save() via spawn_blocking (disk IO off async pool)
   - On success: truncate AOF
6. Start TCP server (run listener + client handling)
```

---

## Concurrency Model

- One Tokio task per client connection
- `Store` is wrapped in `Arc<DashMap>` internally, cloned across tasks (Arc refcount bump, not deep copy)
- `DashMap` handles internal locking per key-shard — no global lock
- `AofWriter` is shared via `Arc<Mutex<AofWriter>>` — `std::sync::Mutex` (not tokio) since lock is held only for synchronous file writes, no `.await` while holding
- TTL expiration runs as a background Tokio task (lazy expiry on GET/EXISTS + active sweep every 3s)

---

## What This Doc Doesn't Cover Yet
- Config / CLI args
- Error propagation strategy
- Benchmarking setup

