# Xactor is a rust actors framework based on async-std

## Documentation

* [GitHub repository](https://github.com/sunli829/xactor)
* [Cargo package](https://crates.io/crates/xactor)
* Minimum supported Rust version: 1.39 or later

## Features

* Async/Sync actors.
* Actor communication in a local/thread context.
* Using Futures for asynchronous message handling.
* Typed messages (No `Any` type). Generic messages are allowed.

## Performance

**Actix vs. Xactor**

|        |Wait for response|Send only|
|--------|-----------------|---------|
|Actix   |          1548 ms|    14 ms|
|Xactor  |           930 ms|    30 ms|

_Code:_ https://github.com/sunli829/xactor-benchmarks

## References

* [Actix](https://github.com/actix/actix)
* [Async-std](https://github.com/async-rs/async-std)