# DEVLOG - redis-lite

A chronological journal of sessions, decisions, mistakes, and progress.
Updated at the end of each working session.

---

## How I Will Use This File
- Adding a new entry each session (even short ones)
- Being honest about what broke and why 
- Link to commits where relevant

---

## Session Log

---

### Session 001 - [2026-03-02 ~4AM]
**Phase:** Project Setup
**Commit(s):** `initial scaffold`

#### What We Did
- Initialized Cargo project (`cargo new redis-lite`)
- Set up directory structure: `server/`, `storage/`, `protocol/`, `persistence/`
- Created README, DEVLOG, ARCHITECTURE, DECISIONS docs
- Pushed to GitHub

#### Decisions Made
- Nothing major

#### Mistakes / Learnings
- 

#### Open Questions
- 

#### Next Session
- Start Phase 1: implement the in-memory store module
- Write `storage/mod.rs` with GET, SET, DEL

---

### Session 002 - [2026-03-02]
**Phase:** 1
**Commit(s):** `set up dashmap data-storage and defined key/value data-type; also implemented a static method to create the store, and the SET/GET/DEL instance-methods.`

#### What We Did
- Designed and implemented storage/mod.rs - the core in-memory store
- Defined Entry struct: Bytes value
- Defined Store struct: wraps Arc<DashMap<String, Entry>>, derives Clone
- Implemented new(), get(), set(), del()
- Wired storage module into main.rs via mod storage
- Wrote 5 unit tests inline in the module: create/read, get nonexistent, del existent, del nonexistent, clone-shares-state concurrency

#### Decisions Made
- String keys, Bytes values - Redis keys are strings; Bytes for values because Redis is binary-safe. Locking this in early avoids painful type changes later.
- Arc inside Store, not outside - encapsulation decision. Callers clone Store freely without knowing or caring about the Arc. Cloning Store increments the refcount, not deep-copies the map.
- Unit tests inline the storage-module, not in 'tests/' integration-tests are for testing public API across layers. That's a later-phase concern.


#### Mistakes / Learnings
- Learned how to set up and use Rust's testing system
- mod storage must be declared in main.rs or the module is invisible to the compiler
- return on last expressions is unnecessary in Rust - idiomatic to drop it
- Threads need owned data - move closures + cloned Store for the concurrency test
- Two Store::new() calls are two separate stores, not shared state - clone one instance instead

#### Open Questions
- How stable is this design long-term?

#### Next Session
- Set up the TCP server with Tokio - listener, connection loop, one task per client

---


### Session 003 - [2026-03-03]
**Phase:** Phase 2 - TCP Server
**Commit(s):** 
- `session 03 - made store-methods public; implemented exists(key) method.`
- `session 03 - implemented server/mod.rs; run() and handle_client() with simple logging; wired up main.rs and tested successfully.`

#### What We Did
- Implemented `server/mod.rs` with `run()` and `handle_client()`
- `run()` loops on `listener.accept()`, spawns a Tokio task per connection
- `handle_client()` currently logs the client address - primed for Phase 3
- Wired up `main.rs`: initializes `Store`, binds `TcpListener`, calls `server::run()`
- Smoke tested with a browser - multiple concurrent connections accepted correctly

#### Decisions Made
- No new entries in DECISIONS.md - no major tradeoffs this phase

#### Mistakes / Learnings
- Initially wrapped `TcpListener` and `TcpStream` in custom structs - unnecessary,
  Tokio's types are sufficient directly
- Hit a "use of moved value" compile error: `store` was being moved into the 
  `async move` block on the first iteration, consumed before the second. 
  Fix: clone `store` at the top of each loop iteration before the spawn. 
  Cloning `Store` is cheap - it only bumps the `Arc` refcount, the underlying 
  `DashMap` is shared across all clones.

#### Open Questions
- None

#### Next Session
- Phase 3: RESP protocol parser
- Implement `protocol/mod.rs` - parse raw bytes into a `Command` enum


### Session 004 - [2026-03-05]
**Phase:** Phase 3 - RESP Protocol Parser
**Commit(s):** 
- `seasion 04 - set up protocol; defined ENUMs for byte-wire-format(ProtocolError, RespValue, Command); low-level-parsing in progress; implemented parse_simple_string and parse_error. Also establish inline module testing with a single test-case`
- `Session 04 - implemented parse_integer`
- `Session 04 - implemented parse_bulk_string`
- `Session 04 - implemented parse_array(); 24 tests LLM-generated and all passed.`

