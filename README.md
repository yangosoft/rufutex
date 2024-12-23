# rufutex

![rufutex workflow](https://github.com/yangosoft/rufutex/actions/workflows/rust.yml/badge.svg)
[![crates.io](https://img.shields.io/crates/v/rufutex.svg)](https://crates.io/crates/rufutex)
[![documentation](https://img.shields.io/badge/docs-live-brightgreen)](https://docs.rs/rufutex)

Ulrich Drepper's mutex using futex implementation in Rust.

Based on [Eli Bendersky Mutex https://eli.thegreenplace.net/2018/basics-of-futexes/](https://eli.thegreenplace.net/2018/basics-of-futexes/) implementation of the [Ulrich Drepper's Futexes are Tricky paper](https://www.akkadia.org/drepper/futex.pdf)

Examples:

See [rufutex-example.rs](examples/rufutex-example.rs)
