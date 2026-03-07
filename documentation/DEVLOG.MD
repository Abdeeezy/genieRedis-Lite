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