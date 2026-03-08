// AOF (Append-Only File)
//      Every write command (SET, DEL) gets appended to a log file as it happens. it's storing the commands that produced the data.
//
//  On save: After executing a write, append the command (in RESP format or some serializable form) to the AOF file.
//  On startup: Read the file top-to-bottom, replay every command against an empty store. You end up in the same state as before the crash.
/*
Key questions:

    - Format - raw RESP bytes (simple, matches what clients send) or a custom format?
    - When to fsync - every write, every N seconds, or let the OS decide?
    - File growth - AOF grows forever. Real Redis solves this with "AOF rewrite" (compact the log by snapshotting current state as commands). We can skip this initially.
*/



use std::fs::OpenOptions;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::io::SeekFrom;
use std::io::Seek;

use tokio::time::Duration;
use tokio::time::Instant;

use bytes::{Buf, Bytes, BytesMut}; // BytesMut can be thought of as containing a buf: Arc<Vec<u8>>

use std::fs::File;

use super::storage::Store;
use super::protocol;
use super::server;

pub struct AofWriter {
    pub file: File
} 


impl AofWriter {
    // static method to create a `AofWriter` object
    pub fn new(path: &Path) -> Result<Self, tokio::io::Error> {
        // open file in append mode, create if doesn't exist
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?; // propagate error

        Ok(AofWriter { file }) //construct and return the writer
    }

    pub fn append(&mut self, bytes: &[u8]) -> Result<(), std::io::Error> {  // Bytes/Vec<u8>/slices all defer to &[u8], any of those can be passed in and work.
        // write to file
        self.file.write_all(bytes)?; //propagate error

        Ok(())
    }

    pub fn truncate(&mut self) -> Result<(), std::io::Error> {
        self.file.set_len(0)?; //wipes the contents
        self.file.seek(SeekFrom::Start(0))?; //resets the write cursor back to the start. 
        Ok(())
    }
}


pub fn replay(path: &Path, store: Store)-> Result<(), std::io::Error>{
    //basically server::handle_client's parse loop, minus the socket.

    // Read bytes into a buffer
    let data = std::fs::read(path)?; // Vec<u8>, whole file in memory
    let mut pos = 0;
    // Loop: parse RESP frame → convert to Command → execute against store → advance cursor
    // Stop when you hit Incomplete or run out of data
    loop {
            match protocol::parse_value(&data, &mut pos) {
                Err(protocol::ProtocolError::Incomplete) => break,
                Err(e) => { 
                    println!("Error in AOF-replay: {}", e);
                    break;
                 }
                Ok(value) => {
                    // parse_command, dispatch, encode, write...
                    match protocol::parse_command(value) {
                        Ok(cmd) => {
                            // if command-parsing successful...

                            //execute the command on the KV-Store
                            let _ = server::execute_command(cmd, &store);
                        }
                        Err(error) => {
                            println!("Error in AOF-replay: {}", error);
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    
}

