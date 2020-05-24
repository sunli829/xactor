use crate::addr::ActorEvent;
use crate::broker::{Subscribe, Unsubscribe};
use crate::runtime::{sleep, spawn};
use crate::{Addr, Broker, Error, Handler, Message, Result, Service, StreamHandler};
use futures::channel::mpsc;
use futures::{Stream, StreamExt};
use once_cell::sync::OnceCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

// pub struct Inbox<A> {
//     priority: mpsc::UnboundedReceiver<ActorEvent<A>>,
//     outgoing: mpsc::UnboundedReceiver<ActorEvent<A>>,
// }

// impl<A> Inbox<A> {
//     fn new(outgoing: mpsc::UnboundedReceiver<ActorEvent<A>>) -> (mpsc::Sender<ActorEvent<A>>, Self) {
//         let (priority_tx, priority_rx) = mpsc::channel::<ActorEvent<A>>(16);

//         (
//             priority_tx,
//             Self {
//                 priority: priority_rx,
//                 outgoing: outgoing,
//             ),
//         }
//     }
// }

// impl<A> Stream for Inbox<A> {
//     type Item = ActorEvent<A>;
//     fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<A>> {
//         match self.priority.poll_next(cx) {
//             Poll::Pending => self.outgoing.poll_next(cx)
//             ready => ready,
//         }
//     }
// }

///An actor execution context.
pub struct Context<A> {
    actor_id: u64,
    addr: Addr<A>,
}

impl<A> Context<A> {
    pub(crate) fn new() -> (Arc<Self>, Inbox<A>) {
        static ACTOR_ID: OnceCell<AtomicU64> = OnceCell::new();

        // Get an actor id
        let actor_id = ACTOR_ID
            .get_or_init(|| Default::default())
            .fetch_add(1, Ordering::Relaxed);

        let (actor_tx, actor_rx) = mpsc::unbounded::<ActorEvent<A>>();
        let (priority_tx, priority_rx) = mpsc::unbounded::<ActorEvent<A>>();
        (
            Arc::new(Self {
                actor_id,
                addr: Addr { actor_id, tx },
            }),
            select! {
                
            },
        )
    }

    /// Returns the address of the actor.
    pub fn address(&self) -> Addr<A> {
        self.addr.clone()
    }

    /// Returns the id of the actor.
    pub fn actor_id(&self) -> u64 {
        self.actor_id
    }

    /// Stop the actor.
    pub fn stop(&self, err: Option<Error>) {
        self.addr.tx.clone().start_send(ActorEvent::Stop(err)).ok();
    }

    /// Create a stream handler for the actor.
    ///
    /// # Examples
    /// ```rust
    /// use xactor::*;
    /// use futures::stream;
    /// use async_std::task;
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
    ///     async fn started(&mut self, _ctx: &Context<Self>) {
    ///         println!("stream started");
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
    ///     async fn started(&mut self, ctx: &Context<Self>) {
    ///         let values = (0..100).collect::<Vec<_>>();
    ///         ctx.add_stream(stream::iter(values));
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() -> Result<()> {
    ///     let mut addr = MyActor::start_default().await;
    ///     task::sleep(Duration::from_secs(1)).await; // Wait for the stream to complete
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
        let mut addr = self.addr.clone();
        spawn(async move {
            addr.tx
                .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                    Box::pin(async move {
                        let mut actor = actor.lock().await;
                        StreamHandler::started(&mut *actor, &ctx).await;
                    })
                })))
                .ok();

            while let Some(msg) = stream.next().await {
                if let Err(_) = addr
                    .tx
                    .start_send(ActorEvent::Exec(Box::new(move |actor, ctx| {
                        Box::pin(async move {
                            let mut actor = actor.lock().await;
                            StreamHandler::handle(&mut *actor, &ctx, msg).await;
                        })
                    })))
                {
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
        });
    }

    /// Sends the message `msg` to self after a specified period of time.
    pub fn send_later<T>(&self, msg: T, after: Duration)
    where
        A: Handler<T>,
        T: Message<Result = ()>,
    {
        let mut addr = self.addr.clone();
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
        let mut addr = self.addr.clone();
        spawn(async move {
            loop {
                sleep(dur).await;
                if let Err(_) = addr.send(f()) {
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

    pub fn send_with_timeout<T>(&self, msg: T, timeout: Duration)
    where
        A: Handler<T>,
        T: Message,
    {
        let mut addr = self.addr.clone();
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
