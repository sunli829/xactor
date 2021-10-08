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
//! * Async actors.
//! * Actor communication in a local context.
//! * Using Futures for asynchronous message handling.
//! * Typed messages (No `Any` type). Generic messages are allowed.
//!
//! ## Examples
//!
//! ```rust
//! use xactor::*;
//!
//! #[message(result = "String")]
//! struct ToUppercase(String);
//!
//! struct MyActor;
//!
//! impl Actor for MyActor {}
//!
//! #[async_trait::async_trait]
//! impl Handler<ToUppercase> for MyActor {
//!     async fn handle(&mut self, _ctx: &mut Context<Self>, msg: ToUppercase) -> String {
//!         msg.0.to_uppercase()
//!     }
//! }
//!
//! #[xactor::main]
//! async fn main() -> Result<()> {
//!     // Start actor and get its address
//!     let mut addr = MyActor.start().await?;
//!
//!     // Send message `ToUppercase` to actor via addr
//!     let res = addr.call(ToUppercase("lowercase".to_string())).await?;
//!     assert_eq!(res, "LOWERCASE");
//!     Ok(())
//! }
//! ```
//!
//! ## Performance
//!
//! **Actix vs. Xactor**
//!
//! |        |Wait for response|Send only|
//! |--------|-----------------|---------|
//! |Actix   |          1548 ms|    14 ms|
//! |Xactor  |           930 ms|    18 ms|
//!
//! [GitHub repository](https://github.com/sunli829/xactor-benchmarks)
//!
//! ## References
//!
//! * [Actix](https://github.com/actix/actix)
//! * [Async-std](https://github.com/async-rs/async-std)

#![allow(clippy::type_complexity)]

mod actor;
mod addr;
mod broker;
mod caller;
mod context;
mod runtime;
mod service;
mod supervisor;
mod lifecycle;

#[cfg(all(feature = "anyhow", feature = "eyre"))]
compile_error!(r#"
    features `xactor/anyhow` and `xactor/eyre` are mutually exclusive.
    If you are trying to disable anyhow set `default-features = false`.
"#);

#[cfg(feature="anyhow")]
pub use anyhow as error;

#[cfg(feature="eyre")]
pub use eyre as error;

/// Alias of error::Result
pub type Result<T> = error::Result<T>;

/// Alias of error::Error
pub type Error = error::Error;

pub type ActorId = u64;

pub use actor::{Actor, Handler, Message, StreamHandler};
pub use addr::{Addr, WeakAddr};
pub use broker::Broker;
pub use caller::{Caller, Sender};
pub use context::Context;
pub use runtime::{block_on, sleep, spawn, timeout};
pub use service::{LocalService, Service};
pub use supervisor::Supervisor;
pub use xactor_derive::{main, message};
