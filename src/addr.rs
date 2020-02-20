use crate::{Actor, Caller, Context, Error, Handler, Message, Result, Sender, StreamHandler};
use futures::channel::{mpsc, oneshot};
use futures::lock::Mutex;
use futures::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::Arc;

type ExecFuture = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub(crate) type ExecFn<A> =
    Box<dyn FnOnce(Arc<Mutex<A>>, Arc<Context<A>>) -> ExecFuture + Send + 'static>;

pub(crate) enum ActorEvent<A> {
    Exec(ExecFn<A>),
    Stop(Option<Error>),
}

/// The address of an actor.
///
/// When all references to `Addr<A>` are dropped, the actor ends.
/// You can use `Clone` trait to create multiple copies of `Addr<A>`.
pub struct Addr<A> {
    pub(crate) actor_id: u64,
    pub(crate) tx: mpsc::UnboundedSender<ActorEvent<A>>,
}

impl<A> Clone for Addr<A> {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id,
            tx: self.tx.clone(),
        }
    }
}

impl<A> PartialEq for Addr<A> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<A> Hash for Addr<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

impl<A: Actor> Addr<A> {
    /// Returns the id of the actor.
    pub fn actor_id(&self) -> u64 {
        self.actor_id
    }

    /// Send a message `msg` to the actor and wait for the return value.
    pub async fn call<T: Message>(&mut self, msg: T) -> Result<T::Result>
    where
        A: Handler<T>,
    {
        let (tx, rx) = oneshot::channel();
        self.tx
            .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                Box::pin(async move {
                    let mut actor = actor.lock().await;
                    let res = actor.handle(&ctx, msg).await;
                    let _ = tx.send(res);
                })
            })))?;

        Ok(rx.await?)
    }

    /// Send a message `msg` to the actor without waiting for the return value.
    pub fn send<T: Message<Result = ()>>(&mut self, msg: T) -> Result<()>
    where
        A: Handler<T>,
    {
        self.tx
            .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                Box::pin(async move {
                    let mut actor = actor.lock().await;
                    actor.handle(&ctx, msg).await;
                })
            })))?;
        Ok(())
    }

    pub(crate) fn send_stream_msg<T>(&mut self, msg: T) -> Result<()>
    where
        A: StreamHandler<T>,
        T: Send + 'static,
    {
        self.tx
            .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                Box::pin(async move {
                    let mut actor = actor.lock().await;
                    actor.handle(&ctx, msg).await;
                })
            })))?;
        Ok(())
    }

    /// Create a `Caller<T>` for a specific message type
    pub fn caller<T: Message>(&self) -> Caller<T>
    where
        A: Handler<T>,
    {
        let addr = self.clone();
        Caller(Box::new(move |msg| {
            let mut addr = addr.clone();
            Box::pin(async move { addr.call(msg).await })
        }))
    }

    /// Create a `Sender<T>` for a specific message type
    pub fn sender<T: Message<Result = ()>>(&self) -> Sender<T>
    where
        A: Handler<T>,
    {
        let addr = self.clone();
        Sender(Box::new(move |msg| {
            let mut addr = addr.clone();
            addr.send(msg)
        }))
    }
}
