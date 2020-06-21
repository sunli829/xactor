use crate::actor::start_actor;
use crate::{Actor, Addr, Context};
use fnv::FnvHasher;
use futures::channel::oneshot;
use futures::lock::Mutex;
use futures::FutureExt;
use once_cell::sync::OnceCell;
use std::any::{Any, TypeId};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use anyhow::Result;

/// Trait define a global service.
///
/// The service is a global actor.
/// You can use `Actor::from_registry` to get the address `Addr<A>` of the service.
///
/// # Examples
///
/// ```rust
/// use xactor::*;
///
/// #[message(result = "i32")]
/// struct AddMsg(i32);
///
/// #[derive(Default)]
/// struct MyService(i32);
///
/// impl Actor for MyService {}
///
/// impl Service for MyService {}
///
/// #[async_trait::async_trait]
/// impl Handler<AddMsg> for MyService {
///     async fn handle(&mut self, ctx: &Context<Self>, msg: AddMsg) -> i32 {
///         self.0 += msg.0;
///         self.0
///     }
/// }
///
/// #[xactor::main]
/// async fn main() -> Result<()> {
///     let mut addr = MyService::from_registry().await?;
///     assert_eq!(addr.call(AddMsg(1)).await?, 1);
///     assert_eq!(addr.call(AddMsg(5)).await?, 6);
///     Ok(())
/// }
/// ```
#[async_trait::async_trait]
pub trait Service: Actor + Default {
    async fn from_registry() -> Result<Addr<Self>> {
        static REGISTRY: OnceCell<
            Mutex<HashMap<TypeId, Box<dyn Any + Send>, BuildHasherDefault<FnvHasher>>>,
        > = OnceCell::new();
        let registry = REGISTRY.get_or_init(Default::default);
        let mut registry = registry.lock().await;

        match registry.get_mut(&TypeId::of::<Self>()) {
            Some(addr) => Ok(addr.downcast_ref::<Addr<Self>>().unwrap().clone()),
            None => {
                let (tx_exit, rx_exit) = oneshot::channel();
                let rx_exit = rx_exit.shared();
                let (ctx, rx) = Context::new(Some(rx_exit));
                registry.insert(TypeId::of::<Self>(), Box::new(ctx.address()));
                drop(registry);
                start_actor(ctx.clone(), rx, tx_exit, Self::default()).await?;
                Ok(ctx.address())
            }
        }
    }
}

thread_local! {
    static LOCAL_REGISTRY: RefCell<HashMap<TypeId, Box<dyn Any + Send>, BuildHasherDefault<FnvHasher>>> = Default::default();
}

/// Trait define a local service.
///
/// The service is a thread local actor.
/// You can use `Actor::from_registry` to get the address `Addr<A>` of the service.
#[async_trait::async_trait]
pub trait LocalService: Actor + Default {
    async fn from_registry() -> Result<Addr<Self>> {
        let res = LOCAL_REGISTRY.with(|registry| {
            registry
                .borrow_mut()
                .get_mut(&TypeId::of::<Self>())
                .map(|addr| addr.downcast_ref::<Addr<Self>>().unwrap().clone())
        });
        match res {
            Some(addr) => Ok(addr),
            None => {
                let addr = {
                    let (tx_exit, rx_exit) = oneshot::channel();
                    let rx_exit = rx_exit.shared();
                    let (ctx, rx) = Context::new(Some(rx_exit));
                    start_actor(ctx.clone(), rx, tx_exit, Self::default()).await?;
                    ctx.address()
                };
                LOCAL_REGISTRY.with(|registry| {
                    registry
                        .borrow_mut()
                        .insert(TypeId::of::<Self>(), Box::new(addr.clone()));
                });
                Ok(addr)
            }
        }
    }
}
