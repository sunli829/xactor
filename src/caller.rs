use crate::{ActorId, Message, Result};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;

/// Caller of a specific message type
///
/// Like `Sender<T>`, Caller has a weak reference to the recipient of the message type, and so will not prevent an actor from stopping if all Addr's have been dropped elsewhere.
/// This takes a boxed closure with the message as a parameter with the mpsc channel of the actor inside and the type of actor abstracted away.

pub struct Caller<T: Message> {
    pub actor_id: ActorId,
    pub(crate) caller_fn: Box<dyn CallerFn<T>>,
}

impl<T: Message> Caller<T> {
    pub async fn call(&self, msg: T) -> Result<T::Result> {
        self.caller_fn.call(msg).await
    }
}

impl<T: Message> PartialEq for Caller<T> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<T: Message> Hash for Caller<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

impl<T: Message> Clone for Caller<T> {
    fn clone(&self) -> Caller<T> {
        Caller {
            actor_id: self.actor_id,
            caller_fn: dyn_clone::clone_box(&*self.caller_fn),
        }
    }
}

/// Sender of a specific message type
///
/// Like `Caller<T>`, Sender has a weak reference to the recipient of the message type, and so will not prevent an actor from stopping if all Addr's have been dropped elsewhere.
/// This allows it to be used in `send_later` `send_interval` actor functions, and not keep the actor alive indefinitely even after all references to it have been dropped (unless `ctx.stop()` is called from within)

pub struct Sender<T: Message> {
    pub actor_id: ActorId,
    pub(crate) sender_fn: Box<dyn SenderFn<T>>,
}

impl<T: Message<Result = ()>> Sender<T> {
    pub fn send(&self, msg: T) -> Result<()> {
        self.sender_fn.send(msg)
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

impl<T: Message<Result = ()>> Clone for Sender<T> {
    fn clone(&self) -> Sender<T> {
        Sender {
            actor_id: self.actor_id,
            sender_fn: dyn_clone::clone_box(&*self.sender_fn),
        }
    }
}

// https://stackoverflow.com/questions/63842261/how-to-derive-clone-for-structures-with-boxed-closure
// https://users.rust-lang.org/t/expected-opaque-type-found-a-different-opaque-type-when-trying-futures-join-all/40596/3
use dyn_clone::DynClone;

pub trait SenderFn<T>: DynClone + 'static + Send + Sync
where
    T: Message,
{
    fn send(&self, msg: T) -> Result<()>;
}

impl<F, T> SenderFn<T> for F
where
    F: Fn(T) -> Result<()> + 'static + Send + Sync + Clone,
    T: Message,
{
    fn send(&self, msg: T) -> Result<()> {
        self(msg)
    }
}

pub trait CallerFn<T>: DynClone + 'static + Send + Sync
where
    T: Message,
{
    fn call(&self, msg: T) -> Pin<Box<dyn Future<Output = Result<T::Result>>>>;
}

impl<F, T> CallerFn<T> for F
where
    F: Fn(T) -> Pin<Box<dyn Future<Output = Result<T::Result>>>> + 'static + Send + Sync + Clone,
    T: Message,
{
    fn call(&self, msg: T) -> Pin<Box<dyn Future<Output = Result<T::Result>>>> {
        self(msg)
    }
}
