use std::time::Duration;
use xactor::{message, Actor, Context, Handler};

#[derive(Debug, Default)]
pub struct PingLater;

#[async_trait::async_trait]
impl Actor for PingLater {
    async fn started(&mut self, ctx: &mut Context<Self>) -> xactor::Result<()> {
        ctx.send_later(Ping("after halt"), Duration::from_millis(1_500));

        Ok(())
    }
    /// Called after an actor is stopped.
    async fn stopped(&mut self, _: &mut Context<Self>) {
        println!("PingLater:: stopped()");
    }
}

#[message]
#[derive(Debug)]
struct Ping(&'static str);

#[async_trait::async_trait]
impl Handler<Ping> for PingLater {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: Ping) {
        println!("PingLater:: handle {:?}", msg);
    }
}
#[message]
struct Halt;

#[async_trait::async_trait]
impl Handler<Halt> for PingLater {
    async fn handle(&mut self, ctx: &mut Context<Self>, _msg: Halt) {
        println!("PingLater:: received Halt");
        ctx.stop(None);
        println!("PingLater:: stopped");
    }
}

#[xactor::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service_supervisor = xactor::Supervisor::start(PingLater::default).await?;
    let service_addr = service_supervisor.clone();

    let supervisor_task = xactor::spawn(async {
        service_supervisor.wait_for_stop().await;
    });

    let send_ping = async {
        println!("  main  :: sending Ping");
        service_addr.send(Ping("before halt")).unwrap();
    };

    let send_halt = async {
        xactor::sleep(Duration::from_millis(1_000)).await;
        println!("  main  :: sending Halt");
        service_addr.send(Halt).unwrap();
    };

    let _ = futures::join!(supervisor_task, send_halt, send_ping);

    Ok(())
}
