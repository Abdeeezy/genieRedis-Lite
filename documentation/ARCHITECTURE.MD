# ARCHITECTURE — redis-lite

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
│   RESP parser — bytes in, Command enum out  │
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
│   AOF or snapshot — writes to disk          │
│   Replays on startup for recovery           │
└─────────────────────────────────────────────┘
```

---

## Modules

### `main.rs`
**Responsibility:** Bootstrap. Initialize tracing, load config, start the server and run the server.

**Status:** testing stages 

---

### `server/`
**Responsibility:** Accept TCP connections. Spawn a Tokio task per client. Pass raw bytes to the protocol layer, return encoded responses.

**Key types:**
- _TBD_

**Interface:**
- pub async fn handle_client(socket: TcpStream, store: Store)
- pub async fn run(listener: TcpListener, store: Store)

**Status:** skeleton-status but it's working. connections are handled, simple logs..

---

### `protocol/`
**Responsibility:** Parse raw RESP bytes into typed `Command` variants. Serialize responses back to RESP bytes.

**Key types:**
- `Command` enum — `Get(key)`, `Set(key, value)`, `Del(key)`, etc.
- `RespValue` enum — `SimpleString`, `Error`, `Integer`, `BulkString`, `Array`

**Interface:**
- `parse(buf: &[u8]) -> Result<Command, ProtocolError>`
- `encode(value: RespValue) -> Bytes`

**Status:** ⬜ Not started

---

### `storage/`
**Responsibility:** Thread-safe in-memory key-value store. Core data structure. Owns TTL logic.

**Key types:**
- `Store` struct — wraps `DashMap<String, Entry>`
- `Entry` struct — `{ value: String, expires_at: Option<Instant> }`

**Interface:**
- `Store::new() -> Store`
- `store.get(key) -> Option<String>`
- `store.set(key, value, ttl: Option<Duration>)`
- `store.del(key) -> bool`
- `store.exists(key) -> bool`

**Status:** ⬜ Not started

---

### `persistence/`
**Responsibility:** Durability. Write commands to an AOF (Append-Only File) or periodically snapshot the store to disk. Replay on startup.

**Key types:**
- _TBD_

**Interface:**
- _TBD_

**Status:** ⬜ Not started

---

## Data Flow — Example: `SET foo bar`

```
1. Client sends: "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n"
2. server/ receives raw bytes from TCP stream
3. protocol/ parses → Command::Set("foo", "bar", None)
4. storage/ executes → inserts into DashMap
5. persistence/ appends command to AOF (if enabled)
6. protocol/ encodes → "+OK\r\n"
7. server/ writes response back to TCP stream
8. Client receives: "OK"
```

---

## Concurrency Model

- One Tokio task per client connection
- `Store` is wrapped in `Arc<Store>` and cloned across tasks
- `DashMap` handles internal locking per key-shard — no global lock
- TTL expiration runs as a background Tokio task (lazy expiry on GET + active sweep)

---

## What This Doc Doesn't Cover Yet
- Config / CLI args
- Error propagation strategy
- Benchmarking setup
- Graceful shutdown