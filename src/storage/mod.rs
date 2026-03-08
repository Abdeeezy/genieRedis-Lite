use bytes::Bytes; // import the Bytes type from the bytes crate
use dashmap::DashMap;

// import the DashMap type from the dashmap crate
use std::sync::Arc; // import the Arc type from the standard library for thread-safe reference counting

use tokio::time::Duration;
use tokio::time::Instant;
use tokio::time::sleep;

#[derive(Clone)]
pub struct Entry {
    pub value: Bytes,                // raw value
    pub expires_at: Option<Instant>, // None = no expiry (attribute relevant to TTL - time-to-live context)
}

//not familiar to rust-paradigms so this is new to me but this allows the Store struct to be cheaply cloned
//      - and what it's actually doing is making any `clone()` on Store just increment the Arc refcount, not deep-copy'ing the DashMap
#[derive(Clone)]
pub struct Store {
    pub data: Arc<DashMap<String, Entry>>, // thread-safe, concurrent hash map for storing key-value pairs {string = hash key, Entry = value-in-bytes + metadata}
}

// implement the methods for the Store struct
impl Store {
    // Initializes both the dashmap and the ARC
    // implicitly a static function due to the lack of the self-referential `&self` parameter
    pub fn new() -> Self {
        Store {
            data: Arc::new(DashMap::new()),
        }
    }
    
    pub fn snapshot_entries(&self) -> Vec<(String, Entry)>{
        self.data.iter()
                    .map(|kv_pair| (kv_pair.key().clone(), kv_pair.value().clone()))
                    .collect()
    }

    pub fn get(&self, key: &str) -> Option<Bytes> {
        // Lazy-check the expiry date and remove it if it exceeds it's lifetime
        self.data.remove_if(key, |_, entry| {
            entry.expires_at.map_or(false, |expiry| Instant::now() > expiry)
        });

        //|entry| is a closure argument. The || is closure syntax in Rust - like an anonymous function/lambda.
        return self.data.get(key).map(|entry| entry.value.clone()); // retrieves the value associated with the given key, returning it as a clone of the Bytes if found, or None if the key does not exist in the store.

        // also in rust, return is optional, the last expression in a function is implicitly returned.
    }

    pub fn set(&self, key: &str, value: Bytes, ttl: Option<Duration>) {
        // convert the duration to a clock value, offset by how long it's desired to live.
        let expires_at = ttl.map(|d| Instant::now() + d);

        self.data.insert(
            key.to_string(),
            Entry {
                value: value,
                expires_at: expires_at,
            },
        ); // to_string on the string-reference so it can be owned by the dashmap
    }

    pub fn exists(&self, key: &str) -> bool {
        // Lazy-check the expiry date and remove it if it exceeds it's lifetime
        self.data.remove_if(key, |_, entry| {
            entry.expires_at.map_or(false, |expiry| Instant::now() > expiry)
        });

        return self.data.contains_key(key);
    }

    pub fn del(&self, key: &str) -> bool {
        return self.data.remove(key).is_some(); // removes the key-value pair associated with the given key from the store, returning true if the key was found and removed, or false if the key did not exist in the store.
    }

    // background sweep - the active deletion of expired-keys
    pub async fn expiry_sweep(&self, interval: Duration) {
        /*
           When iterating DashMap, each iteration yields a Ref guard.
               If you call .remove() while holding that guard, you can deadlock (same "shard") ["a 'shard' typically refers to a portion of shared state that is managed independently"]

           So, we need to collect expired keys first, drop the iterator, then remove them in a separate pass.
        */

        loop {
            sleep(interval).await;

            let mut key_vec: Vec<String> = Vec::new();

            // pass 1: collect expired keys
            for item in self.data.iter() {
                let key = item.key();
                let entry = item.value();
                if let Some(expires_at) = entry.expires_at {
                    if Instant::now() > expires_at {
                        key_vec.push(key.clone());
                    }
                }
            }

            // pass 2: remove them
            for key in key_vec {
                self.del(&key);
            }
        }
    }
}

// ------------ TEST-CASES - IGNORE ------------ \\
//Unit tests go inline - that's the Rust convention for testing module internals.

// run in cmd: `cargo test --lib storage`
#[cfg(test)]
mod tests {
    use super::*;

    // test create and add
    #[test]
    fn create_and_add_and_read() {
        //create and set key with string-value explicitly cast to bytes
        let store: Store = Store::new();
        store.set("stargazing", Bytes::from("stars in reach"), None);

        let result: Option<Bytes> = store.get("stargazing");

        // assert the bool-result of a comparison-operation
        assert_eq!(result.unwrap(), Bytes::from("stars in reach"));
    }

    // test - getting a key that doesn't exist
    #[test]
    fn get_nonexistent_key() {
        let store: Store = Store::new();

        let result: Option<Bytes> = store.get("stargazing");

        // should be empty, due to never being
        assert!(result.is_none());
        println!("WOO!");
    }

    // test - deleting a key that does exist
    #[test]
    fn delete_existent_key() {
        let store: Store = Store::new();

        store.set(
            "stargazing",
            Bytes::from("The stars wherein a sight blazes"),
            None,
        );

        let deletion_result: bool = store.del("stargazing");

        assert_eq!(deletion_result, true);
    }
    // test - deleting a key that shouldn't exist
    #[test]
    fn delete_nonexistent_key() {
        let store: Store = Store::new();

        let deletion_result: bool = store.del("stargazing");

        assert_eq!(deletion_result, false);
    }
    // test - clone shares state
    #[test]
    fn check_if_clone_shares_state() {
        use std::thread;

        let store = Store::new();
        let store2 = store.clone(); // same underlying DashMap

        let handler1 = thread::spawn(move || {
            //`move` -> capture a closure's environment by value
            store.set(
                "stargazing",
                Bytes::from("The stars wherein a sight blazes"),
                None,
            );
        });

        handler1.join().unwrap(); // wait for write to complete

        let handler2 = thread::spawn(move || {
            // then start the reader
            let result = store2.get("stargazing");
            assert!(result.is_some()); // does it exist, or is it NOT None
        });

        handler2.join().unwrap();
    }
}
