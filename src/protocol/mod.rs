
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
// $          |  Bulk String    |  `$30\r\nlonger string\r\ncontinues onwards\r\n`   --  [Null bulk string is `$-1\r\n`]
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
-------
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
    raw bytes  →  parse_value()  →  RespValue  →  parse_command()  →  Command
-------
*/

//// --- LOW-LEVEL PARSING AREA ---
fn parse_value(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>{

    // return incomplete if not enough bytes
    if *pos >= buf.len(){
        return Err(ProtocolError::Incomplete)
    }
        

    let type_byte = buf[*pos];

    match type_byte {
        // b'[character]'   -> tells compiler that it should be treated as a byte character-literal  //-//  b"str" for string-literal 
        b'+' => parse_simple_string(buf, pos),
        b'-' => parse_error(buf, pos),
        b':' => parse_integer(buf, pos),
        b'$' => parse_bulk_string(buf, pos),
        b'*' => parse_array(buf, pos),
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
    //error example: `-ERR erroneous behaviour\r\n`

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

fn parse_integer(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>
{
    //integer example: `:1337\r\n`

    let start = *pos;       // snapshot for rollback

    // skip past the ':' byte
    *pos += 1;

    // find the next \r\n starting from pos
    for i in *pos .. buf.len(){
        // if `\r\n` found
        if i + 1 < buf.len() && buf[i] == b'\r' && buf[i+1] == b'\n'
        {
            // extract/store the string between pos and \r\n
            let str = String::from_utf8(buf[*pos..i].to_vec())
                .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?;

            // parse the extracted-string
             let num:i64 = str.parse::<i64>()
                  .map_err(|_| ProtocolError::InvalidFormat("invalid integer".into()))?;
            
            // advance the cursor-reference, skip past the `\r\n` 
            *pos = i + 2;
            return Ok((RespValue::Integer((num))))
        }    
    }

    // if the loop escapes without return an OK() response, then it failed
    // thus: rollback and ERR()
    *pos = start;           // rollback — nothing was consumed
    Err(ProtocolError::Incomplete)
}



fn parse_bulk_string(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>
{
    //bulk_string example: `$30\r\nlonger string\r\ncontinues onwards\r\n`   --  [Null bulk string is `$-1\r\n`]

    let start = *pos;       // snapshot for rollback

    // skip past the '$' byte
    *pos += 1;

    //let mut bulkStringBytes: <Option<Bytes, None>

    // find the next \r\n starting from pos
    for i in *pos .. buf.len(){
        // lengthPrefix-extraction - if `\r\n` found
        if i + 1 < buf.len() && buf[i] == b'\r' && buf[i+1] == b'\n'
        {
            // extract/store the string between pos and \r\n
            let str = String::from_utf8(buf[*pos..i].to_vec())
                .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?;

            // parse for the length-prefix-value
            let length = str.parse::<i64>()
                  .map_err(|_| ProtocolError::InvalidFormat("invalid integer".into()))?;

            // if nothing/null/nil supplied, return accordingly
            if length == -1{
                *pos = i + 2;  // advance past the \r\n
                return Ok(RespValue::BulkString(None))
            }

            // if length-supplied is larger than the bytes available in the buffer
            let u_size_length: usize = length.try_into().map_err(|_| ProtocolError::InvalidFormat(("Failed conversion").into()))?; //error shouldn't run 
            if *pos + u_size_length + 2 > buf.len()
            {
                *pos = start;           // rollback — nothing was consumed
                return Err(ProtocolError::Incomplete);
            }


            // advance the cursor-reference, skip past the `\r\n` 
            *pos = i + 2;

            // extract exactly `length` bytes — no scanning needed
            let data = Bytes::copy_from_slice(&buf[*pos .. *pos + u_size_length]);
            // verify the terminator is there
            if buf[*pos + u_size_length] != b'\r' || buf[*pos + u_size_length + 1] != b'\n' {
                *pos = start; //rollback
                return Err(ProtocolError::InvalidFormat("missing bulk string terminator".into()));
            }

            // advance past data + \r\n
            *pos = *pos + u_size_length + 2;

            return Ok(RespValue::BulkString(Some(data)));
        } 
    }

    // if the loop escapes without return an OK() response, then it failed
    // thus: rollback and ERR()
    *pos = start;           // rollback — nothing was consumed
    Err(ProtocolError::Incomplete)
}


fn parse_array(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>
{
    //array example: `*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n`
    
    
    let start = *pos;       // snapshot for rollback

    // skip past the '*' byte
    *pos += 1;

    // find the next \r\n starting from pos
    for i in *pos .. buf.len()
    {
        // if `\r\n` found
        if i + 1 < buf.len() && buf[i] == b'\r' && buf[i+1] == b'\n'
        {
           
             // extract/store the string between pos and \r\n
            let str = String::from_utf8(buf[*pos..i].to_vec())
                .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?;

            // parse for the array-length-prefix-value
            let array_length = str.parse::<i64>()
                  .map_err(|_| ProtocolError::InvalidFormat("invalid integer".into()))?;

            // if null array-length supplied, return corresponding value
            // apparently this is just RESP-standard
            //      array-length of 0 is treated as "an empty-box"
            //      array-length of -1 is treated as "the box not existing"
            //      anything less is treated as erroneous 
            if array_length == -1{
                //advance the cursor 
                *pos = i + 2;
                return Ok(RespValue::BulkString(None));
            }
            if array_length < -1 {
                return Err(ProtocolError::InvalidFormat("negative array length".into()));
            }

             // advance the cursor-reference, skip past the `\r\n` 
            *pos = i + 2;

            // create a vector to populate with the recursively obtained values..
            let mut elements_vector:Vec<RespValue> = Vec::new();
            for _ in 0 .. array_length as usize{ //no need for any real index-variable
                // RECURSIVELY ("higher-order recursion" I guess I would call it? or "indirect-recursion"? because this function is called by a different function that would then call THIS function)
                let result: Result<RespValue, ProtocolError> = parse_value(buf, pos); 

                match result{
                    Ok(value) => elements_vector.push(value),
                    Err(e) =>{
                        *pos = start; //ROLLBACK
                        return Err(e); //propagate to the higher-order/parental error-handling
                    }
                }
            }

            return Ok((RespValue::Array(elements_vector)));


        }    
    }

     *pos = start;           // rollback — nothing was consumed
    Err(ProtocolError::Incomplete)
}
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









//// ------------ TEST-CASES - IGNORE ------------ \\\\
//Unit tests go inline - that's the Rust convention for testing module internals.

/* 

24 Claude-Generated tests covering:

    - Each RESP type (happy path + edge cases)
    - Incomplete data with rollback verification for every type that can be incomplete
    - Null values (bulk string and array)
    - Empty values (empty string, empty array, zero-length bulk string)
    - Invalid input (bad type byte, empty buffer, non-numeric integer)
    - Nested arrays (the recursion test)
    - Binary data with embedded \r\n (the length-prefix test — this one will catch bugs if your bulk string parser accidentally scans for \r\n instead of trusting the length)
    - Realistic Redis commands (what redis-cli actually sends)
*/

// run in cmd: `cargo test --lib protocol`
#[cfg(test)]
mod tests {
    use super::*;

    // -- SimpleString --------------------------------------

    #[test]
    fn test_parse_simple_string() {
        let buf = b"+OK\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::SimpleString(s)) => assert_eq!(s, "OK"),
            other => panic!("expected SimpleString(\"OK\"), got {:?}", other),
        }
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_parse_simple_string_empty() {
        let buf = b"+\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::SimpleString(s)) => assert_eq!(s, ""),
            other => panic!("expected SimpleString(\"\"), got {:?}", other),
        }
        assert_eq!(pos, 3);
    }

    #[test]
    fn test_parse_simple_string_incomplete() {
        let buf = b"+OK";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::Incomplete) => {}
            other => panic!("expected Incomplete, got {:?}", other),
        }
        assert_eq!(pos, 0); // rollback
    }

    // -- Error ---------------------------------------------

    #[test]
    fn test_parse_error() {
        let buf = b"-ERR unknown command\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Error(s)) => assert_eq!(s, "ERR unknown command"),
            other => panic!("expected Error, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    // -- Integer -------------------------------------------

    #[test]
    fn test_parse_integer_positive() {
        let buf = b":1337\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Integer(n)) => assert_eq!(n, 1337),
            other => panic!("expected Integer(1337), got {:?}", other),
        }
        assert_eq!(pos, 7);
    }

    #[test]
    fn test_parse_integer_negative() {
        let buf = b":-42\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Integer(n)) => assert_eq!(n, -42),
            other => panic!("expected Integer(-42), got {:?}", other),
        }
        assert_eq!(pos, 6);
    }

    #[test]
    fn test_parse_integer_zero() {
        let buf = b":0\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Integer(n)) => assert_eq!(n, 0),
            other => panic!("expected Integer(0), got {:?}", other),
        }
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_parse_integer_invalid() {
        let buf = b":notanumber\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::InvalidFormat(_)) => {}
            other => panic!("expected InvalidFormat, got {:?}", other),
        }
    }

    // -- BulkString ----------------------------------------

    #[test]
    fn test_parse_bulk_string() {
        let buf = b"$5\r\nhello\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::BulkString(Some(b))) => assert_eq!(&b[..], b"hello"),
            other => panic!("expected BulkString(Some(\"hello\")), got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_parse_bulk_string_empty() {
        let buf = b"$0\r\n\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::BulkString(Some(b))) => assert_eq!(b.len(), 0),
            other => panic!("expected BulkString(Some(empty)), got {:?}", other),
        }
        assert_eq!(pos, 6);
    }

    #[test]
    fn test_parse_bulk_string_null() {
        let buf = b"$-1\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::BulkString(None)) => {}
            other => panic!("expected BulkString(None), got {:?}", other),
        }
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_parse_bulk_string_incomplete_data() {
        let buf = b"$10\r\nhello"; // claims 10 bytes, only 5 present
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::Incomplete) => {}
            other => panic!("expected Incomplete, got {:?}", other),
        }
        assert_eq!(pos, 0); // rollback
    }

    #[test]
    fn test_parse_bulk_string_incomplete_length() {
        let buf = b"$5\r"; // \r\n not complete
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::Incomplete) => {}
            other => panic!("expected Incomplete, got {:?}", other),
        }
        assert_eq!(pos, 0);
    }

    #[test]
    fn test_parse_bulk_string_with_crlf_inside() {
        // bulk string containing \r\n in the data itself — length-prefix handles this
        let buf = b"$7\r\nfoo\r\nbar\r\n"; // "foo\r\nba" wait — let me be precise
        // data is: f o o \r \n b a = 7 bytes, then \r\n terminator
        let buf = b"$7\r\nfoo\r\nba\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::BulkString(Some(b))) => assert_eq!(&b[..], b"foo\r\nba"),
            other => panic!("expected BulkString with embedded crlf, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    // -- Array ---------------------------------------------

    #[test]
    fn test_parse_array_simple() {
        // *2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n
        let buf = b"*2\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Array(elements)) => {
                assert_eq!(elements.len(), 2);
                match &elements[0] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"foo"),
                    other => panic!("expected BulkString(foo), got {:?}", other),
                }
                match &elements[1] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"bar"),
                    other => panic!("expected BulkString(bar), got {:?}", other),
                }
            }
            other => panic!("expected Array, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_parse_array_empty() {
        let buf = b"*0\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Array(elements)) => assert_eq!(elements.len(), 0),
            other => panic!("expected empty Array, got {:?}", other),
        }
        assert_eq!(pos, 4);
    }

    #[test]
    fn test_parse_array_null() {
        let buf = b"*-1\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::BulkString(None)) => {}
            other => panic!("expected BulkString(None) for null array, got {:?}", other),
        }
        assert_eq!(pos, 5);
    }

    #[test]
    fn test_parse_array_mixed_types() {
        // array of: simple string, integer, bulk string
        // *3\r\n+hello\r\n:42\r\n$3\r\nfoo\r\n
        let buf = b"*3\r\n+hello\r\n:42\r\n$3\r\nfoo\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Array(elements)) => {
                assert_eq!(elements.len(), 3);
                match &elements[0] {
                    RespValue::SimpleString(s) => assert_eq!(s, "hello"),
                    other => panic!("expected SimpleString, got {:?}", other),
                }
                match &elements[1] {
                    RespValue::Integer(n) => assert_eq!(*n, 42),
                    other => panic!("expected Integer, got {:?}", other),
                }
                match &elements[2] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"foo"),
                    other => panic!("expected BulkString, got {:?}", other),
                }
            }
            other => panic!("expected Array, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_parse_array_nested() {
        // *2\r\n*2\r\n:1\r\n:2\r\n*2\r\n:3\r\n:4\r\n
        // array of two arrays: [[1, 2], [3, 4]]
        let buf = b"*2\r\n*2\r\n:1\r\n:2\r\n*2\r\n:3\r\n:4\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Array(outer)) => {
                assert_eq!(outer.len(), 2);
                match &outer[0] {
                    RespValue::Array(inner) => {
                        assert_eq!(inner.len(), 2);
                        match (&inner[0], &inner[1]) {
                            (RespValue::Integer(1), RespValue::Integer(2)) => {}
                            other => panic!("expected [1, 2], got {:?}", other),
                        }
                    }
                    other => panic!("expected inner array, got {:?}", other),
                }
                match &outer[1] {
                    RespValue::Array(inner) => {
                        assert_eq!(inner.len(), 2);
                        match (&inner[0], &inner[1]) {
                            (RespValue::Integer(3), RespValue::Integer(4)) => {}
                            other => panic!("expected [3, 4], got {:?}", other),
                        }
                    }
                    other => panic!("expected inner array, got {:?}", other),
                }
            }
            other => panic!("expected nested Array, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_parse_array_incomplete() {
        // says 3 elements, only 2 present
        let buf = b"*3\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::Incomplete) => {}
            other => panic!("expected Incomplete, got {:?}", other),
        }
        assert_eq!(pos, 0); // full rollback
    }

    // -- Edge cases ----------------------------------------

    #[test]
    fn test_parse_invalid_type_byte() {
        let buf = b"!garbage\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::InvalidType(b'!')) => {}
            other => panic!("expected InvalidType('!'), got {:?}", other),
        }
    }

    #[test]
    fn test_parse_empty_buffer() {
        let buf = b"";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Err(ProtocolError::Incomplete) => {}
            other => panic!("expected Incomplete, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_realistic_get_command() {
        // what redis-cli actually sends for GET foo
        let buf = b"*2\r\n$3\r\nGET\r\n$3\r\nfoo\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Array(elements)) => {
                assert_eq!(elements.len(), 2);
                match &elements[0] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"GET"),
                    other => panic!("expected BulkString(GET), got {:?}", other),
                }
                match &elements[1] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"foo"),
                    other => panic!("expected BulkString(foo), got {:?}", other),
                }
            }
            other => panic!("expected Array, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }

    #[test]
    fn test_parse_realistic_set_command() {
        // SET foo bar
        let buf = b"*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        let mut pos = 0;
        let result = parse_value(buf, &mut pos);

        match result {
            Ok(RespValue::Array(elements)) => {
                assert_eq!(elements.len(), 3);
                match &elements[0] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"SET"),
                    other => panic!("expected BulkString(SET), got {:?}", other),
                }
                match &elements[1] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"foo"),
                    other => panic!("expected BulkString(foo), got {:?}", other),
                }
                match &elements[2] {
                    RespValue::BulkString(Some(b)) => assert_eq!(&b[..], b"bar"),
                    other => panic!("expected BulkString(bar), got {:?}", other),
                }
            }
            other => panic!("expected Array, got {:?}", other),
        }
        assert_eq!(pos, buf.len());
    }
}