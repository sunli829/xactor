use crate::addr::ActorEvent;
use crate::runtime::spawn;
use crate::system::System;
use crate::{Actor, Addr, Context};
use futures::lock::Mutex;
use futures::StreamExt;
use std::sync::Arc;

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
    /// use async_std::task;
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
    ///     async fn handle(&mut self, ctx: &Context<Self>, _: Add) {
    ///         self.0 += 1;
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Get> for MyActor {
    ///     async fn handle(&mut self, ctx: &Context<Self>, _: Get) -> i32 {
    ///         self.0
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Die> for MyActor {
    ///     async fn handle(&mut self, ctx: &Context<Self>, _: Die) {
    ///         ctx.stop(None);
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<()> {
    ///     let mut addr = Supervisor::start(|| MyActor(0)).await;
    ///
    ///     addr.send(Add)?;
    ///     assert_eq!(addr.call(Get).await?, 1);
    ///
    ///     addr.send(Add)?;
    ///     assert_eq!(addr.call(Get).await?, 2);
    ///
    ///     addr.send(Die)?;
    ///     task::sleep(Duration::from_secs(1)).await; // Wait for actor restart
    ///
    ///     assert_eq!(addr.call(Get).await?, 0);
    ///     Ok(())
    /// }
    /// ```
    pub async fn start<A, F>(f: F) -> Addr<A>
    where
        A: Actor,
        F: Fn() -> A + Send + 'static,
    {
        System::inc_count();

        let (ctx, mut rx) = Context::new();
        let addr = ctx.address();

        // Create the actor
        let mut actor = Arc::new(Mutex::new(f()));

        // Call started
        actor.lock().await.started(&ctx).await;

        spawn({
            async move {
                loop {
                    while let Some(event) = rx.next().await {
                        match event {
                            ActorEvent::Exec(f) => f(actor.clone(), ctx.clone()).await,
                            ActorEvent::Stop(_err) => break,
                        }
                    }

                    actor.lock().await.stopped(&ctx).await;

                    actor = Arc::new(Mutex::new(f()));
                    actor.lock().await.started(&ctx).await;
                }
            }
        });

        addr
    }
}
