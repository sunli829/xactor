//! # Xactor is a rust actors framework based on async-std
//!
//! ## Documentation
//!
//! * [GitHub repository](https://github.com/sunli829/xactor)
//! * [Cargo package](https://crates.io/crates/xactor)
//! * Minimum supported Rust version: 1.39 or later
//!
//! ## Features
//!
//! * Async/Sync actors.
//! * Actor communication in a local/thread context.
//! * Using Futures for asynchronous message handling.
//! * Typed messages (No `Any` type). Generic messages are allowed.
//!
//! ## Performance
//!
//! *Actix vs. xactor*
//!
//! |        |Wait for response|Send only|
//! |--------|-----------------|---------|
//! |Actix   |          1548 ms|    14 ms|
//! |Xactor  |           930 ms|    30 ms|
//!
//! **Code** https://github.com/sunli829/xactor
//!
//! ## References
//!
//! * [Actix](https://github.com/actix/actix)
//! * [Async-std](https://github.com/async-rs/async-std)

mod actor;
mod addr;
mod caller;
mod context;
mod service;

pub use actor::{Actor, Handler, Message};
pub use addr::Addr;
pub use caller::{Caller, Sender};
pub use context::Context;
pub use service::Service;
