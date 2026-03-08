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
use std::io::Write;
use std::path::Path;

use tokio::time::Duration;
use tokio::time::Instant;

use bytes::Bytes;

use std::fs::File;

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
}


