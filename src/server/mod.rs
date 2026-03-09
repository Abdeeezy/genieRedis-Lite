use super::protocol;
use super::protocol::Command;
use super::protocol::RespValue;
use super::storage::Store;

use tokio::net::{TcpListener, TcpStream};

use tokio::time::Instant;

use tokio::io::AsyncReadExt; // gives `read_buf()` which writes directly into BytesMut
use tokio::io::AsyncWriteExt; // gives `write_all()` for writing responses back
use tokio_util::sync::CancellationToken;

use bytes::{Buf, Bytes, BytesMut}; // BytesMut can be thought of as containing a buf: Arc<Vec<u8>>
// BytesMut's BufMut implementation will implicitly grow its buffer as necessary.

use super::persistence::aof::AofWriter;
use std::sync::Arc;
use std::sync::Mutex;

// TCP client handling and lister loop
pub async fn handle_client(
    mut socket: TcpStream,
    store: Store,
    aof_writer: Arc<Mutex<AofWriter>>,
) -> Result<(), Box<dyn std::error::Error>> {
    // todo
    println!("Being handled!");
    println!("Address: {:?}", socket.peer_addr());

    /*
       Uses a persistent buffer (BytesMut) that accumulates across reads.
       Attempts to parse after each read.
       On Incomplete - loop back and read more bytes
       On success - advance the buffer past the consumed bytes, then execute the command
    */

    let mut buf = BytesMut::with_capacity(512); // start with 512 bytes 

    loop {
        // 1. read from socket into buffer
        // and store the number of bytes the latest-read added
        let num_bytes_read = socket.read_buf(&mut buf).await?; // propagate error back to run()

        // 2. if 0 bytes → client disconnected, break
        if num_bytes_read == 0 {
            println!("Client Disconnected.");
            break;
        }

        // 3. try parse_value on buffer
        //    - Incomplete → continue (read more)
        //    - Error → send RESP error, reset buffer
        //    - Ok(value) → advance buffer, parse_command, dispatch, encode, write
        //
        // done in an inner loop to consume all complete frames in the buffer;
        //          so that commands sent in batches aren't neglected and so the stream doesn't stall.
        loop {
            let mut pos: usize = 0;
            match protocol::parse_value(&buf, &mut pos) {
                //&buf works because BytesMut derefs to &[u8]
                Err(protocol::ProtocolError::Incomplete) => {
                    // if buffer incomplete, don't consume any bytes and just keep waiting for more info/data/bytes
                    break; // break inner loop -> back to outer loop to read more bytes
                }
                Err(error) => {
                    //send back a RESP error
                    let resp = protocol::encode(&RespValue::Error(error.to_string()));
                    socket.write_all(&resp).await?;

                    //clear the buffer from the garbage/corruption
                    buf.clear();
                }
                Ok(value) => {
                    // temp-store for the AOF-log further on..
                    let raw = buf[..pos].to_vec();

                    // After a successful parse, advance the buffer
                    buf.advance(pos); // drop the consumed bytes  (bytes::Buf.advance())

                    // now parse_command, dispatch, encode, write...
                    match protocol::parse_command(value) {
                        Ok(cmd) => {
                            // if command-parsing successful...

                            // check if it's a SET or DEL (the only commands we care to log the AOF)
                            let is_write =
                                matches!(&cmd, Command::Set { .. } | Command::Del { .. });

                            //execute the command on the KV-Store
                            let result = execute_command(cmd, &store);

                            if is_write {
                                // log the respvalue (in it's wire byte form) to the aof log
                                match aof_writer.lock() {
                                    Ok(mut writer) => {
                                        if let Err(e) = writer.append(&raw) { //log IO error, non-fatal handling, keep serving 
                                            eprintln!("AOF write failed: {}", e);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("AOF lock poisoned, skipping write: {}", e); //log mutex-poisoning error, non-fatal, keep serving 
                                    }
                                }
                            }

                            // send result back
                            let resp = protocol::encode(&result);
                            socket.write_all(&resp).await?;
                        }
                        Err(error) => {
                            // if invalid command attempted...
                            //send back a RESP error
                            let resp = protocol::encode(&RespValue::Error(error.to_string()));
                            socket.write_all(&resp).await?;

                            // no buffer-clear needed..
                            // The RESP frame itself was valid (it passed), it's just the command that was wrong
                            // the buffer was already advanced past the consumed bytes
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn execute_command(cmd: protocol::Command, store: &Store) -> RespValue {
    match cmd {
        protocol::Command::Ping => RespValue::SimpleString("PONG".into()),
        protocol::Command::Exists { key } => {
            //Exists
            let key_exists = store.exists(&key);
            if (key_exists == true) {
                // redis standard, returns an integer
                RespValue::Integer(1)
            } else {
                RespValue::Integer(0)
            }
        }
        protocol::Command::Del { key } => {
            //Del
            let was_key_deleted = store.del(&key);
            if (was_key_deleted == true) {
                RespValue::Integer(1)
            } else {
                RespValue::Integer(0)
            }
        }
        protocol::Command::Get { key } => {
            // Get
            match store.get(&key) {
                Some(value) => RespValue::BulkString(Some(value)),
                None => RespValue::BulkString(None),
            }
        }
        protocol::Command::Set { key, value, ttl } => {
            // Set
            store.set(&key, value, ttl);
            RespValue::SimpleString("OK".to_string()) //certain success
        }
    }
}

pub async fn run(
    listener: TcpListener,
    store: Store,
    aof_writer: Arc<Mutex<AofWriter>>,
    shutdown: CancellationToken,
) {
    println!("-- accepting inbound connections --");

    //forever loop
    loop {
        // macro explained..
        // Race two futures: accept a new connection OR receive shutdown signal.
        // Whichever resolves first runs its branch; the other is dropped.
        tokio::select! {
            _ = shutdown.cancelled() => {
                println!("Shutdown signal received, stopping accept loop.");
                break;
            }
            // listen for client-connections, if found, run handle_client code.
            result = listener.accept() => {
                match result {
                    Ok((socket, _addr)) => {
                        // "overshadowing" the variable `store` which only occurs in this scope; the store-variable remains untouched outside of this scope
                        let store = store.clone(); // clone per iteration, original stays
                        let aof_writer = aof_writer.clone(); // clone the Arc before moving into spawn (ggain, just increments the reference count)

                        // spawn an async task to allow for execution-concurrency.
                        tokio::spawn(async move {
                            match handle_client(socket, store, aof_writer).await {
                                Ok(_) => println!("Client session ended."),
                                Err(e) => println!("Error in handle_client: {}", e),
                            }
                        });
                    }
                    Err(e) => println!("couldn't get client: {:?}", e),
                }
            }
        }
    }
}
