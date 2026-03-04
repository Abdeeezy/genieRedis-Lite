//Rust only compiles modules that are declared in `main.rs`
mod storage;
mod server;

use storage::Store; //exposes the crate to be used implicitly for the rest of the code in here.
use tokio::io;
use tokio::net::{TcpListener};

#[tokio::main]
async fn main() -> io::Result<()> {

    let listener = TcpListener::bind("127.0.0.1:6379").await?; // Redis standard is 6379

    println!("Redis Main.rs-Testing Entry..");

    // initilize the store and the ARC-dashmap
    let store: Store = Store::new();

    
    server::run(listener, store).await;


    // do cargo run in the terminal, then open your browser and put in the url: "127.0.0.1:6379"
    // it'll show that the server works. 
    // Each browser request spawned a separate task - that's the concurrency model working correctly. 
    // The multiple connections are the browser making several HTTP attempts (browsers do that: retry, prefetch, etc.) Expected behavior.

    Ok(()) // implicit Ok-response return
}
