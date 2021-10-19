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
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: Ping) -> usize {
        self.count += msg.0;
        self.count
    }
}

#[xactor::main]
async fn main() -> Result<()> {
    // start new actor
    let addr = MyActor { count: 10 }.start().await?;

    let sender: Caller<Ping> = addr.caller();

    let res = sender.call(Ping(10)).await?;
    println!("RESULT: {}", res == 20);

    std::mem::drop(addr);

    let res = sender.call(Ping(10)).await?;
    println!("RESULT: {}", res == 20);

    Ok(())
}
