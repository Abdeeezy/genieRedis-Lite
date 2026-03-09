# genieRedis-lite

A lightweight, high-performance key-value store built from scratch in Rust. A simplified Redis clone with RESP protocol support, concurrent in-memory storage, key expiration, hybrid persistence, and graceful shutdown.

This project was my introduction and practice for the Rust programming-language; There will be comments that I wrote alongside some code that might seem needlessly explanatory in the eyes of someone who's more adept at Rust. 

> **Status:** ✅ Complete - all build phases finished

---

## What It Does

redis-lite is a from-scratch implementation of a Redis-like server. It speaks a compatible subset of the [RESP protocol](https://redis.io/docs/reference/protocol-spec/), meaning standard Redis clients (`redis-cli`, etc.) can connect to it out of the box.

**Supported commands:**
| Command | Description |
|---------|-------------|
| `PING` | Health check - returns PONG |
| `SET key value [EX seconds \| PX milliseconds]` | Store a value with optional TTL |
| `GET key` | Retrieve a value |
| `DEL key` | Delete a key |
| `EXISTS key` | Check if a key exists |

---

## Performance

Benchmarked with `redis-benchmark` on localhost:

| Scenario | SET ops/sec | GET ops/sec | p99 latency |
|----------|-------------|-------------|-------------|
| 50 clients, 3B payload | ~86k | ~106k | <0.5ms |
| 200 clients, 1KB payload | ~81k | ~96k | <1.7ms |
| 200 clients, 1KB, 16-pipeline | ~76k | ~78k | SET ~27ms* |

\*Tail latency under pipelined writes is caused by AOF mutex contention - see [Architecture](./ARCHITECTURE.md#benchmarks) for details.

---

## Architecture

```
redis-lite/
├── src/
│   ├── main.rs           # Bootstrap, background tasks, shutdown
│   ├── server/           # TCP listener, connection handling (Tokio)
│   ├── protocol/         # RESP parser + encoder
│   ├── storage/          # Concurrent in-memory store (DashMap)
│   └── persistence/      # Hybrid RDB snapshots + AOF logging
```

See [`ARCHITECTURE.md`](./ARCHITECTURE.md) for a deeper breakdown of each module, data flows, concurrency model, and benchmarks.

---

## Tech Stack

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime, TCP networking |
| `tokio_util` | CancellationToken for graceful shutdown |
| `dashmap` | Lock-free concurrent HashMap |
| `bytes` | Efficient byte buffer handling |
| `serde` + `bincode` | Binary serialization for RDB snapshots |
| `thiserror` | Typed error enums |

---

## Getting Started

### Prerequisites
- Rust 1.75+ (`rustup update stable`)
- Cargo (comes with Rust)

### Build & Run

```bash
git clone https://github.com/Abdeeezy/genieRedis-Lite.git
cd genieRedis-Lite

cargo build --release
cargo run --release
```

The server starts on `127.0.0.1:6379` by default.

### Connect with redis-cli

```bash
redis-cli -p 6379
> PING
PONG
> SET hello world
OK
> GET hello
"world"
> SET temp data EX 10
OK
> GET temp
"data"
# ... wait 10 seconds ...
> GET temp
(nil)
```

### Run Tests

```bash
cargo test
```

### Run Benchmarks

```bash
# Install redis-benchmark (Ubuntu)
sudo apt install redis-tools

# Basic throughput
redis-benchmark -p 6379 -t set,get -n 100000 -c 50

# Stress test - high concurrency, larger payloads
redis-benchmark -p 6379 -t set,get -n 100000 -c 200 -d 1024

# Pipeline test
redis-benchmark -p 6379 -t set,get -n 100000 -c 200 -d 1024 -P 16
```

---

## Key Features

- **RESP protocol** - compatible with `redis-cli` and any standard Redis client
- **Concurrent storage** - DashMap with per-shard locking, no global lock bottleneck
- **Key expiration** - lazy expiry on access + active background sweep every 3 seconds
- **Hybrid persistence** - RDB-style snapshots (every 5 minutes) + AOF write-ahead logging between snapshots. Replay on startup for crash recovery.
- **Graceful shutdown** - Ctrl+C triggers final snapshot save and AOF flush before exit
- **Pipelined commands** - inner parse loop handles batched commands in a single TCP read

---

## Build Roadmap

- [x] Phase 1 - In-memory store (GET/SET/DEL with DashMap)
- [x] Phase 2 - TCP server (Tokio async listener)
- [x] Phase 3 - RESP protocol parser
- [x] Phase 4 - Wire protocol -> storage layer
- [x] Phase 5 - TTL / key expiration
- [x] Phase 6 - Persistence (hybrid RDB snapshots + AOF)
- [x] Phase 7 - Benchmarking & hardening

---

## Possible Extensions

Things that could be added but are outside the original scope:

- **More commands** - MGET/MSET, INCR/DECR, APPEND, KEYS, FLUSHALL, TTL, PERSIST, RENAME
- **AUTH** - password-based client authentication
- **Pub/Sub** - SUBSCRIBE/PUBLISH with per-channel broadcast using Tokio channels
- **Data structures** - Lists (LPUSH/RPUSH/LPOP), Sets (SADD/SMEMBERS), Hashes (HSET/HGET)
- **AOF write batching** - collect writes and flush periodically to reduce mutex contention under pipelining (addresses the known tail latency bottleneck)
- **Dedicated AOF writer task** - move AOF to a background task fed by an mpsc channel, eliminating mutex contention entirely
- **Config file / CLI args** - port, snapshot interval, AOF fsync policy, max memory
- **Memory limits** - max memory with eviction policies (LRU, LFU, random)


---

## Project Docs

| File | Purpose |
|------|---------|
| [`ARCHITECTURE.md`](./ARCHITECTURE.md) | System design, module map, data flows, concurrency model, benchmarks |
| [`DECISIONS.md`](./DECISIONS.md) | Technical decisions and tradeoffs |
| [`DEVLOG.md`](./DEVLOG.md) | Session-by-session progress journal |

---

## Why Build This?

This project exists to develop a deeper understanding of:
- Systems programming in Rust
- Async networking with Tokio
- Concurrent data structures and memory management
- Protocol design and parsing (RESP)
- Persistence strategies (AOF + snapshots)
- Error handling and graceful shutdown in production-style servers

---

## License

MIT
