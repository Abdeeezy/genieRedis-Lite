//Snapshot (RDB-style)
//      At some interval, serialize the entire store to a single file on disk. A point-in-time photo of all keys, values, and their expiry times.
//
//  On save: Iterate the DashMap, serialize every entry, write to a file (atomically - write to a temp file, then rename, so a crash mid-write doesn't corrupt your snapshot).
//  On startup: Read the file, deserialize, populate the DashMap.
/*
Key design considerations 
    - Serialization format - Serde + bincode. instead of a Custom binary; maybe later but I  want to simplify.
    - handling expiry times - Need to convert to a duration-from-now or absolute timestamp.
    - background-saving instead of blocking - clone-and-write in a background task?

    - have to use Serde v1.3.3 for battle-tested/reliabile version that works with Serde
        - cargo add bincode@1.3.3
        - cargo add serde --features derive
*/

use super::protocol;
use super::storage::Entry;
use super::storage::Store;

use std::path::Path;

use tokio::time::Duration;
use tokio::time::Instant;

use serde::{Serialize, Deserialize};



///// --- ENUMS --- 
// Serializable representation of one entry
#[derive(Serialize, Deserialize)] // Serde-derives for the bincode
struct SnapshotEntry {
    key: String,
    value: Vec<u8>,                     // Bytes -> Vec<u8> for serde
    ttl_remaining: Option<Duration>,    // converted from Instant
}

#[derive(Debug, thiserror::Error)]
pub enum SnapshotError {
    #[error("IO error: {0}")] 
    Io(#[from] std::io::Error), // The #[from] attributes give you free ? propagation from the underlying errors.
    #[error("Bincode error: {0}")]
    Bincode(#[from] Box<bincode::ErrorKind>),
}


const DEFAULT_PATH: &str = "dump.rdb";




//// --- OPERATIONS ---
/// Collect current state (fast, touches the store, blocking but the speed allows for it to be negilible to the run-time of the server)
pub fn collect_snapshot(store: &Store) -> Vec<SnapshotEntry>
{
    // Iterate the DashMap
    // For each entry, compute remaining TTL (skip if already expired)
    store.snapshot_entries().iter().map(|kv_pair| SnapshotEntry({key: kv_pair.key})) 
    // Collect into a Vec<SnapshotEntry>
}

/// Write to disk (slow, no store access - this is what runs in background in tokio.spawn() )
pub fn save(entries: Vec<SnapshotEntry>, path: &Path) -> Result<(), SnapshotError>{
    // iterate the entries
    // Serialize with bincode (serde)
        //let bytes: Vec<u8> = bincode::serialize(&entries)?;
    // Write to a temp file, then rename (atomic swap)
}

/// Load a snapshot file into a store, return the populated Store
pub fn load(path: &Path) -> Result<Store, SnapshotError>{
    // Read the file
    // Deserialize into Vec<SnapshotEntry>
        //let entries: Vec<SnapshotEntry> = bincode::deserialize(&bytes)?;
    // For each entry, convert TTL back to Instant, insert into a fresh Store
    // Return the Store
}