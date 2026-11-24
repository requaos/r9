//! tpht — the succinct concurrent hash table: `FlattenedTphtShard`,
//! `ChainedTphtOverflow`, Feistel quotienting and `deref_tiny` (DS-01 to
//! DS-05).  This is the one module that may hold `unsafe` in Phase 3
//! (Phase 3 CONTEXT D-09); the implementation arrives in plan 03-06.
