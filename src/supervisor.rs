use crate::addr::ActorEvent;
use crate::runtime::spawn;
use crate::{Actor, Addr, Context};
use anyhow::Result;
use futures::StreamExt;

/// Actor supervisor
///
/// Supervisor gives the actor the ability to restart after failure.
/// When the actor fails, recreate a new actor instance and replace it.
pub struct Supervisor;

impl Supervisor {
    /// Start a supervisor
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xactor::*;
    /// use std::time::Duration;
    ///
    /// #[message]
    /// struct Die;
    ///
    /// #[message]
    /// struct Add;
    ///
    /// #[message(result = "i32")]
    /// struct Get;
    ///
    /// struct MyActor(i32);
    ///
    /// impl Actor for MyActor {}
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Add> for MyActor {
    ///     async fn handle(&mut self, ctx: &mut Context<Self>, _: Add) {
    ///         self.0 += 1;
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Get> for MyActor {
    ///     async fn handle(&mut self, ctx: &mut Context<Self>, _: Get) -> i32 {
    ///         self.0
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Die> for MyActor {
    ///     async fn handle(&mut self, ctx: &mut Context<Self>, _: Die) {
    ///         ctx.stop(None);
    ///     }
    /// }
    ///
    /// #[xactor::main]
    /// async fn main() -> Result<()> {
    ///     let mut addr = Supervisor::start(|| MyActor(0)).await?;
    ///
    ///     addr.send(Add)?;
    ///     assert_eq!(addr.call(Get).await?, 1);
    ///
    ///     addr.send(Add)?;
    ///     assert_eq!(addr.call(Get).await?, 2);
    ///
    ///     addr.send(Die)?;
    ///     sleep(Duration::from_secs(1)).await; // Wait for actor restart
    ///
    ///     assert_eq!(addr.call(Get).await?, 0);
    ///     Ok(())
    /// }
    /// ```
    pub async fn start<A, F>(f: F) -> Result<Addr<A>>
    where
        A: Actor,
        F: Fn() -> A + Send + 'static,
    {
        let (mut ctx, mut rx, tx) = Context::new(None);
        let addr = Addr {
            actor_id: ctx.actor_id(),
            tx,
            rx_exit: ctx.rx_exit.clone(),
        };

        // Create the actor
        let mut actor = f();

        // Call started
        actor.started(&mut ctx).await?;

        spawn({
            async move {
                'restart_loop: loop {
                    'event_loop: loop {
                        match rx.next().await {
                            None => break 'restart_loop,
                            Some(ActorEvent::Stop(_err)) => break 'event_loop,
                            Some(ActorEvent::Exec(f)) => f(&mut actor, &mut ctx).await,
                            Some(ActorEvent::RemoveStream(id)) => {
                                if ctx.streams.contains(id) {
                                    ctx.streams.remove(id);
                                }
                            }
                        }
                    }

                    actor.stopped(&mut ctx).await;
                    actor = f();
                    actor.started(&mut ctx).await.ok();
                }

                actor.stopped(&mut ctx).await;
            }
        });

        Ok(addr)
    }
}