#### What We Did
- Designed and implemented protocol/mod.rs - the low-level RESP parser
- Defined RespValue enum: SimpleString, BulkString(Option<Bytes>), Integer, Array, Error
- Defined Command enum with named fields: Ping, Get{key}, Set{key, value, ttl}, Del{key}, Exists{key}
- Defined ProtocolError enum with descriptive variants using `thiserror`: Incomplete, InvalidType(u8), InvalidFormat(String), InvalidCommand(String), WrongArgCount{command, expected, got}
- Implemented parse_value() dispatcher + 5 helpers: parse_simple_string, parse_error, parse_integer, parse_bulk_string, parse_array
- Used cursor-based approach (&mut usize) with recursion for arrays
- Implemented rollback-on-incomplete pattern - snapshot pos at function entry, restore on failure
- Wrote 24 unit tests covering all RESP types, edge cases, incomplete data, rollback, nested arrays, realistic Redis commands
- All 24 tests passing

#### Decisions Made
- Cursor + recursion approach for parsing - cursor tracks position in buffer, recursion handles nested arrays naturally
- Rollback pattern for incomplete TCP data - save pos before parsing, restore if Incomplete, so the server can retry when more bytes arrive
- BulkString(Option<Bytes>) instead of separate Nil variant - None represents $-1\r\n null, mirrors RESP spec more closely
- Included ttl in Command::Set as Option<Duration> - forward-thinking for Phase 5, avoids refactor later
- Descriptive error variants - InvalidCommand carries the name, WrongArgCount carries command/expected/got for easier debugging

#### Mistakes / Learnings
- Shadowing &mut references with let creates a new local instead of modifying the original - must use *pos to dereference
- Bulk string parsing doesn't need to scan for \r\n in the data - the length prefix tells you exactly where the data ends, that's the whole point of length-prefixed formats
- cargo test --lib doesn't work for binary-only projects - use cargo test instead
- b'+' syntax for byte literals when comparing against &[u8] buffers

#### Open Questions
- None

#### Next Session
- Implement parse_command() - high-level layer: RespValue (array of bulk strings) -> Command enum
- Implement encode() - serialize RespValue back to RESP bytes for server responses
- Wire protocol into the TCP server (Phase 4)


### Session 005 - [2026-03-07]
**Phase:** Phase 3 - RESP Protocol Parser (continued)
**Commit(s):** 
- `Session 05 - updated documentation; implemented parse_command() with arg error handling and such and returning Command-Enum objects; implemented encode() to turn RESP-value-types into bytes.`
- `session 05 - LLM-Generated tests established for testing parse_command() and encode()`

#### What We Did
- Implemented `parse_command()` - converts RespValue (array of bulk strings) into typed Command enum
- Implemented `encode()` - serializes RespValue back to RESP wire format bytes (inverse of parse_value)
- Generated 30 tests covering both functions: arg count validation, all command types, all RESP types, edge cases, round-trip encode-to-parse consistency
- All protocol tests passing 

#### Decisions Made
- Redis-style `EX`/`PX` flags for SET TTL instead of positional third arg - slightly more parsing complexity, but compatible with redis-cli and standard Redis clients

#### Mistakes / Learnings
- `String::from_utf8` takes owned `Vec<u8>`, `std::str::from_utf8` borrows `&[u8]` - use the latter when you just need to read
- `{:?}` debug format on Bytes prints `b"foo"` not raw bytes - use `extend_from_slice` for binary data instead of format strings
- `split_first()` returns `Option<(&first, &rest)>` - good for separating command name from args without consuming the vec
- `.ok_or()?` pattern cleanly converts `None` into a typed error and propagates

#### Open Questions
- Should `WrongArgCount.expected` be a String instead of usize to handle "2 or 4" cases? (I dont care for now)

#### Next Session
- Phase 4: Wire protocol into the TCP server
- handle_client reads bytes, parses commands, executes against Store, encodes and writes responses


### Session 005-02 - [2026-03-07]
**Phase:** Phase 4 - Wiring Protocol to Storage
**Commit(s):** 
- `session 05 - connected server with protocol; handling TcpStream buffer; executing commands; tested with redis-cli and it works.`

