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