use crate::{Actor, Addr, Context, Handler, Message, Result, Sender, Service};
use fnv::FnvHasher;
use std::any::Any;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use std::marker::PhantomData;

pub(crate) struct Subscribe<T: Message<Result = ()>> {
    pub(crate) id: u64,
    pub(crate) sender: Sender<T>,
}

impl<T: Message<Result = ()>> Message for Subscribe<T> {
    type Result = ();
}

pub(crate) struct Unsubscribe {
    pub(crate) id: u64,
}

impl Message for Unsubscribe {
    type Result = ();
}

struct Publish<T: Message<Result = ()> + Clone>(T);

impl<T: Message<Result = ()> + Clone> Message for Publish<T> {
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
///     async fn started(&mut self, ctx: &Context<Self>)  {
///         ctx.subscribe::<MyMsg>().await;
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl Handler<MyMsg> for MyActor {
///     async fn handle(&mut self, _ctx: &Context<Self>, msg: MyMsg) {
///         self.0 += msg.0;
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl Handler<GetValue> for MyActor {
///     async fn handle(&mut self, _ctx: &Context<Self>, _msg: GetValue) -> String {
///         self.0.clone()
///     }
/// }
///
/// #[xactor::main]
/// async fn main() -> Result<()> {
///     let mut addr1 = MyActor::start_default().await;
///     let mut addr2 = MyActor::start_default().await;
///
///     Broker::from_registry().await.publish(MyMsg("a"));
///     Broker::from_registry().await.publish(MyMsg("b"));
///
///     sleep(Duration::from_secs(1)).await; // Wait for the messages
///
///     assert_eq!(addr1.call(GetValue).await?, "ab");
///     assert_eq!(addr2.call(GetValue).await?, "ab");
///     Ok(())
/// }
/// ```
pub struct Broker<T: Message<Result = ()>> {
    subscribes: HashMap<u64, Box<dyn Any + Send>, BuildHasherDefault<FnvHasher>>,
    mark: PhantomData<T>,
}

impl<T: Message<Result = ()>> Default for Broker<T> {
    fn default() -> Self {
        Self {
            subscribes: Default::default(),
            mark: PhantomData,
        }
    }
}

impl<T: Message<Result = ()>> Actor for Broker<T> {}

impl<T: Message<Result = ()>> Service for Broker<T> {}

#[async_trait::async_trait]
impl<T: Message<Result = ()>> Handler<Subscribe<T>> for Broker<T> {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Subscribe<T>) {
        self.subscribes.insert(msg.id, Box::new(msg.sender));
    }
}

#[async_trait::async_trait]
impl<T: Message<Result = ()>> Handler<Unsubscribe> for Broker<T> {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Unsubscribe) {
        self.subscribes.remove(&msg.id);
    }
}

#[async_trait::async_trait]
impl<T: Message<Result = ()> + Clone> Handler<Publish<T>> for Broker<T> {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Publish<T>) {
        for sender in self.subscribes.values_mut() {
            if let Some(sender) = sender.downcast_mut::<Sender<T>>() {
                sender.send(msg.0.clone()).ok();
            }
        }
    }
}

impl<T: Message<Result = ()> + Clone> Addr<Broker<T>> {
    /// Publishes a message of the specified type.
    pub fn publish(&mut self, msg: T) -> Result<()> {
        self.send(Publish(msg))
    }
}