#### What We Did
- Rewrote `handle_client()` - now reads bytes into a persistent `BytesMut` buffer, parses RESP frames, dispatches commands against the store, encodes responses, and writes them back to the socket
- Implemented a dual-loop structure: outer loop reads from the socket, inner loop drains all complete RESP "frames" from the buffer before waiting for more data - handles pipelined commands
- `handle_client()` now returns `Result<(), Box<dyn std::error::Error>>` with error handling in the `tokio::spawn` call in `run()`
- Implemented `execute_command(cmd, store) -> RespValue` as a pure dispatch function - no IO, just takes a command and store reference, returns a RESP value
- All five commands wired up: PING -> SimpleString("PONG"), GET -> BulkString or null, SET -> SimpleString("OK"), DEL -> Integer(1/0), EXISTS -> Integer(1/0)
- Smoke tested with `redis-cli` - PING, SET, GET, DEL, EXISTS all working end-to-end

#### Decisions Made
- `execute_command` returns `RespValue` rather than writing to the socket directly - keeps IO out of dispatch logic, makes it unit-testable without Tokio or a real socket
- Inner parse loop to handle pipelined commands - without it, two commands in one TCP packet would stall on the second until the next read

#### Mistakes / Learnings
- `read_buf().await` returns the number of bytes read - client-disconnect detection must check that return value, not `buf.len()`
- `BytesMut` derefs to `&[u8]` so it can be passed directly to `parse_value`
- `buf.clear()` only needed on RESP-level parse errors (corrupted stream) - command-level errors (bad args) don't need it because the buffer was already advanced past the valid frame
- Unnecessary `&` on method calls like `&store.exists()` when the method returns by value - just call directly
- for Redis-standards, SET returns `+OK\r\n` (SimpleString), not an integer

#### Open Questions
- None

#### Next Session
- Phase 5: TTL / key expiration
- Extend `store.set()` to accept an optional Duration
- Background task for active expiry sweep + lazy expiry on GET



### Session 005-03 - [2026-03-07]
**Phase:** Phase 5 - TTL / Key Expiration
**Commit(s):** 
- `Sesson 05 - updated Entry struct to allow for expiry date; changed Store.Get()/Store.Exists() to lazy-check the time and delete them before continuing on with their normal logic; updated store.set() for expiry/TTL.`
- `Session 05 - implemented lazy expiry-checking/deletes on the get-set; implemented active-sweeping of expired-keys, as a method on the store, and spawned as a task in the main.rs`
- `Session 05 - pattern fixing in storage:store get()/exists() for lazy-expiry`

#### What We Did
- Extended `Entry` struct with `expires_at: Option<Instant>` field
- Updated `store.set()` to accept `Option<Duration>` and convert internally to `Instant` via `Instant::now() + duration`
- Implemented lazy expiry in `store.get()` using DashMap's `remove_if` - expired keys are deleted atomically on access
- Implemented lazy expiry in `store.exists()` - same pattern
- Wired `ttl` from `Command::Set` through `execute_command` into the store
- Implemented `store.expiry_sweep()` - async background loop that collects expired keys then deletes in a second pass to avoid DashMap deadlocks
- Spawned the sweep task in `main.rs` with a 3-second interval
- Tested with `redis-cli`: `SET foo bar EX 5`, confirmed value returns, confirmed nil after expiry

#### Decisions Made
- Duration-to-Instant conversion lives inside the store, not at the call site - keeps the store's internal representation encapsulated
- Two-phase sweep (collect then delete) to avoid holding DashMap Ref guards during removal - prevents same-shard deadlocks
- Sweep interval set at 3 seconds - simple fixed interval, not Redis-style probabilistic sampling
- `expiry_sweep` is an async method on Store, spawned externally in main.rs - keeps Store runtime-agnostic

#### Mistakes / Learnings
- `.unwrap()` on `expires_at` inside `remove_if` is a crash risk for keys without TTL - must guard with a None check or use `.map_or()`
- DashMap iteration yields Ref guards - safe as long as no `.await` is crossed while holding them
- `async fn` return type is implicit - no need to manually specify a future
- `&self` in an async method causes lifetime issues with `tokio::spawn` - clone the Store and call on the owned clone in a `async move` block

#### Open Questions
- One-shot `redis-cli` commands sometimes stall on rapid repeated use - likely a connection handling issue in handle_client, not TTL-related. Will investigate later. I wonder it's root-cause..?

#### Next Session
- Phase 6: Persistence - AOF or snapshot


