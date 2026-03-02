## Phases 
1. In-memory Store with `DashMap` (GET/SET/DEL)
2. TCP server with `tokio`
3. RESP protocol parser
4. Connect protocol -> storage
5. TTL / Key expiration
6. Persistence (snapshot or AOF)
7. Benchmarking with `redis-benchmark`