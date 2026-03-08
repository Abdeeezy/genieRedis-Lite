# DECISIONS — redis-lite

A log of non-trivial technical decisions: what we chose, what we rejected, and why.

---

## How I Will Use This File
- Log a decision when a meaningful tradeoff was made
- Short is fine - the goal is a traceable record, not an essay
- Link to the relevant session in DEVLOG where possible

---

## Decision Log

---

### D001 — DashMap over RwLock\<HashMap\>
**Date:** [2026-03-02]  
**Phase:** Phase 1 — Storage  
**Session:** Session 001

**Decision:** Use `DashMap` as the core concurrent store.

**Options Considered:**
| Option | Notes |
|--------|-------|
| `DashMap` | Lock-free sharded map, ergonomic API, minimal boilerplate |
| `RwLock<HashMap>` | Stdlib only, full-map lock on writes - bottleneck under concurrency |
| `Mutex<HashMap>` | Simpler but worse read performance, same bottleneck problem |

**Why DashMap:**  
Surface-level consideration and decision made to streamline progress faster. If need be, this will change. but for now, it's fine. 

**Tradeoffs:**  
- External dependency (not stdlib)
- Slightly harder to reason about iteration consistency
- Deadlock risk if you hold a DashMap reference across an `.await` point — must be careful with async code

---

### D002 — Tokio as Async Runtime
**Date:** [2026-03-02]  
**Phase:** Phase 2 — Server  
**Session:** Session 001

**Decision:** Use `Tokio` for the async runtime and TCP networking.

**Options Considered:**
| Option | Notes |
|--------|-------|
| `Tokio` | Industry standard, massive ecosystem, best docs |


**Why [Tokio]:**  
Seems like a strong reputation standard for async Rust. Best documentation, most Stack Overflow answers, integrates with everything. Great for a networking-heavy project.

**Tradeoffs:**  
- Compile times are heavier
- Must be careful about blocking operations on the async executor (use `tokio::task::spawn_blocking` if needed)
---

### D003 — Protocol SET-Command Format Time-To-Live time-variability 
**Date:** [2026-03-07]  
**Phase:**   
**Session:** Session 005

**Decision:**  decided on time-variation flag for the SET-Command

**Options Considered:**
|                Option             |    Notes   |
|-----------------------------------|------------|
| forcing seconds only              | lazy       |
| allowing seconds or milliseconds  | flexibility|
 
**Why [allowing seconds or milliseconds]:**  
Chose establishing time-choice to allow for flexibility and precision. Also to match with Redis-standards; it'll work with stuff like Redis-cli more seamlessly.

**Tradeoffs:**  
Nothing.

### D00X — store set TTL disregard
**Date:** [2026-03-08]  
**Phase:**  5 
**Session:** Session 006

**Decision:** chose to disregard the RDB-snapshot loading semantics in the context of how the Store's Get() function is programmed.  

**Options Considered:**
|                        Option                                    |                           Notes                              |
|------------------------------------------------------------------|--------------------------------------------------------------|
| leave it                                                         | the accuracy drift between Instant::now() calls is negilible |
| reconfigure the store's get() and all functions that utilize it. |  it's annoying lol.                                          |

**Why [leave it]:**  
It's negilible to change and the accuracy drift really doesn't matter. it's a difference of nanometers- and if a key exists
for even 1 second longer than desired (which will never happen), who cares.. 

**Tradeoffs:**  
integrity/principles

<!-- COPY-PASTE TEMPLATE -->
<!--
### D00X — [Short Title]
**Date:** [DATE]  
**Phase:**   
**Session:** Session 00X

**Decision:** 

**Options Considered:**
| Option | Notes |
|--------|-------|
|  |  |
|  |  |

**Why [Chosen Option]:**  


**Tradeoffs:**  

-->