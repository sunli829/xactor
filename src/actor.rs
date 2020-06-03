use crate::addr::ActorEvent;
use crate::runtime::spawn;
use crate::{Addr, Context};
use futures::channel::mpsc::UnboundedReceiver;
use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::{FutureExt, StreamExt};
use std::sync::Arc;

/// Represents a message that can be handled by the actor.
pub trait Message: 'static + Send {
    /// The return value type of the message
    /// This type can be set to () if the message does not return a value, or if it is a notification message
    type Result: 'static + Send;
}

/// Describes how to handle messages of a specific type.
/// Implementing Handler is a general way to handle incoming messages.
/// The type T is a message which can be handled by the actor.
#[async_trait::async_trait]
pub trait Handler<T: Message>: Actor {
    /// Method is called for every message received by this Actor.
    async fn handle(&mut self, ctx: &Context<Self>, msg: T) -> T::Result;
}

/// Describes how to handle messages of a specific type.
/// Implementing Handler is a general way to handle incoming streams.
/// The type T is a stream message which can be handled by the actor.
/// Stream messages do not need to implement the `Message` trait.
#[async_trait::async_trait]
#[allow(unused_variables)]
pub trait StreamHandler<T: 'static>: Actor {
    /// Method is called for every message received by this Actor.
    async fn handle(&mut self, ctx: &Context<Self>, msg: T);

    /// Method is called when stream get polled first time.
    async fn started(&mut self, ctx: &Context<Self>) {}

    /// Method is called when stream finishes.
    ///
    /// By default this method stops actor execution.
    async fn finished(&mut self, ctx: &Context<Self>) {
        ctx.stop(None);
    }
}

/// Actors are objects which encapsulate state and behavior.
/// Actors run within a specific execution context `Context<A>`.
/// The context object is available only during execution.
/// Each actor has a separate execution context.
///
/// Roles communicate by exchanging messages.
/// The requester can wait for a response.
/// By `Addr` referring to the actors, the actors must provide an `Handle<T>` implementation for this message.
/// All messages are statically typed.
#[async_trait::async_trait]
#[allow(unused_variables)]
pub trait Actor: Sized + Send + 'static {
    /// Called when the actor is first started.
    async fn started(&mut self, ctx: &Context<Self>) {}

    /// Called after an actor is stopped.
    async fn stopped(&mut self, ctx: &Context<Self>) {}

    /// Construct and start a new actor, returning its address.
    ///
    /// This is constructs a new actor using the `Default` trait, and invokes its `start` method.
    async fn start_default() -> Addr<Self>
    where
        Self: Default,
    {
        Self::default().start().await
    }

    /// Start a new actor, returning its address.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use xactor::*;
    ///
    /// struct MyActor;
    ///
    /// impl Actor for MyActor {}
    ///
    /// #[message(result = "i32")]
    /// struct MyMsg(i32);
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<MyMsg> for MyActor {
    ///     async fn handle(&mut self, _ctx: &Context<Self>, msg: MyMsg) -> i32 {
    ///         msg.0 * msg.0
    ///     }
    /// }
    ///
    /// #[xactor::main]
    /// async fn main() -> Result<()> {
    ///     // Start actor and get its address
    ///     let mut addr = MyActor.start().await;
    ///
    ///     // Send message `MyMsg` to actor via addr
    ///     let res = addr.call(MyMsg(10)).await?;
    ///     assert_eq!(res, 100);
    ///     Ok(())
    /// }
    /// ```
    async fn start(self) -> Addr<Self> {
        let (tx_exit, rx_exit) = oneshot::channel();
        let rx_exit = rx_exit.shared();
        let (ctx, rx) = Context::new(Some(rx_exit));
        start_actor(ctx.clone(), rx, tx_exit, self).await;
        ctx.address()
    }
}

pub(crate) async fn start_actor<A: Actor>(
    ctx: Arc<Context<A>>,
    mut rx: UnboundedReceiver<ActorEvent<A>>,
    tx_exit: oneshot::Sender<()>,
    actor: A,
) {
    let actor = Arc::new(Mutex::new(actor));

    // Call started
    actor.lock().await.started(&ctx).await;

    spawn({
        async move {
            while let Some(event) = rx.next().await {
                match event {
                    ActorEvent::Exec(f) => f(actor.clone(), ctx.clone()).await,
                    ActorEvent::Stop(_err) => break,
                }
            }

            actor.lock().await.stopped(&ctx).await;
            tx_exit.send(()).ok();
        }
    });
}