### Session 006 - [2026-03-08]
**Phase:** Phase 6 - Persistence (Snapshot + AOF)
**Commit(s):** 
- `Session 06 - bug fix in server's handle_client which stalled the socket/TcpStream/buffer-reading.`
- `session 06 - persistence module partly scaffolding, specifcally EBD snapshotting.`
- `session 06 - installed serde/bincode, had to repair the project by adding something to settings.json; implemented new store-method to return a vector of key-value pairs found in the store's data.`
- `Session 06 - implemented snapshot.rs's collect_snapshot(), save(), load(); and documentation.`
- `session 06 - wired RDB snapshotting logic to main.rs`
- `session 06 - Bug fix in server's run() client-handling`
- `session 06 - implemented AOF-logging; rewired main.rs and server's run() and handle_client() to take the writer.`


#### What We Did
- Investigated and fixed the one-shot `redis-cli` stalling bug from Session 005 - root cause was `continue` instead of `break` in the inner parse loop's `Incomplete` arm, causing an infinite spin instead of returning to the outer read loop
- Fixed `run()`'s match on `handle_client` result - `_ =>` was matching both Ok and Err, printing "Error" on every normal disconnect. Changed to proper `Ok(_)` / `Err(e)` arms
- Designed and implemented `persistence/snapshot.rs` - RDB-style snapshotting
  - `SnapshotEntry` struct with serde derives for bincode serialization
  - `SnapshotError` enum using `thiserror` with `#[from]` for io::Error and bincode::ErrorKind
  - `collect_snapshot(store)` - iterates the DashMap, skips expired keys, converts `Instant` to remaining `Duration`, returns `Vec<SnapshotEntry>`
  - `save(entries, path)` - serializes with bincode, writes atomically (temp file + rename)
  - `load(path)` - reads file, deserializes, converts TTL durations back to `Instant`, populates and returns a Store
- Added `snapshot_entries()` method on Store to expose DashMap contents without making the data field public
- Wired snapshot loading into `main.rs` startup - loads from `dump.rdb` if it exists, crashes on corruption (strict mode, matches Redis behavior)
- Designed and implemented `persistence/aof.rs` - Append-Only File logging
  - `AofWriter` struct wrapping a `std::fs::File` opened in append+create mode via `OpenOptions`
  - `append(&mut self, bytes: &[u8])` - writes raw RESP bytes to the AOF file
  - Wrapped in `Arc<Mutex<AofWriter>>` for sharing across client tasks
- Integrated AOF logging into `handle_client`:
  - Captures raw RESP bytes from buffer before `buf.advance(pos)`
  - Uses `matches!(&cmd, Command::Set { .. } | Command::Del { .. })` to check write commands
  - Logs only after successful execution of write commands
- Wired `AofWriter` through `main.rs` -> `run()` -> `handle_client()` with Arc+Mutex cloning pattern
- Tested end-to-end: SET/GET working, `appendonly.aof` file created with raw RESP bytes confirmed

#### Decisions Made
- **Bincode v1.3.3 over v2/v3** - v3.0.0 is a tombstone release (maintainer faced harassment, ceased development)[LOL]. v2.0.1 works but v1.3.3 is battle-tested, widely used, and has the simplest serde-based API (`serialize`/`deserialize`).
- **Background snapshot saving** - collect entries from DashMap (fast, briefly locks shards), then write to disk in a spawned task. Avoids blocking the server during disk I/O. Consistency is maintained because the collected Vec is a point-in-time copy detached from the live store.
- **`std::sync::Mutex` for AofWriter, not `tokio::Mutex`** - more like a mistake than a discision but.. anyways: lock is only held for a quick synchronous file write, no `.await` while holding it. `std::sync::Mutex` is cheaper and correct for this use case.
- **AOF format is raw RESP bytes** - reuses existing parser for replay, zero new serialization code needed.
- **AOF fsync strategy: OS-managed** - no explicit fsync on every write. Simplest option, acceptable durability since snapshots cover the bulk of state. Matches Redis's `appendfsync no` option.
- **Log at caller level (`handle_client`), not inside `execute_command`** - keeps execute_command pure. handle_client already knows the command type and has access to the raw bytes.
- **Strict snapshot load failure** - if `dump.rdb` exists but is corrupted, crash on startup rather than silently starting fresh. Matches Redis behavior; silent data loss is worse than a noisy failure.

