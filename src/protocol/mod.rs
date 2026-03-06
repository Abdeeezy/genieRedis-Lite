
use tokio::time::Duration;
use bytes::Bytes;
use thiserror::Error; // for the error handling in the protocol parsing - derives `std::error::Error` and `Display`  


// module-goal: parse raw RESP bytes into a Command enum, and encode responses back to RESP bytes.
// protocol parsing and command handling
//
// ---
// Redis-Serialization-Protocol (RESP) wire format
// ---
// Type byte  |      Meaning    |    Example
// -----------|-----------------|----------------
// +          |  Simple String  |  `+OK\r\n`
// -          |  Error          |  `-ERR unknown command\r\n`
// :          |  Integer        |  `:1000\r\n`
// $          |  Bulk String    |  `$longer string\r\ncontinues onwards\r\n`   --  [Null bulk string is `$-1\r\n`]
// *          |  Array          |  `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`
// ---
// Clients send commands as arrays of bulk strings.
// EXAMPLE: So `GET foobar` arrives as:
// |`*2\r\n$3\r\nGET\r\n$6\r\nfoobar\r\n`|
// |
// |    `*2\r\n`             ->   indicates an array of 2 elements
// |    `$3\r\nGET\r\n`      ->   is the first bulk string (the command name)
// |    `$6\r\nfoobar\r\n`   ->   is the second bulk string (the command value)



//// Define ENUMS/data-types for use 


#[derive(Debug, Error)]
enum ProtocolError {
    #[error("incomplete data, need more bytes")] // generates a Display implementation  
    Incomplete,
    #[error("invalid RESP type byte: 0x{0:02x}")] // 0x{0:02x} formats the byte as a two-digit hexadecimal number
    InvalidType(u8),
    #[error("invalid format: {0}")]
    InvalidFormat(String),
    #[error("invalid command: {0}")]
    InvalidCommand(String),
    #[error("wrong arg count for {command}: expected {expected}, got {got}")]
    WrongArgCount { command: String, expected: usize, got: usize },
}


//   Command - struct-like enum variant
enum Command {
    Ping,                                          // health-checking server purposes - no args
    Get { key: String },                           // named-field, accessed by named
    Set { key: String, value: Bytes, ttl: Option<Duration> }, // 2-3 args
    Del { key: String },                           
    Exists { key: String },                        
}

// RESP wire format vocabulary - used in both directions of the client/server
//   RespValue - struct-like enum variant
#[derive(Debug)] //for the in-line tests/asserts at the bottom of this file
enum RespValue {
    SimpleString(String),       // unnamed field, accessed by position
    BulkString(Option<Bytes>),  // Option allows for the possibility of a null/nil bulk string, which is represented as None
    Integer(i64),
    Array(Vec<RespValue>),
    Error(String),
}

//The parsing job is two layers:
// - Low-level:     raw bytes -> RespValue (generic RESP decoding)
// - High-level:    RespValue (specifically an array of bulk strings) -> Command enum

/* 
```
That takes the `RespValue` (which should be an array of bulk strings if the client is behaving),
    pulls out the command name, 
    matches it, 
    extracts the args, 
    and returns your `Command` enum. 
    
    That's where `"SET"` + `"foo"` + `"bar"` becomes `Command::Set { key, value, ttl: None }`.

// Clients send commands as arrays of bulk strings.
// EXAMPLE: So `GET foobar` arrives as:
// |`*2\r\n$3\r\nGET\r\n$6\r\nfoobar\r\n`|
// |
// |    `*2\r\n`             ->   indicates an array of 2 elements
// |    `$3\r\nGET\r\n`      ->   is the first bulk string (the command name)
// |    `$6\r\nfoobar\r\n`   ->   is the second bulk string (the command value)




So the full pipeline is:
```
raw bytes  →  parse_value()  →  RespValue  →  parse_command()  →  Command
*/

