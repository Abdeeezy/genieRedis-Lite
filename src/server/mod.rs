use super::storage::Store;

use tokio::net::{TcpListener, TcpStream};

// TCP client handling and lister loop
pub async fn handle_client(socket: TcpStream, store: Store) {
    // todo
    println!("Being handled!");
    println!("Address: {:?}", socket.peer_addr());
}

pub async fn run(listener: TcpListener, store: Store) {
    println!("-- accepting inbound connections --");

    //forever loop
    loop {
        // listen for client-connections, if found, run handle_client code.
        match listener.accept().await {
            Ok((socket, addr)) => {
                
                // "overshadowing" the variable `store` which only occurs in this scope; the store-variable remains untouched outside of this scope
                let store = store.clone(); // clone per iteration, original stays
                
                // spawn an async task to allow for execution-concurrency.
                tokio::spawn(async move {
                    handle_client(socket, store).await;
                });

            }
            Err(e) => println!("couldn't get client: {:?}", e),
        }
    }
}
