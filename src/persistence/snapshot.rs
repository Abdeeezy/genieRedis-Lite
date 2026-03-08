//Snapshot (RDB-style)
//      At some interval, serialize the entire store to a single file on disk. A point-in-time photo of all keys, values, and their expiry times.
//
//  On save: Iterate the DashMap, serialize every entry, write to a file (atomically - write to a temp file, then rename, so a crash mid-write doesn't corrupt your snapshot).
//  On startup: Read the file, deserialize, populate the DashMap.
/*
Key questions we'll need to answer:

    - Serialization format - Serde + bincode? JSON? Custom binary?
    - How to handle expiry times - you can't serialize Instant directly (it's relative to boot time). Need to convert to a duration-from-now or absolute timestamp.
    - Blocking vs background - do we freeze the store to snapshot, or clone-and-write in a background task?
*/



/// Serializable representation of one entry
struct SnapshotEntry {
    key: String,
    value: Vec<u8>,                     // Bytes -> Vec<u8> for serde
    ttl_remaining: Option<Duration>,    // converted from Instant
}

struct SnapshotEntry {
    key: String,
    value: Vec<u8>,                     // Bytes -> Vec<u8> for serde
    ttl_remaining: Option<Duration>,    // converted from Instant
}


const FILE_PATH: Path = Path::From("dump.rdb");



//// --- OPERATIONS ---
/// 
/// Save the entire store to disk
pub fn save(store: &Store, path: &Path) -> Result<(), SnapshotError>{
    // Iterate the DashMap
    // For each entry, compute remaining TTL (skip if already expired)
    // Collect into a Vec<SnapshotEntry>
    // Serialize with bincode
    // Write to a temp file, then rename (atomic swap)
}

/// Load a snapshot file into a store, return the populated Store
pub fn load(path: &Path) -> Result<Store, SnapshotError>{
    // Read the file
    // Deserialize into Vec<SnapshotEntry>
    // For each entry, convert TTL back to Instant, insert into a fresh Store
    // Return the Store
}