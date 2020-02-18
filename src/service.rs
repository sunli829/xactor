use crate::{Actor, Addr};
use once_cell::sync::OnceCell;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Mutex;

/// Trait define a system service.
///
/// The system service is a global actor.
/// You can use `Actor::from_registry` to get the address `Addr<A>` of the service.
pub trait Service: Actor + Default {
    fn from_registry() -> Addr<Self> {
        static REGISTRY: OnceCell<Mutex<HashMap<TypeId, Box<dyn Any + Send>>>> = OnceCell::new();
        let registry = REGISTRY.get_or_init(|| Default::default());
        let mut registry = registry.lock().unwrap();

        let addr = registry
            .entry(TypeId::of::<Self>())
            .or_insert_with(|| Box::new(Self::default().start()));
        addr.downcast_ref::<Addr<Self>>().unwrap().clone()
    }
}
