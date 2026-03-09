//Rust only compiles modules that are declared in `main.rs`
mod persistence;
mod protocol;
mod server;
mod storage;

use bincode::Error;
use storage::Store; //exposes the crate to be used implicitly for the rest of the code in here.

use tokio::io;
use tokio::net::TcpListener;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

use persistence::aof::AofWriter;
use std::sync::Arc;
use std::sync::Mutex;

use std::path::Path;

use crate::persistence::aof;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:6379").await?; // Redis standard is 6379

    println!("Redis Main.rs-Testing Entry..");

    let shutdown_token = CancellationToken::new(); // like Arc - you clone it to share it. When any clone calls `.cancel()`, all clones see it.

    // the sequence should be: load snapshot if exists -> replay AOF on top -> then open a fresh AOF writer.
    // initilize the store and the ARC-dashmap
    let store = if Path::new("dump.rdb").exists() {
        persistence::snapshot::load(Path::new("dump.rdb"))? //propagate error
    } else {
        Store::new()
    };

    if Path::new("appendonly.aof").exists() {
        persistence::aof::replay(Path::new("appendonly.aof"), store.clone())?; //propagate error
    }

    // create the writer (mutex'd and atomically referenced)
    let aof_writer = Arc::new(Mutex::new(AofWriter::new(Path::new("appendonly.aof"))?));

    // spawn a task that utilizes store's active key-expiry sweeping
    let store_clone: Store = store.clone(); // so we can call it on a OWNED `store`
    tokio::spawn(async move {
        store_clone.expiry_sweep(Duration::from_secs(3)).await; //every 3 seconds, sweep. 
    });

    // spawn a task that snapshots the data-store periodically for long-term persistance.
    // and also wipe the short-form persistance for state-validity (since the snapshot captures all state up to that point)
    let duration_in_seconds = 300;
    let store_clone2 = store.clone();
    let aof_writer_clone = aof_writer.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(duration_in_seconds)).await; // every 300sec/5min of server-up-time 
            let entries = persistence::snapshot::collect_snapshot(&store_clone2);
            match tokio::task::spawn_blocking(move || {
                persistence::snapshot::save(entries, Path::new("dump.rdb"))
            })
            .await
            {
                Ok(Ok(())) => match aof_writer_clone.lock() {
                    Ok(mut writer) => {
                        writer
                            .truncate()
                            .unwrap_or_else(|e| eprintln!("AOF truncate failed: {}", e)); //log error, non-fatal, keep serving
                        println!("Snapshot saved");
                    }
                    Err(e) => eprintln!("AOF lock poisoned after snapshot: {}", e), //log error, non-fatal, keep serving
                },
                Ok(Err(e)) => eprintln!("Snapshot save error: {}", e),
                Err(e) => eprintln!("Snapshot task panicked: {}", e),
            }
        }
    });

    let shutdown_clone = shutdown_token.clone();

    // run the server with cancellation-proofing
    tokio::select! {
        // run listener and client-handling
        _ = server::run(listener, store.clone(), aof_writer.clone(), shutdown_clone) => {
            // run() returned on its own (shouldn't happen, but handle it)
        }
        _ = tokio::signal::ctrl_c() => {
            println!("Ctrl+C received, shutting down...");
            shutdown_token.cancel(); // state propagates to clones who refer to it
        }
    }

    // -- shutdown clean up --
    // FINAL SNAPSHOT and FLUSH AOF
    // clones not needed, last-step, we will consume them
    println!("Saving final snapshot...");
    let entries = persistence::snapshot::collect_snapshot(&store);
    match tokio::task::spawn_blocking(move || {
        persistence::snapshot::save(entries, Path::new("dump.rdb")) //snapshot
    })
    .await
    {
        Ok(Ok(())) => match aof_writer.lock() {
            Ok(mut writer) => {
                writer 
                    .truncate() 
                    .unwrap_or_else(|e| eprintln!("AOF truncate failed: {}", e));
                println!("Final snapshot saved, AOF truncated.");
            }
            Err(e) => eprintln!("AOF lock poisoned during shutdown: {}", e),
        },
        Ok(Err(e)) => eprintln!("Final snapshot save error: {}", e),
        Err(e) => eprintln!("Final snapshot task panicked: {}", e),
    }

    // BTW: test using `redis-cli`

    Ok(()) // implicit Ok-response return
}
