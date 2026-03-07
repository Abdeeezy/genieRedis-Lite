
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


#[derive(Debug, Error, PartialEq)]
pub enum ProtocolError {
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
#[derive(Debug, PartialEq)] //for the in-line tests/asserts at the bottom of this file
pub enum Command {
    Ping,                                          // health-checking server purposes - no args
    Get { key: String },                           // named-field, accessed by named
    Set { key: String, value: Bytes, ttl: Option<Duration> }, // 2-3 args
    Del { key: String },                           
    Exists { key: String },                        
}

// RESP wire format vocabulary - used in both directions of the client/server
//   RespValue - struct-like enum variant
#[derive(Debug, PartialEq)] //for the in-line tests/asserts at the bottom of this file
pub enum RespValue {
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

// function-dispatcher - with recursion-possibility inside the parse_array
pub fn parse_value(buf: &[u8], pos: &mut usize) -> Result<RespValue, ProtocolError>{

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
// Redis clients always send commands as an array of bulk-strings
//    raw bytes  →  parse_value()  →  RespValue  →  parse_command()  →  Command

pub fn parse_command(value: RespValue) -> Result<Command, ProtocolError> {
    
    // example array: ["SET", "foo", "bar"]


    // ensure array of RespValues, force it into this parts variable.
    let parts = match value {
        RespValue::Array(parts) => parts,                           
        _ => return Err(ProtocolError::InvalidFormat(("expected array").into())),   
    };
    
    // transform vec<RespValue> into vec<Bytes>
    let byte_parts: Vec<Bytes> = parts.into_iter().map( 
        // test each part by matching against a bulkstring, declaring each layer and if bytes could be extracted, then result in a success "Ok(bytes)"
        |part| 
        match part {
            RespValue::BulkString(Some(b)) => Ok(b),
        _ => Err(ProtocolError::InvalidFormat("expected bulk string".into())),
    })
    .collect::<Result<Vec<Bytes>, ProtocolError>>()?; //? at the end propagates the first error if any element wasn't a valid bulk string. forcing either a vector-of-bytes or just an error
                                                        //The ? operator says: "if this is an Err, return it from the function immediately. If it's Ok, unwrap the value and keep going"
 


    // isolate the command-name (first-item) from the rest of the elements
    let (command_name, args) = byte_parts.split_first()
        .ok_or(ProtocolError::InvalidFormat("empty command attempt".into()))?; // if it's okay, keep it pushing, otherwise: propagate error..
         //.ok_or()? pattern converts None into my error-type and propagates it 

    // convert the bytes into a string, propagate error if found..
    let command_name_str = String::from_utf8(command_name.to_vec())
        .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?
        .to_uppercase(); //uppercase it if successful.

    

    // check command-type and the args supplied
    match command_name_str.as_str() {
        "PING" => { 
            // no args needed, if there are arguments, disregard them.. 
            return Ok(Command::Ping);
        }
        "GET" => { 
            // return error if arg-count wrong, otherwise parse arg
            if args.len() != 1
            {
                return Err(ProtocolError::WrongArgCount { command: "GET".to_string(), expected: 1, got: args.len() })
            }
            else
            {
                // convert the bytes into a string, propagate error if found..
                //  `String::from_utf8` takes `Vec<u8>` not `&[u8]`, it also wants ownership
                //  `std::str::from_utf8` takes `&[u8]`, and it just borrows. 
                let key = std::str::from_utf8(&args[0])
                    .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?
                    .to_string();

                return Ok(Command::Get { key });
            }
        }
        "SET" => { 
            // return error if arg-count wrong, otherwise parse arg
            if args.len() != 2 && args.len() != 4
            {
                return Err(ProtocolError::WrongArgCount { command: "SET".to_string(), expected: 2, got: args.len() })
            }
            else
            {
                // convert the bytes into a string, propagate error if found..
                //  `String::from_utf8` takes `Vec<u8>` not `&[u8]`, it also wants ownership
                //  `std::str::from_utf8` takes `&[u8]`, and it just borrows. 
                let key = std::str::from_utf8(&args[0])
                    .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))? 
                    .to_string();
                
                // if TIME-TO-LIVE 3rd and 4th arguments supplied, extract 
                let time_to_live_value = if args.len() == 4 {
                    // Redis-standard for TTL has flags to represent EX(seconds) or PX(milliseconds)
                    let flag = std::str::from_utf8(&args[2])
                        .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))? //propagate error if invalid string
                        .to_uppercase();
                    let amount = std::str::from_utf8(&args[3])
                        .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?//propagate error if invalid string
                        .parse::<u64>()
                        .map_err(|_| ProtocolError::InvalidFormat("invalid ttl value".into()))?;// propagate error if integer-parse failed
                    match flag.as_str() {
                        "EX" => Some(Duration::from_secs(amount)), //object that handles seconds and nanoseconds.
                        "PX" => Some(Duration::from_millis(amount)),
                        _ => return Err(ProtocolError::InvalidFormat( // wrong-flag error 
                            format!("unknown SET flag: {}", flag)
                        )),
                    }
                } else {
                    None //nothing
                };

                let value_to_set = args[1].clone();
                return Ok(Command::Set { key, value: (value_to_set), ttl: time_to_live_value });
            }
        }
        "DEL" => { 
            // return error if arg-count wrong, otherwise parse arg
            if args.len() != 1
            {
                return Err(ProtocolError::WrongArgCount { command: "DEL".to_string(), expected: 1, got: args.len() })
            }
            else
            {
                // convert the bytes into a string, propagate error if found..
                //  `String::from_utf8` takes `Vec<u8>` not `&[u8]`, it also wants ownership
                //  `std::str::from_utf8` takes `&[u8]`, and it just borrows. 
                let key = std::str::from_utf8(&args[0])
                    .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?
                    .to_string();

                return Ok(Command::Del { key } );
            }
        }
        "EXISTS" => { 
            // return error if arg-count wrong, otherwise parse arg
            if args.len() != 1
            {
                return Err(ProtocolError::WrongArgCount { command: "EXISTS".to_string(), expected: 1, got: args.len() })
            }
            else
            {
                // convert the bytes into a string, propagate error if found..
                //  `String::from_utf8` takes `Vec<u8>` not `&[u8]`, it also wants ownership
                //  `std::str::from_utf8` takes `&[u8]`, and it just borrows. 
                let key = std::str::from_utf8(&args[0])
                    .map_err(|_| ProtocolError::InvalidFormat("invalid utf8".into()))?
                    .to_string();

                return Ok(Command::Exists { key } );
            }
        }
        _ => Err(ProtocolError::InvalidCommand(command_name_str)),
    }
     
}


