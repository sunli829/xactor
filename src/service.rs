use crate::{Actor, Addr};
use futures::lock::Mutex;
use once_cell::sync::OnceCell;
use std::any::{Any, TypeId};
use std::collections::HashMap;

/// Trait define a system service.
///
/// The system service is a global actor.
/// You can use `Actor::from_registry` to get the address `Addr<A>` of the service.
#[async_trait::async_trait]
pub trait Service: Actor + Default {
    async fn from_registry() -> Addr<Self> {
        static REGISTRY: OnceCell<Mutex<HashMap<TypeId, Box<dyn Any + Send>>>> = OnceCell::new();
        let registry = REGISTRY.get_or_init(|| Default::default());
        let mut registry = registry.lock().await;

        match registry.get_mut(&TypeId::of::<Self>()) {
            Some(addr) => addr.downcast_ref::<Addr<Self>>().unwrap().clone(),
            None => {
                let addr = Self::default().start().await;
                registry.insert(TypeId::of::<Self>(), Box::new(addr.clone()));
                addr
            }
        }
    }
}
