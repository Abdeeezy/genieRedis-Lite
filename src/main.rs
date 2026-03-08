//Rust only compiles modules that are declared in `main.rs`
mod storage;
mod server;
mod protocol;
mod persistence;

use storage::Store; //exposes the crate to be used implicitly for the rest of the code in here.

use tokio::io;
use tokio::net::{TcpListener};
use tokio::time::Duration;

#[tokio::main]
async fn main() -> io::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:6379").await?; // Redis standard is 6379

    println!("Redis Main.rs-Testing Entry..");

    // initilize the store and the ARC-dashmap
    let store: Store = Store::new();

    // spawn a task that utilizes store's active key-expiry sweeping
    let store_clone = store.clone(); // so we can call it on a OWNED `store`
    tokio::spawn(async move {
        store_clone.expiry_sweep(Duration::from_secs(3)).await; //every 3 seconds, sweep. 
    });
    
    // run listener and client-handling
    server::run(listener, store).await;


    // test using `redis-cli` 


    Ok(()) // implicit Ok-response return
}