#### Mistakes / Learnings
- `continue` vs `break` in nested loops is a classic trap - `continue` re-runs the current loop, not the outer one. The stalling bug was exactly this.
- `_ =>` in match catches everything including Ok - always be explicit with `Ok(_)` and `Err(e)`
- rust-analyzer ABI mismatch: proc-macro DLLs must match the exact rustc version. `cargo clean` + rebuild fixes it after a toolchain update. Also fixed by installing `rust-analyzer` as a rustup component instead of using the standalone VS Code bundled version (`rust-analyzer.server.path: "rust-analyzer"`)
- Orphan modules in Rust get no IDE support - `persistence/snapshot.rs` had no intellisense until `mod persistence;` was added to main.rs and `mod snapshot;` to `persistence/mod.rs`. Two separate problems (ABI + orphan module) compounded into one confusing symptom.
- `Instant` can't be serialized - it's relative to boot time. Must convert to `Duration` (remaining TTL) for serialization, convert back on load.
- Snapshot TTL is approximate - if process is down for 10 seconds, keys get their full TTL back on reload instead of (TTL - downtime). Acceptable for now.
- `OpenOptions::new().create(true).append(true).open(path)` is the pattern for append-mode file creation
- `matches!(&cmd, Command::Set { .. })` checks enum variant without consuming the value - useful when you need to inspect then move

#### Open Questions
- None

#### Next Session
- Implement AOF replay on startup - `replay(path, store)` function that reads the AOF file, parses RESP frames in a loop (same cursor pattern as `handle_client`), and executes each command against the store
- Wire hybrid startup in `main.rs`: load snapshot first, then replay AOF on top
- Implement AOF truncation after successful snapshot (so AOF only contains commands since last snapshot)
- Implement periodic snapshot saving (background task on an interval, similar to expiry sweep)
- Update ARCHITECTURE.md with persistence module details

---

### Session 007 - [2026-03-08]
**Phase:** Phase 6 - Persistence (continued - Hybrid AOF/RDB)
**Commit(s):** 
- `session 06 - integrated snapshot/aof in the main.rs`
- `session 06 - implemented a hybrid wiring of AOF and Snapshotting persistence in main.rs; added a truncate() function to aofWriter-struct;`

#### What We Did
- Implemented `aof::replay(path, store)` - reads entire AOF file into memory with `std::fs::read()`, walks it with the same cursor-based parse loop as `handle_client` minus the socket/response logic. Reuses `server::execute_command` (made `pub`) to dispatch replayed commands against the store.
- Wired hybrid startup sequence in `main.rs`: load snapshot if `dump.rdb` exists -> replay AOF if `appendonly.aof` exists -> open fresh AofWriter. AOF replay runs independently of snapshot - works with or without a prior snapshot.
- Implemented periodic snapshot saving as a background Tokio task in `main.rs` - `collect_snapshot` runs on the async executor (fast, just reads DashMap), `save()` runs inside `spawn_blocking` to keep disk IO off the async threads.
- Implemented `AofWriter::truncate()` - `set_len(0)` + `seek(Start(0))` on the existing file handle. Called after a successful snapshot save to clear the AOF, since the snapshot now captures all state up to that point.

#### Decisions Made
- **AOF replay tolerates partial frames** - `Incomplete` at end of file is expected (crash mid-write), just stop and move on. Hard parse errors also stop replay with a warning rather than crashing, since the snapshot has the bulk of state.
- **AOF truncation after snapshot** - small race window exists: writes between `collect_snapshot` and `truncate` are lost. I'm disregarding it. It's an acceptable tradeoff - Redis has the same behavior, and the window is tiny in practice.
- **Periodic snapshot interval** - set to 300s(5m) for now. Not Redis-style conditional ("save after N writes in M seconds"), just a fixed interval. Simple and sufficient enough.

#### Mistakes / Learnings
- Tried `file.read_buf(&mut buf)` for AOF replay - that's Tokio's `AsyncReadExt`, not available on `std::fs::File`. `std::fs::read()` is the right tool for synchronous full-file reads.


#### Open Questions
- None

#### Next Session
- Phase 7: Benchmarking & hardening
- Stress test concurrent connections
- Benchmark GET/SET throughput
- Explore graceful shutdown



<!-- Copy-Paste Template  -->
<!--
### Session 00X - [DATE]
**Phase:** 
**Commit(s):** ``

#### What We Did


#### Decisions Made


#### Mistakes / Learnings


#### Open Questions


#### Next Session

-->