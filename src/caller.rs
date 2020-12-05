use crate::{ActorId, Message, Result};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Mutex;

pub(crate) type CallerFuture<T> =
    Pin<Box<dyn Future<Output = Result<<T as Message>::Result>> + Send + 'static>>;

pub(crate) type CallerFn<T> = Box<dyn Fn(T) -> CallerFuture<T> + Send + 'static>;

pub(crate) type SenderFn<T> = Box<dyn Fn(T) -> Result<()> + 'static + Send>;

/// Caller of a specific message type
///
/// Like `Sender<T>, Caller has a weak reference to the recipient of the message type, and so will not prevent an actor from stopping if all Addr's have been dropped elsewhere.

pub struct Caller<T: Message> {
    pub actor_id: ActorId,
    pub(crate) caller_fn: Mutex<CallerFn<T>>,
}

impl<T: Message> Caller<T> {
    pub fn call(&self, msg: T) -> CallerFuture<T> {
        (self.caller_fn.lock().unwrap())(msg)
    }
}

impl<T: Message<Result = ()>> PartialEq for Caller<T> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<T: Message<Result = ()>> Hash for Caller<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

/// Sender of a specific message type
///
/// Like `Caller<T>, Sender has a weak reference to the recipient of the message type, and so will not prevent an actor from stopping if all Addr's have been dropped elsewhere.
/// This allows it to be used in `send_later` `send_interval` actor functions, and not keep the actor alive indefinitely even after all references to it have been dropped (unless `ctx.stop()` is called from within)

pub struct Sender<T: Message> {
    pub actor_id: ActorId,
    pub(crate) sender_fn: SenderFn<T>,
}

impl<T: Message<Result = ()>> Sender<T> {
    pub fn send(&self, msg: T) -> Result<()> {
        (self.sender_fn)(msg)
    }
}

impl<T: Message<Result = ()>> PartialEq for Sender<T> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<T: Message<Result = ()>> Hash for Sender<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}
