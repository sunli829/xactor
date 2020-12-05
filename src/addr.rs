use crate::{Actor, ActorId, Caller, Context, Error, Handler, Message, Result, Sender};
use futures::channel::{mpsc, oneshot};
use futures::future::Shared;
use futures::Future;
use std::hash::{Hash, Hasher};
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};

type ExecFuture<'a> = Pin<Box<dyn Future<Output = ()> + Send + 'a>>;

pub(crate) type ExecFn<A> =
    Box<dyn for<'a> FnOnce(&'a mut A, &'a mut Context<A>) -> ExecFuture<'a> + Send + 'static>;

pub(crate) enum ActorEvent<A> {
    Exec(ExecFn<A>),
    Stop(Option<Error>),
    RemoveStream(usize),
}

/// The address of an actor.
///
/// When all references to `Addr<A>` are dropped, the actor ends.
/// You can use `Clone` trait to create multiple copies of `Addr<A>`.
pub struct Addr<A> {
    pub(crate) actor_id: ActorId,
    pub(crate) tx: Arc<mpsc::UnboundedSender<ActorEvent<A>>>,
    pub(crate) rx_exit: Option<Shared<oneshot::Receiver<()>>>,
}

impl<A> Clone for Addr<A> {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id,
            tx: self.tx.clone(),
            rx_exit: self.rx_exit.clone(),
        }
    }
}

impl<A> Addr<A> {
    pub fn downgrade(&self) -> WeakAddr<A> {
        WeakAddr {
            actor_id: self.actor_id,
            tx: Arc::downgrade(&self.tx),
            rx_exit: self.rx_exit.clone(),
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
    pub fn actor_id(&self) -> ActorId {
        self.actor_id
    }

    /// Stop the actor.
    pub fn stop(&mut self, err: Option<Error>) -> Result<()> {
        mpsc::UnboundedSender::clone(&*self.tx).start_send(ActorEvent::Stop(err))?;
        Ok(())
    }

    /// Send a message `msg` to the actor and wait for the return value.
    pub async fn call<T: Message>(&self, msg: T) -> Result<T::Result>
    where
        A: Handler<T>,
    {
        let (tx, rx) = oneshot::channel();
        mpsc::UnboundedSender::clone(&*self.tx).start_send(ActorEvent::Exec(Box::new(
            move |actor, ctx| {
                Box::pin(async move {
                    let res = Handler::handle(actor, ctx, msg).await;
                    let _ = tx.send(res);
                })
            },
        )))?;

        Ok(rx.await?)
    }

    /// Send a message `msg` to the actor without waiting for the return value.
    pub fn send<T: Message<Result = ()>>(&self, msg: T) -> Result<()>
    where
        A: Handler<T>,
    {
        mpsc::UnboundedSender::clone(&*self.tx).start_send(ActorEvent::Exec(Box::new(
            move |actor, ctx| {
                Box::pin(async move {
                    Handler::handle(actor, ctx, msg).await;
                })
            },
        )))?;
        Ok(())
    }

    /// Create a `Caller<T>` for a specific message type
    pub fn caller<T: Message>(&self) -> Caller<T>
    where
        A: Handler<T>,
    {
        let weak_tx = Arc::downgrade(&self.tx);

        Caller {
            actor_id: self.actor_id.clone(),
            caller_fn: Mutex::new(Box::new(move |msg| {
                let weak_tx_option = weak_tx.upgrade();
                Box::pin(async move {
                    match weak_tx_option {
                        Some(tx) => {
                            let (oneshot_tx, oneshot_rx) = oneshot::channel();

                            mpsc::UnboundedSender::clone(&tx).start_send(ActorEvent::Exec(
                                Box::new(move |actor, ctx| {
                                    Box::pin(async move {
                                        let res = Handler::handle(&mut *actor, ctx, msg).await;
                                        let _ = oneshot_tx.send(res);
                                    })
                                }),
                            ))?;
                            Ok(oneshot_rx.await?)
                        }
                        None => Err(anyhow::anyhow!("Actor Dropped")),
                    }
                })
            })),
        }
    }

    /// Create a `Sender<T>` for a specific message type
    pub fn sender<T: Message<Result = ()>>(&self) -> Sender<T>
    where
        A: Handler<T>,
    {
        let weak_tx = Arc::downgrade(&self.tx);
        Sender {
            actor_id: self.actor_id.clone(),
            sender_fn: Box::new(move |msg| match weak_tx.upgrade() {
                Some(tx) => {
                    mpsc::UnboundedSender::clone(&tx).start_send(ActorEvent::Exec(Box::new(
                        move |actor, ctx| {
                            Box::pin(async move {
                                Handler::handle(&mut *actor, ctx, msg).await;
                            })
                        },
                    )))?;
                    Ok(())
                }
                None => Ok(()),
            }),
        }
    }

    /// Wait for an actor to finish, and if the actor has finished, the function returns immediately.
    pub async fn wait_for_stop(self) {
        if let Some(rx_exit) = self.rx_exit {
            rx_exit.await.ok();
        } else {
            futures::future::pending::<()>().await;
        }
    }
}

pub struct WeakAddr<A> {
    pub(crate) actor_id: ActorId,
    pub(crate) tx: Weak<mpsc::UnboundedSender<ActorEvent<A>>>,
    pub(crate) rx_exit: Option<Shared<oneshot::Receiver<()>>>,
}

impl<A> PartialEq for WeakAddr<A> {
    fn eq(&self, other: &Self) -> bool {
        self.actor_id == other.actor_id
    }
}

impl<A> Hash for WeakAddr<A> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.actor_id.hash(state)
    }
}

impl<A> WeakAddr<A> {
    pub fn upgrade(&self) -> Option<Addr<A>> {
        match self.tx.upgrade() {
            Some(tx) => Some(Addr {
                actor_id: self.actor_id,
                tx,
                rx_exit: self.rx_exit.clone(),
            }),
            None => None,
        }
    }
}

impl<A> Clone for WeakAddr<A> {
    fn clone(&self) -> Self {
        Self {
            actor_id: self.actor_id,
            tx: self.tx.clone(),
            rx_exit: self.rx_exit.clone(),
        }
    }
}

impl<A: Actor> WeakAddr<A> {
    /// Returns the id of the actor.
    pub fn actor_id(&self) -> ActorId {
        self.actor_id
    }
}
