//Rust only compiles modules that are declared in `main.rs`
mod storage;
mod server;
mod protocol;
mod persistence;

use bincode::Error;
use storage::Store; //exposes the crate to be used implicitly for the rest of the code in here.

use tokio::io;
use tokio::net::{TcpListener};
use tokio::time::Duration;


use std::sync::Arc;
use std::sync::Mutex;
use persistence::aof::AofWriter;


use std::path::Path;



#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    let listener = TcpListener::bind("127.0.0.1:6379").await?; // Redis standard is 6379

    println!("Redis Main.rs-Testing Entry..");

    // initilize the store and the ARC-dashmap
    let store: Store = if Path::new("dump.rdb").exists() { //  if RDB-snapshots file exists, load store with the data
        persistence::snapshot::load(Path::new("dump.rdb"))? // propagate error, crash ("file was found but something went seriously wrong somewhere..")
    } else {
        Store::new()
    };

    // spawn a task that utilizes store's active key-expiry sweeping
    let store_clone: Store = store.clone(); // so we can call it on a OWNED `store`
    tokio::spawn(async move {
        store_clone.expiry_sweep(Duration::from_secs(3)).await; //every 3 seconds, sweep. 
    });
    
    // create the writer (mutex'd and atomically referenced)
    let aofWriter = Arc::new(Mutex::new(AofWriter::new(Path::new("appendonly.aof"))?));

    // run listener and client-handling
    server::run(listener, store, aofWriter).await;


    // BTW: test using `redis-cli` 


    Ok(()) // implicit Ok-response return
}
