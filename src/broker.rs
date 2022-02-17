use crate::{Actor, ActorId, Addr, Context, Handler, Message, Result, Sender, Service};
use fnv::FnvHasher;
use std::{collections::HashMap, hash::BuildHasherDefault};

pub(crate) struct Subscribe<T: Message<Result = ()>> {
    pub(crate) actor_id: ActorId,
    pub(crate) sender: Sender<T>,
}

impl<T: Message<Result = ()>> Message for Subscribe<T> {
    type Result = ();
}

pub(crate) struct Unsubscribe {
    pub(crate) actor_id: ActorId,
}

impl Message for Unsubscribe {
    type Result = ();
}
/// Message broker is used to support publishing and subscribing to messages.
///
/// # Examples
///
/// ```rust
/// use xactor::*;
/// use std::time::Duration;
///
/// #[message]
/// #[derive(Clone)]
/// struct MyMsg(&'static str);
///
/// #[message(result = "String")]
/// struct GetValue;
///
/// #[derive(Default)]
/// struct MyActor(String);
///
/// #[async_trait::async_trait]
/// impl Actor for MyActor {
///     async fn started(&mut self, ctx: &mut Context<Self>) -> Result<()>  {
///         ctx.subscribe::<MyMsg>().await;
///         Ok(())
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl Handler<MyMsg> for MyActor {
///     async fn handle(&mut self, _ctx: &mut Context<Self>, msg: MyMsg) {
///         self.0 += msg.0;
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl Handler<GetValue> for MyActor {
///     async fn handle(&mut self, _ctx: &mut Context<Self>, _msg: GetValue) -> String {
///         self.0.clone()
///     }
/// }
///
/// #[xactor::main]
/// async fn main() -> Result<()> {
///     let mut addr1 = MyActor::start_default().await?;
///     let mut addr2 = MyActor::start_default().await?;
///
///     Broker::from_registry().await?.publish(MyMsg("a"));
///     Broker::from_registry().await?.publish(MyMsg("b"));
///
///     sleep(Duration::from_secs(1)).await; // Wait for the messages
///
///     assert_eq!(addr1.call(GetValue).await?, "ab");
///     assert_eq!(addr2.call(GetValue).await?, "ab");
///     Ok(())
/// }
/// ```
pub struct Broker<T: Message<Result = ()>> {
    subscribers: HashMap<ActorId, Sender<T>, BuildHasherDefault<FnvHasher>>,
}

impl<T: Message<Result = ()>> Default for Broker<T> {
    fn default() -> Self {
        Self {
            subscribers: Default::default(),
        }
    }
}

impl<T: Message<Result = ()>> Actor for Broker<T> {}

impl<T: Message<Result = ()>> Service for Broker<T> {}

#[async_trait::async_trait]
impl<T: Message<Result = ()>> Handler<Subscribe<T>> for Broker<T> {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: Subscribe<T>) {
        self.subscribers.insert(msg.actor_id, msg.sender);
    }
}

#[async_trait::async_trait]
impl<T: Message<Result = ()>> Handler<Unsubscribe> for Broker<T> {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: Unsubscribe) {
        self.subscribers.remove(&msg.actor_id);
    }
}

#[async_trait::async_trait]
impl<T: Message<Result = ()> + Clone> Handler<T> for Broker<T> {
    async fn handle(&mut self, _ctx: &mut Context<Self>, msg: T) {
        // Broadcast to all subscribers and remove any senders that return an error (most likely because reciever dropped because actor already stopped)
        self.subscribers
            .retain(|_actor_id, sender| sender.send(msg.clone()).is_ok())
    }
}

impl<T: Message<Result = ()> + Clone> Addr<Broker<T>> {
    /// Publishes a message of the specified type.
    pub fn publish(&mut self, msg: T) -> Result<()> {
        self.send(msg)
    }
}