//// --- LOW-LEVEL PARSING AREA ---
fn parse_value(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>{

    // return incomplete if not enough bytes
    if *pos >= buf.len(){
        return Err(ProtocolError::Incomplete)
    }
        

    let type_byte = buf[*pos];

    match type_byte {
        // b'[sequence]'   -> tells compiler that it should be treated as a byte string-literal 
        b'+' => parse_simple_string(buf, pos),
        // b'-' => parse_error(buf, pos),
        //b':' => parse_integer(buf, pos),
        //b'$' => parse_bulk_string(buf, pos),
        //b'*' => parse_array(buf, pos),
        _   =>  Err(ProtocolError::InvalidType(type_byte)),
    }

    
}

fn parse_simple_string(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>
{
    //simple_string example: `+hello\r\n`

    let start = *pos;       // snapshot for rollback

    
    // skip past the '+' byte
    *pos += 1;

    // find the next \r\n starting from pos
    for i in *pos .. buf.len(){
        
        // if `\r\n` found
        if i + 1 < buf.len() && buf[i] == b'\r' && buf[i+1] == b'\n'
        {
            // extract/store the string between pos and \r\n
            let str = String::from_utf8(buf[*pos..i].to_vec())
                .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?;

            // advance the cursor-reference, skip past the `\r\n` 
            *pos = i + 2;
            return Ok((RespValue::SimpleString((str))))
        }    
    }

    // if the loop escapes without return an OK() response, then it failed
    // thus: rollback and ERR()
    *pos = start;           // rollback — nothing was consumed
    Err(ProtocolError::Incomplete)
}


fn parse_error(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>
{
    //simple_string example: `-ERR erronous behaviour\r\n`

    let start = *pos;       // snapshot for rollback

    // skip past the '-' byte
    *pos += 1;

    // find the next \r\n starting from pos
    for i in *pos .. buf.len(){
        // if `\r\n` found
        if i + 1 < buf.len() && buf[i] == b'\r' && buf[i+1] == b'\n'
        {
            // extract/store the string between pos and \r\n
            let str = String::from_utf8(buf[*pos..i].to_vec())
                .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?;

            // advance the cursor-reference, skip past the `\r\n` 
            *pos = i + 2;
            return Ok((RespValue::Error((str))))
        }    
    }

    // if the loop escapes without return an OK() response, then it failed
    // thus: rollback and ERR()
    *pos = start;           // rollback — nothing was consumed
    Err(ProtocolError::Incomplete)
}

/*

fn parse_integer(buf, pos):
    // skip past ':'
    // find \r\n
    // extract text between pos and \r\n
    // parse that text as i64 (fail → InvalidFormat)
    // advance pos past \r\n
    // return Integer(value)


fn parse_bulk_string(buf, pos):
    // start = pos //snapshot before we touch anything in case there is incomplete data further down the bulk-string.
    // skip past '$'
    // find \r\n
    // extract length string, parse as integer
    // if length == -1 → advance pos, return BulkString(None)   [null]
    // if buf doesn't have enough bytes for length + \r\n 
    //      pos = start  // rollback, like nothing happened
    //      return Incomplete
    // extract `length` bytes starting after the \r\n
    // verify the next two bytes are \r\n
    // advance pos past all of it
    // return BulkString(Some(data))


fn parse_array(buf, pos):
    // start = pos //snapshot before we touch anything in case there is incomplete data further down the array.
    // skip past '*'
    // find \r\n
    // extract count, parse as integer
    // if count == -1 → return Nil or BulkString(None), your call
    // create empty vec
    // loop `count` times:
    //     result = parse_value(buf, pos)  // RECURSION
    //     if error, 
    //          pos = start  // rollback, like nothing happened
    //          return error 
    //     push result into vec
    // return Array(vec)
 */



//// --- LOW-LEVEL PARSING AREA ---



//// --- High-LEVEL PARSING AREA ---
// Redis clients always send commands as an array of bulk strings. 
fn parse_command(value: RespValue) -> Result<Command, ProtocolError> {
    
    // ensure array of bulk strings
    let parts = match value {
        RespValue::Array(parts) => parts,                           // success-case
        _ => return Err(ProtocolError::InvalidFormat(("expected array").into())),   // fail-case
    };
    

    Ok((Command::Ping)) // TODO - remove. this was just to get rid of compiler error
    
}




// ------------ TEST-CASES - IGNORE ------------ \\
//Unit tests go inline - that's the Rust convention for testing module internals.


#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_parse_simple_string() {
        let buf = b"+OK\r\n";
        let mut pos = 0;
        let result: Result<RespValue, ProtocolError> = parse_value(buf, &mut pos);

        // assert on result and pos
        match result {
            Ok(RespValue::SimpleString(s)) => assert_eq!(s, "OK"),
            other => panic!("expected SimpleString(\"OK\"), got {:?}", other),
        }
        assert_eq!(pos, 5); // should have consumed all 5 bytes
            
    }
}