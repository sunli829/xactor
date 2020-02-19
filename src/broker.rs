use crate::{Actor, Addr, Context, Handler, Message, Result, Sender, Service};
use fnv::FnvHasher;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::hash::BuildHasherDefault;

pub(crate) struct Subscribe<T: Message<Result = ()>> {
    pub(crate) id: u64,
    pub(crate) sender: Sender<T>,
}

impl<T: Message<Result = ()>> Message for Subscribe<T> {
    type Result = ();
}

pub(crate) struct Unsubscribe {
    pub(crate) type_id: TypeId,
    pub(crate) id: u64,
}

impl Message for Unsubscribe {
    type Result = ();
}

struct Publish<T: Message<Result = ()> + Clone>(T);

impl<T: Message<Result = ()> + Clone> Message for Publish<T> {
    type Result = ();
}

type SubscribesMap = HashMap<TypeId, ActorsMap, BuildHasherDefault<FnvHasher>>;

type ActorsMap = HashMap<u64, Box<dyn Any + Send>, BuildHasherDefault<FnvHasher>>;

/// Message broker is used to support publishing and subscribing to messages.
///
/// # Examples
///
/// ```rust
/// use xactor::*;
/// use std::time::Duration;
/// use async_std::task;
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
///         ctx.subscribe::<MyMsg>();
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl Handler<MyMsg> for MyActor {
///     async fn handle(&mut self, _ctx: &Context<Self>, msg: MyMsg) {
///         self.0 += msg.0;
///         println!("add {}", self.0);
///     }
/// }
///
/// #[async_trait::async_trait]
/// impl Handler<GetValue> for MyActor {
///     async fn handle(&mut self, _ctx: &Context<Self>, _msg: GetValue) -> String {
///         println!("get {}", self.0);
///         self.0.clone()
///     }
/// }
///
/// #[async_std::main]
/// async fn main() -> Result<()> {
///     let mut addr1 = MyActor::start_default();
///     let mut addr2 = MyActor::start_default();
///
///     Broker::from_registry().publish(MyMsg("a"));
///     Broker::from_registry().publish(MyMsg("b"));
///
///     task::sleep(Duration::from_secs(1)).await; // Wait for the actors started
///
///     assert_eq!(addr1.call(GetValue).await?, "ab");
///     assert_eq!(addr2.call(GetValue).await?, "ab");
///     Ok(())
/// }
/// ```
#[derive(Default)]
pub struct Broker {
    subscribes: SubscribesMap,
}

impl Actor for Broker {}

impl Service for Broker {}

#[async_trait::async_trait]
impl<T: Message<Result = ()>> Handler<Subscribe<T>> for Broker {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Subscribe<T>) {
        let actors = self
            .subscribes
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Default::default());
        actors.insert(msg.id, Box::new(msg.sender));
    }
}

#[async_trait::async_trait]
impl Handler<Unsubscribe> for Broker {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Unsubscribe) {
        if let Some(actors) = self.subscribes.get_mut(&msg.type_id) {
            actors.remove(&msg.id);
            if actors.is_empty() {
                self.subscribes.remove(&msg.type_id);
            }
        }
    }
}

#[async_trait::async_trait]
impl<T: Message<Result = ()> + Clone> Handler<Publish<T>> for Broker {
    async fn handle(&mut self, _ctx: &Context<Self>, msg: Publish<T>) {
        if let Some(actors) = self.subscribes.get_mut(&msg.0.type_id()) {
            for (_, sender) in actors {
                if let Some(sender) = sender.downcast_mut::<Sender<T>>() {
                    sender.send(msg.0.clone()).ok();
                }
            }
        }
    }
}

impl Addr<Broker> {
    /// Publishes a message of the specified type.
    pub fn publish<T: Message<Result = ()> + Clone>(&mut self, msg: T) -> Result<()> {
        self.send(Publish(msg))
    }
}
