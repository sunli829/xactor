use std::time::{Duration, Instant};
use xactor::{message, Actor, Context, Handler};

#[derive(Debug)]
pub struct PingTimer {
    last_ping: Instant,
}

impl Default for PingTimer {
    fn default() -> Self {
        PingTimer {
            last_ping: Instant::now(),
        }
    }
}

#[async_trait::async_trait]
impl Actor for PingTimer {
    async fn started(&mut self, ctx: &mut Context<Self>) -> xactor::Result<()> {
        println!("PingTimer:: started()");
        ctx.send_interval(Ping, Duration::from_millis(1000));
        Ok(())
    }

    /// Called after an actor is stopped.
    async fn stopped(&mut self, _: &mut Context<Self>) {
        println!("PingTimer:: stopped()");
    }
}

#[message]
#[derive(Clone)]
struct Ping;

#[async_trait::async_trait]
impl Handler<Ping> for PingTimer {
    async fn handle(&mut self, ctx: &mut Context<Self>, _msg: Ping) {
        let now = Instant::now();
        let delta = (now - self.last_ping).as_millis();
        self.last_ping = now;
        println!("PingTimer:: Ping {} {:?}", ctx.actor_id(), delta);
    }
}
#[message]
struct Halt;

#[async_trait::async_trait]
impl Handler<Halt> for PingTimer {
    async fn handle(&mut self, ctx: &mut Context<Self>, _msg: Halt) {
        println!("PingTimer:: received Halt");
        ctx.stop(None);
        println!("PingTimer:: stopped");
    }
}

#[message]
struct Panic;

#[async_trait::async_trait]
impl Handler<Panic> for PingTimer {
    async fn handle(&mut self, _: &mut Context<Self>, _msg: Panic) {
        println!("PingTimer:: received Panic");
        panic!("intentional panic");
    }
}

#[xactor::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let service_supervisor = xactor::Supervisor::start(PingTimer::default).await?;
    let service_addr = service_supervisor.clone();

    let supervisor_task = xactor::spawn(async {
        service_supervisor.wait_for_stop().await;
    });

    let send_halt = async {
        xactor::sleep(Duration::from_millis(5_200)).await;
        println!("  main  :: sending Halt");
        service_addr.send(Halt).unwrap();
    };

    futures::join!(supervisor_task, send_halt);
    // run this to see that the interval is not properly stopped if the ctx is stopped
    // futures::join!(supervisor_task, send_panic); // there is no panic recovery

    Ok(())
}
