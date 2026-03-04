# DEVLOG — redis-lite

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

### Session 001 — [2026-03-02 ~4AM]
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

### Session 002 — [2026-03-02]
**Phase:** 1
**Commit(s):** `set up dashmap data-storage and defined key/value data-type; also implemented a static method to create the store, and the SET/GET/DEL instance-methods.`

#### What We Did
- Designed and implemented storage/mod.rs — the core in-memory store
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


### Session 003 — [2026-03-03]
**Phase:** Phase 2 — TCP Server
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
- Implement `protocol/mod.rs` — parse raw bytes into a `Command` enum


<!-- Copy-Paste Template  -->
<!--
### Session 00X — [DATE]
**Phase:** 
**Commit(s):** ``

#### What We Did


#### Decisions Made


#### Mistakes / Learnings


#### Open Questions


#### Next Session

-->