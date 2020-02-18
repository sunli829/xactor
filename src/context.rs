use crate::{Addr, Handler, Message};
use async_std::task;
use futures::{Stream, StreamExt};
use std::time::Duration;

///An actor execution context.
pub struct Context<A> {
    pub(crate) addr: Addr<A>,
}

impl<A> Context<A> {
    /// Returns the address of the actor.
    pub fn address(&self) -> Addr<A> {
        self.addr.clone()
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
    /// struct Add(i32);
    ///
    /// impl Message for Add {
    ///     type Result = ();
    /// }
    ///
    /// struct GetSum;
    ///
    /// impl Message for GetSum {
    ///     type Result = i32;
    /// }
    ///
    /// #[derive(Default)]
    /// struct MyActor(i32);
    ///
    /// #[async_trait::async_trait]
    /// impl Handler<Add> for MyActor {
    ///     async fn handle(&mut self, _ctx: &Context<Self>, msg: Add) {
    ///         self.0 += msg.0;
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
    ///         let values = (0..100).map(Add).collect::<Vec<_>>();
    ///         ctx.add_stream(stream::iter(values));
    ///     }
    /// }
    ///
    /// #[async_std::main]
    /// async fn main() -> anyhow::Result<()> {
    ///     let mut addr = MyActor::start_default();
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
        S::Item: Message<Result = ()>,
        A: Handler<S::Item>,
    {
        let mut addr = self.addr.clone();
        task::spawn(async move {
            while let Some(msg) = stream.next().await {
                if let Err(_) = addr.send(msg) {
                    return;
                }
            }
        });
    }

    /// Sends the message `msg` to self after a specified period of time.
    pub fn send_later<T>(&self, msg: T, after: Duration)
    where
        A: Handler<T>,
        T: Message<Result = ()>,
    {
        let mut addr = self.addr.clone();
        task::spawn(async move {
            task::sleep(after).await;
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
        task::spawn(async move {
            loop {
                task::sleep(dur).await;
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
}
