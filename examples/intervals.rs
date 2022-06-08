use std::time::Duration;
use xactor::*;

#[message]
struct IntervalMsg;

struct MyActor;

#[async_trait::async_trait]
impl Actor for MyActor {
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        // Send the IntervalMsg message 3 seconds later
        ctx.send_later(IntervalMsg, Duration::from_millis(500));
        Ok(())
    }
}

#[async_trait::async_trait]
impl Handler<IntervalMsg> for MyActor {
    async fn handle(&mut self, ctx: &mut Context<Self>, _msg: IntervalMsg) {
        ctx.send_later(IntervalMsg, Duration::from_millis(500));
    }
}

#[xactor::main]
async fn main() -> Result<()> {
    // Exit the program after 3 seconds
    let addr = MyActor.start().await?;
    addr.wait_for_stop().await;
    Ok(())
}
