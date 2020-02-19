use crate::addr::ExecFn;
use crate::{Addr, Context};
use async_std::task;
use futures::channel::mpsc;
use futures::lock::Mutex;
use futures::StreamExt;
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Represents a message that can be handled by the actor.
pub trait Message: 'static + Send {
    /// The return value type of the message
    /// This type can be set to () if the message does not return a value, or if it is a notification message
    type Result: 'static + Send;
}

/// Describes how to handle messages of a specific type.
/// Implementing Handler is a general way to handle incoming messages and streams.
/// The type T is a message which can be handled by the actor.
#[async_trait::async_trait]
pub trait Handler<T: Message>: Actor {
    async fn handle(&mut self, ctx: &Context<Self>, msg: T) -> T::Result;
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
pub trait Actor: Sized + Send + 'static {
    /// Called when the actor is first started
    async fn started(&mut self, _ctx: &Context<Self>) {}

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
    /// #[async_std::main]
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
    async fn start(mut self) -> Addr<Self> {
        static ACTOR_ID: OnceCell<AtomicU64> = OnceCell::new();

        // Get an actor id
        let actor_id = ACTOR_ID
            .get_or_init(|| Default::default())
            .fetch_add(1, Ordering::Relaxed);

        let (tx, mut rx) = mpsc::unbounded::<ExecFn<Self>>();
        let addr = Addr { actor_id, tx };

        // Call started
        self.started(&Context {
            actor_id,
            addr: addr.clone(),
        })
        .await;

        let actor = Arc::new(Mutex::new(self));
        task::spawn({
            async move {
                while let Some(f) = rx.next().await {
                    f(actor.clone()).await;
                }
            }
        });

        addr
    }
}