// inverse of the parse_value function.
pub fn encode(value: &RespValue)-> Bytes{
     // parse each RespValue into their respective byte-string sequence
    match value {
        RespValue::SimpleString(str) => {
            Bytes::from(format!("+{}\r\n", str))
        },
        RespValue::Error(errStr) => {
            Bytes::from(format!("-{}\r\n", errStr))
        },
        RespValue::Integer(int) => {
            Bytes::from(format!(":{}\r\n", int))
        },
        RespValue::BulkString(None) => { 
             Bytes::from(format!("$-1\r\n"))
        },
        RespValue::BulkString(Some(data)) => {
            // turn into vec of bytes so we can append the data (postional string formatting couldn't take the Bytes)
            let mut byte_buffer = format!("${}\r\n", data.len()).into_bytes();
            byte_buffer.extend_from_slice(data);
            byte_buffer.extend_from_slice(b"\r\n");
            Bytes::from(byte_buffer)
        },
        RespValue::Array(vec) => {
            // vec of bytes so that it's easy to append the bytes retrieved from recursively calling this function
            let mut byte_buffer = format!("*{}\r\n", vec.len()).into_bytes();

            for item in vec {
                // recursively call encode(), bytes are returned of course so all that's needed is to concatenate
                byte_buffer.extend_from_slice(&encode(item));
            }

            Bytes::from(byte_buffer)
        }
    }

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


#[cfg(test)]
mod command_tests {
    use super::*;
    use bytes::Bytes;
    use std::time::Duration;

    // helper to build a BulkString RespValue
    fn bulk(s: &str) -> RespValue {
        RespValue::BulkString(Some(Bytes::from(s.to_string())))
    }

    fn cmd(args: Vec<&str>) -> RespValue {
        RespValue::Array(args.into_iter().map(bulk).collect())
    }

    // ---- parse_command tests ----

    #[test]
    fn parse_ping() {
        let result = parse_command(cmd(vec!["PING"])).unwrap();
        assert_eq!(result, Command::Ping);
    }

    #[test]
    fn parse_ping_with_extra_args() {
        let result = parse_command(cmd(vec!["PING", "hello"])).unwrap();
        assert_eq!(result, Command::Ping);
    }

    #[test]
    fn parse_ping_lowercase() {
        let result = parse_command(cmd(vec!["ping"])).unwrap();
        assert_eq!(result, Command::Ping);
    }

    #[test]
    fn parse_get_valid() {
        let result = parse_command(cmd(vec!["GET", "mykey"])).unwrap();
        assert_eq!(result, Command::Get { key: "mykey".to_string() });
    }

    #[test]
    fn parse_get_no_args() {
        let result = parse_command(cmd(vec!["GET"]));
        assert!(matches!(result, Err(ProtocolError::WrongArgCount { .. })));
    }

    #[test]
    fn parse_get_too_many_args() {
        let result = parse_command(cmd(vec!["GET", "a", "b"]));
        assert!(matches!(result, Err(ProtocolError::WrongArgCount { .. })));
    }

    #[test]
    fn parse_set_no_ttl() {
        let result = parse_command(cmd(vec!["SET", "foo", "bar"])).unwrap();
        assert_eq!(result, Command::Set {
            key: "foo".to_string(),
            value: Bytes::from("bar"),
            ttl: None,
        });
    }

    #[test]
    fn parse_set_with_ex() {
        let result = parse_command(cmd(vec!["SET", "foo", "bar", "EX", "10"])).unwrap();
        assert_eq!(result, Command::Set {
            key: "foo".to_string(),
            value: Bytes::from("bar"),
            ttl: Some(Duration::from_secs(10)),
        });
    }

    #[test]
    fn parse_set_with_px() {
        let result = parse_command(cmd(vec!["SET", "foo", "bar", "PX", "5000"])).unwrap();
        assert_eq!(result, Command::Set {
            key: "foo".to_string(),
            value: Bytes::from("bar"),
            ttl: Some(Duration::from_millis(5000)),
        });
    }

    #[test]
    fn parse_set_invalid_flag() {
        let result = parse_command(cmd(vec!["SET", "foo", "bar", "XX", "10"]));
        assert!(matches!(result, Err(ProtocolError::InvalidFormat(_))));
    }

    #[test]
    fn parse_set_wrong_arg_count() {
        let result = parse_command(cmd(vec!["SET", "foo"]));
        assert!(matches!(result, Err(ProtocolError::WrongArgCount { .. })));
    }

    #[test]
    fn parse_set_three_args_invalid() {
        let result = parse_command(cmd(vec!["SET", "foo", "bar", "EX"]));
        assert!(matches!(result, Err(ProtocolError::WrongArgCount { .. })));
    }

    #[test]
    fn parse_del_valid() {
        let result = parse_command(cmd(vec!["DEL", "mykey"])).unwrap();
        assert_eq!(result, Command::Del { key: "mykey".to_string() });
    }

    #[test]
    fn parse_exists_valid() {
        let result = parse_command(cmd(vec!["EXISTS", "mykey"])).unwrap();
        assert_eq!(result, Command::Exists { key: "mykey".to_string() });
    }

    #[test]
    fn parse_unknown_command() {
        let result = parse_command(cmd(vec!["FLUSHALL"]));
        assert!(matches!(result, Err(ProtocolError::InvalidCommand(_))));
    }

    #[test]
    fn parse_non_array_input() {
        let result = parse_command(RespValue::SimpleString("PING".to_string()));
        assert!(matches!(result, Err(ProtocolError::InvalidFormat(_))));
    }

    #[test]
    fn parse_array_with_non_bulk_string() {
        let input = RespValue::Array(vec![
            RespValue::Integer(42),
        ]);
        let result = parse_command(input);
        assert!(matches!(result, Err(ProtocolError::InvalidFormat(_))));
    }

    #[test]
    fn parse_empty_array() {
        let input = RespValue::Array(vec![]);
        let result = parse_command(input);
        assert!(matches!(result, Err(ProtocolError::InvalidFormat(_))));
    }
}

#[cfg(test)]
mod encode_tests {
    use super::*;
    use bytes::Bytes;

    #[test]
    fn encode_simple_string() {
        let val = RespValue::SimpleString("OK".to_string());
        assert_eq!(encode(&val), Bytes::from("+OK\r\n"));
    }

    #[test]
    fn encode_error() {
        let val = RespValue::Error("ERR something".to_string());
        assert_eq!(encode(&val), Bytes::from("-ERR something\r\n"));
    }

    #[test]
    fn encode_integer() {
        let val = RespValue::Integer(42);
        assert_eq!(encode(&val), Bytes::from(":42\r\n"));
    }

    #[test]
    fn encode_negative_integer() {
        let val = RespValue::Integer(-1);
        assert_eq!(encode(&val), Bytes::from(":-1\r\n"));
    }

    #[test]
    fn encode_bulk_string() {
        let val = RespValue::BulkString(Some(Bytes::from("foo")));
        assert_eq!(encode(&val), Bytes::from("$3\r\nfoo\r\n"));
    }

    #[test]
    fn encode_null_bulk_string() {
        let val = RespValue::BulkString(None);
        assert_eq!(encode(&val), Bytes::from("$-1\r\n"));
    }

    #[test]
    fn encode_empty_bulk_string() {
        let val = RespValue::BulkString(Some(Bytes::from("")));
        assert_eq!(encode(&val), Bytes::from("$0\r\n\r\n"));
    }

    #[test]
    fn encode_array() {
        let val = RespValue::Array(vec![
            RespValue::BulkString(Some(Bytes::from("SET"))),
            RespValue::BulkString(Some(Bytes::from("foo"))),
            RespValue::BulkString(Some(Bytes::from("bar"))),
        ]);
        let expected = "*3\r\n$3\r\nSET\r\n$3\r\nfoo\r\n$3\r\nbar\r\n";
        assert_eq!(encode(&val), Bytes::from(expected));
    }

    #[test]
    fn encode_empty_array() {
        let val = RespValue::Array(vec![]);
        assert_eq!(encode(&val), Bytes::from("*0\r\n"));
    }

    #[test]
    fn round_trip_simple_string() {
        let original = RespValue::SimpleString("hello".to_string());
        let encoded = encode(&original);
        let mut pos = 0;
        let decoded = parse_value(&encoded, &mut pos).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn round_trip_bulk_string() {
        let original = RespValue::BulkString(Some(Bytes::from("hello world")));
        let encoded = encode(&original);
        let mut pos = 0;
        let decoded = parse_value(&encoded, &mut pos).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn round_trip_array() {
        let original = RespValue::Array(vec![
            RespValue::Integer(1),
            RespValue::SimpleString("OK".to_string()),
            RespValue::BulkString(Some(Bytes::from("data"))),
        ]);
        let encoded = encode(&original);
        let mut pos = 0;
        let decoded = parse_value(&encoded, &mut pos).unwrap();
        assert_eq!(original, decoded);
    }
}