use crate::addr::ActorEvent;
use crate::runtime::spawn;
use crate::{Addr, Context};
use crate::error::Result;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use futures::{FutureExt, StreamExt};

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
pub trait Handler<T: Message>: Actor
where
    Self: std::marker::Sized,
{
    /// Method is called for every message received by this Actor.
    async fn handle(&mut self, ctx: &mut Context<Self>, msg: T) -> T::Result;
}

/// Describes how to handle messages of a specific type.
/// Implementing Handler is a general way to handle incoming streams.
/// The type T is a stream message which can be handled by the actor.
/// Stream messages do not need to implement the `Message` trait.
#[async_trait::async_trait]
#[allow(unused_variables)]
pub trait StreamHandler<T: 'static>: Actor {
    /// Method is called for every message received by this Actor.
    async fn handle(&mut self, ctx: &mut Context<Self>, msg: T);

    /// Method is called when stream get polled first time.
    async fn started(&mut self, ctx: &mut Context<Self>) {}

    /// Method is called when stream finishes.
    ///
    /// By default this method stops actor execution.
    async fn finished(&mut self, ctx: &mut Context<Self>) {
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
    async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()> {
        Ok(())
    }

    /// Called after an actor is stopped.
    async fn stopped(&mut self, ctx: &mut Context<Self>) {}

    /// Construct and start a new actor, returning its address.
    ///
    /// This is constructs a new actor using the `Default` trait, and invokes its `start` method.
    async fn start_default() -> Result<Addr<Self>>
    where
        Self: Default,
    {
        Ok(Self::default().start().await?)
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
    ///     async fn handle(&mut self, _ctx: &mut Context<Self>, msg: MyMsg) -> i32 {
    ///         msg.0 * msg.0
    ///     }
    /// }
    ///
    /// #[xactor::main]
    /// async fn main() -> Result<()> {
    ///     // Start actor and get its address
    ///     let mut addr = MyActor.start().await?;
    ///
    ///     // Send message `MyMsg` to actor via addr
    ///     let res = addr.call(MyMsg(10)).await?;
    ///     assert_eq!(res, 100);
    ///     Ok(())
    /// }
    /// ```
    async fn start(self) -> Result<Addr<Self>> {
        ActorManager::new().start_actor(self).await
    }
}

pub(crate) struct ActorManager<A: Actor> {
    ctx: Context<A>,
    tx: std::sync::Arc<UnboundedSender<ActorEvent<A>>>,
    rx: UnboundedReceiver<ActorEvent<A>>,
    tx_exit: oneshot::Sender<()>,
}

impl<A: Actor> ActorManager<A> {
    pub(crate) fn new() -> Self {
        let (tx_exit, rx_exit) = oneshot::channel();
        let rx_exit = rx_exit.shared();
        let (ctx, rx, tx) = Context::new(Some(rx_exit));
        Self {
            ctx,
            rx,
            tx,
            tx_exit,
        }
    }

    pub(crate) fn address(&self) -> Addr<A> {
        self.ctx.address()
    }

    pub(crate) async fn start_actor(self, mut actor: A) -> Result<Addr<A>> {
        let Self {
            mut ctx,
            mut rx,
            tx,
            tx_exit,
        } = self;

        let rx_exit = ctx.rx_exit.clone();
        let actor_id = ctx.actor_id();

        // Call started
        actor.started(&mut ctx).await?;

        spawn({
            async move {
                while let Some(event) = rx.next().await {
                    match event {
                        ActorEvent::Exec(f) => f(&mut actor, &mut ctx).await,
                        ActorEvent::Stop(_err) => break,
                        ActorEvent::RemoveStream(id) => {
                            if ctx.streams.contains(id) {
                                ctx.streams.remove(id);
                            }
                        }
                    }
                }

                actor.stopped(&mut ctx).await;

                ctx.abort_streams();
                ctx.abort_intervals();

                tx_exit.send(()).ok();
            }
        });

        Ok(Addr {
            actor_id,
            tx,
            rx_exit,
        })
    }
}
