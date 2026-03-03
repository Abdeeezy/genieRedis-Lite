# redis-lite

A lightweight, high-performance key-value store built in Rust. A simplified Redis clone with persistence, TCP networking, concurrent access handling, and key expiration.

> **Status:** 🚧 In active development — Phase 1 (In-memory storage)

---

## What It Does

redis-lite is a from-scratch implementation of a Redis-like server. It speaks a compatible subset of the [RESP protocol](https://redis.io/docs/reference/protocol-spec/), meaning standard Redis clients can connect to it.

**Supported commands (planned):**
| Command | Description |
|---------|-------------|
| `SET key value` | Store a value |
| `GET key` | Retrieve a value |
| `DEL key` | Delete a key |
| `EXISTS key` | Check if a key exists |
| `EXPIRE key seconds` | Set a TTL on a key |
| `TTL key` | Check remaining TTL |
| `FLUSH` | Clear all keys |

---

## Architecture

```
redis-lite/
├── src/
│   ├── main.rs           # Entry point, server bootstrap
│   ├── server/           # TCP listener, connection handling (Tokio)
│   ├── storage/          # Concurrent in-memory store (DashMap)
│   ├── protocol/         # RESP protocol parser
│   └── persistence/      # AOF / snapshot to disk
└── tests/
    └── integration_test.rs
```

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for a deeper breakdown of each module and how they connect.

---

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime, TCP networking |
| `dashmap` | Lock-free concurrent HashMap |
| `bytes` | Efficient byte buffer handling |
| `serde` + `serde_json` | Serialization for persistence |
| `thiserror` | Typed error handling |
| `tracing` | Structured logging |

---

## Getting Started

### Prerequisites
- Rust 1.75+ (`rustup update stable`)
- Cargo (comes with Rust)

### Build & Run

```bash
git clone https://github.com/Abdeeezy/genieRedis-Lite.git
cd genieRedis-Lite

cargo build
cargo run
```

The server starts on `127.0.0.1:6379` by default.

### Connect with redis-cli

```bash
redis-cli -p 6379
> SET hello world
OK
> GET hello
"world"
```

### Run Tests

```bash
cargo build        
cargo test         
cargo clippy       # Linter
cargo fmt          # Formatter
```


---

## Build Roadmap

- [x] Project scaffold & dependencies
- [ ] Phase 1 — In-memory store (GET/SET/DEL with DashMap)
- [ ] Phase 2 — TCP server (Tokio async listener)
- [ ] Phase 3 — RESP protocol parser
- [ ] Phase 4 — Wire protocol → storage layer
- [ ] Phase 5 — TTL / key expiration
- [ ] Phase 6 — Persistence (AOF or snapshot)
- [ ] Phase 7 — Benchmarking & hardening

---

## Project Docs

| File | Purpose |
|------|---------|
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | System design, module map, interfaces |
| [`DECISIONS.md`](./DECISIONS.md) | Technical decisions and tradeoffs |
| [`DEVLOG.md`](./DEVLOG.md) | Session-by-session progress journal |

---

## Why Build This?

This project exists to develop a deeper understanding of:
- Systems programming in Rust
- Async networking with Tokio
- Concurrent data structures and memory management
- Protocol design and parsing
- Persistence strategies

---

## License

MIT
