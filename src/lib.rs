//! Linux shared memory futex for Rust
//! implementation based on https://eli.thegreenplace.net/2018/basics-of-futexes/
//!
//! [`rufutex`]: https://github.com/yangosoft/rufutex
//! YangoSoft

pub mod rufutex;

const UNLOCKED: u32 = 0;
const LOCKED_NO_WAITERS: u32 = 1;
const LOCKED_WAITERS: u32 = 2;
const INVALID_FD: i64 = -1;
