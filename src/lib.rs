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
//!     async fn handle(&mut self, _ctx: &Context<Self>, msg: ToUppercase) -> String {
//!         msg.0.to_uppercase()
//!     }
//! }
//!
//! #[async_std::main]
//! async fn main() -> Result<()> {
//!     // Start actor and get its address
//!     let mut addr = MyActor.start().await;
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

mod actor;
mod addr;
mod broker;
mod caller;
mod context;
mod service;
mod supervisor;
mod system;

/// Alias of anyhow::Result
pub type Result<T> = anyhow::Result<T>;

/// Alias of anyhow::Error
pub type Error = anyhow::Error;

pub use actor::{Actor, Handler, Message, StreamHandler};
pub use addr::Addr;
pub use broker::Broker;
pub use caller::{Caller, Sender};
pub use context::Context;
pub use service::{LocalService, Service};
pub use supervisor::Supervisor;
pub use system::System;
pub use xactor_derive::{main, message};
