use xactor::*;

/// Define `Ping` message
#[message(result = "usize")]
struct Ping(usize);

/// Actor
struct MyActor {
    count: usize,
}

/// Declare actor and its context
impl Actor for MyActor {}

/// Handler for `Ping` message
#[async_trait::async_trait]
impl Handler<Ping> for MyActor {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Ping) -> usize {
        self.count += msg.0;
        self.count
    }
}

#[async_std::main]
async fn main() -> Result<()> {
    // start new actor
    let mut addr = MyActor { count: 10 }.start().await;

    // send message and get future for result
    let res = addr.call(Ping(10)).await?;
    println!("RESULT: {}", res == 20);

    Ok(())
}
