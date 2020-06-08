use crate::addr::ActorEvent;
use crate::broker::{Subscribe, Unsubscribe};
use crate::runtime::{sleep, spawn};
use crate::{Addr, Broker, Error, Handler, Message, Result, Service, StreamHandler};
use futures::channel::{mpsc, oneshot};
use futures::future::{AbortHandle, Abortable, Shared};
use futures::{Stream, StreamExt};
use once_cell::sync::OnceCell;
use slab::Slab;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

///An actor execution context.
pub struct Context<A> {
    actor_id: u64,
    tx: mpsc::UnboundedSender<ActorEvent<A>>,
    rx_exit: Option<Shared<oneshot::Receiver<()>>>,
    pub(crate) streams: Arc<Mutex<Slab<AbortHandle>>>,
}

impl<A> Context<A> {
    pub(crate) fn new(
        rx_exit: Option<Shared<oneshot::Receiver<()>>>,
    ) -> (Arc<Self>, mpsc::UnboundedReceiver<ActorEvent<A>>) {
        static ACTOR_ID: OnceCell<AtomicU64> = OnceCell::new();

        // Get an actor id
        let actor_id = ACTOR_ID
            .get_or_init(Default::default)
            .fetch_add(1, Ordering::Relaxed);

        let (tx, rx) = mpsc::unbounded::<ActorEvent<A>>();
        (
            Arc::new(Self {
                actor_id,
                tx,
                rx_exit,
                streams: Default::default(),
            }),
            rx,
        )
    }

    /// Returns the address of the actor.
    pub fn address(&self) -> Addr<A> {
        Addr {
            actor_id: self.actor_id,
            tx: self.tx.clone(),
            rx_exit: self.rx_exit.clone(),
        }
    }

    /// Returns the id of the actor.
    pub fn actor_id(&self) -> u64 {
        self.actor_id
    }

    /// Stop the actor.
    pub fn stop(&self, err: Option<Error>) {
        self.tx.clone().start_send(ActorEvent::Stop(err)).ok();
    }

    /// Create a stream handler for the actor.
    ///
    /// # Examples
    /// ```rust
    /// use xactor::*;
    /// use futures::stream;
    /// use std::time::Duration;
    ///
    /// #[message(result = "i32")]
    /// struct GetSum;
    ///
    /// #[derive(Default)]
    /// struct MyActor(i32);
    ///
    /// #[async_trait::async_trait]
    /// impl StreamHandler<i32> for MyActor {
    ///     async fn handle(&mut self, _ctx: &Context<Self>, msg: i32) {
    ///         self.0 += msg;
    ///     }
    ///
    ///     async fn started(&mut self, _ctx: &Context<Self>) -> Result<()>{
    ///         println!("stream started");
    ///         Ok(())
    ///     }
    ///
    ///     async fn finished(&mut self, _ctx: &Context<Self>) {
    ///         println!("stream finished");
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<GetSum> for MyActor {
    ///     async fn handle(&mut self, _ctx: &Context<Self>, _msg: GetSum) -> i32 {
    ///         self.0
    ///     }
    /// }
    ///
    /// #[async_trait::async_trait]
    /// impl Actor for MyActor {
    ///     async fn started(&mut self, ctx: &Context<Self>) -> Result<()> {
    ///         let values = (0..100).collect::<Vec<_>>();
    ///         ctx.add_stream(stream::iter(values));
    ///         Ok(())
    ///     }
    /// }
    ///
    /// #[xactor::main]
    /// async fn main() -> Result<()> {
    ///     let mut addr = MyActor::start_default().await;
    ///     sleep(Duration::from_secs(1)).await; // Wait for the stream to complete
    ///     let res = addr.call(GetSum).await?;
    ///     assert_eq!(res, (0..100).sum::<i32>());
    ///     Ok(())
    /// }
    /// ```
    /// ```
    pub fn add_stream<S>(&self, mut stream: S)
    where
        S: Stream + Unpin + Send + 'static,
        S::Item: 'static + Send,
        A: StreamHandler<S::Item>,
    {
        let mut addr = self.address();
        let mut inner_streams = self.streams.lock().unwrap();
        let entry = inner_streams.vacant_entry();
        let id = entry.key();
        let (handle, registration) = futures::future::AbortHandle::new_pair();
        entry.insert(handle);

        let fut = {
            let streams = self.streams.clone();
            async move {
                addr.tx
                    .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                        Box::pin(async move {
                            let mut actor = actor.lock().await;
                            StreamHandler::started(&mut *actor, &ctx).await;
                        })
                    })))
                    .ok();

                while let Some(msg) = stream.next().await {
                    let res = addr
                        .tx
                        .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                            Box::pin(async move {
                                let mut actor = actor.lock().await;
                                StreamHandler::handle(&mut *actor, &ctx, msg).await;
                            })
                        })));
                    if res.is_err() {
                        return;
                    }
                }

                addr.tx
                    .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                        Box::pin(async move {
                            let mut actor = actor.lock().await;
                            StreamHandler::finished(&mut *actor, &ctx).await;
                        })
                    })))
                    .ok();

                let mut streams = streams.lock().unwrap();
                if streams.contains(id) {
                    streams.remove(id);
                }
            }
        };
        spawn(Abortable::new(fut, registration));
    }

    /// Sends the message `msg` to self after a specified period of time.
    pub fn send_later<T>(&self, msg: T, after: Duration)
    where
        A: Handler<T>,
        T: Message<Result = ()>,
    {
        let mut addr = self.address();
        spawn(async move {
            sleep(after).await;
            addr.send(msg).ok();
        });
    }

    /// Sends the message  to self, at a specified fixed interval.
    /// The message is created each time using a closure `f`.
    pub fn send_interval_with<T, F>(&self, f: F, dur: Duration)
    where
        A: Handler<T>,
        F: Fn() -> T + Sync + Send + 'static,
        T: Message<Result = ()>,
    {
        let mut addr = self.address();
        spawn(async move {
            loop {
                sleep(dur).await;
                if addr.send(f()).is_err() {
                    break;
                }
            }
        });
    }

    /// Sends the message `msg` to self, at a specified fixed interval.
    pub fn send_interval<T>(&self, msg: T, dur: Duration)
    where
        A: Handler<T>,
        T: Message<Result = ()> + Clone + Sync,
    {
        self.send_interval_with(move || msg.clone(), dur);
    }

    /// Subscribes to a message of a specified type.
    pub async fn subscribe<T: Message<Result = ()>>(&self) -> Result<()>
    where
        A: Handler<T>,
    {
        let mut broker = Broker::<T>::from_registry().await;
        broker.send(Subscribe {
            id: self.actor_id,
            sender: self.address().sender::<T>(),
        })
    }

    /// Unsubscribe to a message of a specified type.
    pub async fn unsubscribe<T: Message<Result = ()>>(&self) -> Result<()> {
        let mut broker = Broker::<T>::from_registry().await;
        broker.send(Unsubscribe { id: self.actor_id })
    }
}
