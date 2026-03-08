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


use super::storage::Store;

use std::path::Path;

use tokio::time::Duration;
use tokio::time::Instant;

use serde::{Serialize, Deserialize};

use bytes::Bytes;


///// --- STRUCTS / ENUMS --- 
// Serializable representation of one entry
#[derive(Serialize, Deserialize)] // Serde-derives for the bincode
pub struct SnapshotEntry {
    key: String,
    value: Vec<u8>,                     // Bytes -> Vec<u8> for serde
    ttl_duration_remaining: Option<Duration>,    // converted from Instant
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
    // Iterate the DashMap, and For each entry, compute remaining TTL (skip if already expired)
    let now = Instant::now();
    store.snapshot_entries()
        .into_iter() // Vec is owned, so take ownership of the elements rather than borrowing; it avoids unnecessary cloning.
        .filter_map(|(key, entry)| { 
            // skip expired entries
            if let Some(expires_at) = entry.expires_at {
                if expires_at <= now {
                    return None;
                }
            }
            // convert Instant to remaining Duration
            let ttl_duration_remaining = entry.expires_at.map(|expiry| expiry.duration_since(now));

            Some(SnapshotEntry {
                key,
                value: entry.value.to_vec(),  // Bytes -> Vec<u8>
                ttl_duration_remaining,
            })
        }).collect()// Collect into a Vec<SnapshotEntry>
}

/// Write to disk (slow, no store access - this is what runs in background in tokio.spawn() )
pub fn save(entries: Vec<SnapshotEntry>, path: &Path) -> Result<(), SnapshotError>{
    
    // serialize the entries (doesn't need to be individually done due to the derive[Serialize] on the struct) 
    let bytes = bincode::serialize(&entries)?; //propagate the Bincode-error as a SnapshotError-enum-object if occured, that's what the #[from] attribute on the error enum does. No manual match needed.
    
    // Write to a temp file, then rename (atomic swap)
    let tmp_path = path.with_extension("tmp"); //dump.rdb.tmp
    std::fs::write(&tmp_path, &bytes)?;
    std::fs::rename(&tmp_path, path)?;
   
    Ok(())
}

/// Load a snapshot file into a store, return the populated Store
pub fn load(path: &Path) -> Result<Store, SnapshotError>{
    // Read the file
    let bytes = std::fs::read(path)?; // propagate the IO error as a SnapshotError-enum-object if occurs..

    // Deserialize into Vec<SnapshotEntry>
    let entries: Vec<SnapshotEntry> = bincode::deserialize(&bytes)?; //propagate the Bincode-error as a SnapshotError-enum-object if occured
    
    // create the store
    let store: Store = Store::new();
    // For each entry, convert TTL back to Instant, insert into a fresh Store
    for entry in entries{
        // the store's set function internally sets the timestamp to be the current-time offset by the remaining duration
        store.set(
            &entry.key,
            Bytes::from(entry.value),
            entry.ttl_duration_remaining);
    }
    
    // Return the Store
    Ok(store)
}