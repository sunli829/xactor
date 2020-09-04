# Xactor is a rust actors framework based on async-std

<div align="center">
  <!-- CI -->
  <img src="https://github.com/sunli829/potatonet/workflows/CI/badge.svg" />
  <!-- Crates version -->
  <a href="https://crates.io/crates/xactor">
    <img src="https://img.shields.io/crates/v/xactor.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/xactor">
    <img src="https://img.shields.io/crates/d/xactor.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs.rs docs -->
  <a href="https://docs.rs/xactor">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

## Documentation

* [GitHub repository](https://github.com/sunli829/xactor)
* [Cargo package](https://crates.io/crates/xactor)
* Minimum supported Rust version: 1.39 or later

## Features

* Async actors.
* Actor communication in a local context.
* Using Futures for asynchronous message handling.
* Typed messages (No `Any` type). Generic messages are allowed.

## Examples

```rust
use xactor::*;

#[message(result = "String")]
struct ToUppercase(String);

struct MyActor;

impl Actor for MyActor {}

#[async_trait::async_trait]
impl Handler<ToUppercase> for MyActor {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: ToUppercase) -> String {
        msg.0.to_uppercase()
    }
}

#[xactor::main]
async fn main() -> Result<()> {
    // Start actor and get its address
    let mut addr = MyActor.start().await?;

    // Send message `ToUppercase` to actor via addr
    let res = addr.call(ToUppercase("lowercase".to_string())).await?;
    assert_eq!(res, "LOWERCASE");
    Ok(())
}
```

## Performance

|        |Wait for response|Send only|
|--------|-----------------|---------|
|Actix   |          1548 ms|    14 ms|
|Xactor  |           930 ms|    18 ms|

[GitHub repository](https://github.com/sunli829/xactor-benchmarks)

## Installation

Xactor requires [async-trait](https://github.com/dtolnay/async-trait) on userland.

With [cargo add][cargo-add] installed, run:

```sh
$ cargo add xactor
$ cargo add async-trait
```

We also provide the [tokio](https://tokio.rs/) runtime instead of [async-std](https://async.rs/). To use it, you need to activate `runtime-tokio` and disable default features.

You can edit your `Cargo.toml` as follows:
```toml
xactor = { version = "x.x.x", features = ["runtime-tokio"], default-features = false }
```

[cargo-add]: https://github.com/killercup/cargo-edit

## References

* [Actix](https://github.com/actix/actix)
* [Async-std](https://github.com/async-rs/async-std)
* [Tokio](https://github.com/tokio-rs/tokio)
