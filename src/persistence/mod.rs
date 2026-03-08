// to essentially match with real Redis the closest we'll implement persistance in the following way:
//      - RDB (Redis Database Backup) snapshots for periodic full-state dumps (compact, fast recovery)
//          - serialize/deserialize the store, save/load to disk
//      - AOF for write-ahead logging between snapshots (durability)
//          - append write commands, replay on startup




// this mod.rs acts as just the glue that tells Rust "here are the submodules inside persistence/"
use super::protocol;
use super::storage;

pub mod snapshot;
//pub mod aof;

pub use snapshot::SnapshotError;



// AOF (Append-Only File)
//      Every write command (SET, DEL) gets appended to a log file as it happens. it's storing the commands that produced the data.
//
//  On save: After executing a write, append the command (in RESP format or some serializable form) to the AOF file.
//  On startup: Read the file top-to-bottom, replay every command against an empty store. You end up in the same state as before the crash.
/*
Key questions:

    - Format - raw RESP bytes (simple, matches what clients send) or a custom format?
    - When to fsync - every write, every N seconds, or let the OS decide?
    - File growth - AOF grows forever. Real Redis solves this with "AOF rewrite" (compact the log by snapshotting current state as commands). We can skip this initially.
*/






// Hybrid Startup 
// The logic on startup is:
/* 
    - If a snapshot file exists, load it (fast - gives you the bulk of state)
    - If an AOF file exists, replay only the entries written after the snapshot (fills the gap)
    - If neither exists, start fresh
*/
