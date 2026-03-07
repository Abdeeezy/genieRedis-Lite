// to essentially "match with real Redis" we'll implement persistance in the following way:
//      - RDB snapshots for periodic full-state dumps (compact, fast recovery)
//          - serialize/deserialize the store, save/load to disk
//      - AOF for write-ahead logging between snapshots (durability)
//          - append write commands, replay on startup
//      - On startup: if AOF exists, replay it (it's more complete). If not, load the RDB snapshot.