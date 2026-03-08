// to essentially match with real Redis the closest we'll implement persistance in the following way:
//      - RDB (Redis Database Backup) snapshots for periodic full-state dumps (compact, fast recovery)
//          - serialize/deserialize the store, save/load to disk
//      - AOF for write-ahead logging between snapshots (durability)
//          - append write commands, replay on startup




// this mod.rs acts as just the glue that tells Rust "here are the submodules inside persistence/"
use super::protocol;
use super::storage;
use super::server;

pub mod snapshot;
pub mod aof;

pub use snapshot::SnapshotError;



// Hybrid Startup 
// The logic on startup is:
/* 
    - If a snapshot file exists, load it (fast - gives you the bulk of state)
    - If an AOF file exists, replay only the entries written after the snapshot (fills the gap)
    - If neither exists, start fresh
*/
