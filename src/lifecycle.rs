use crate::addr::ActorEvent;
use crate::error::Result;
use crate::runtime::spawn;
use crate::Actor;
use crate::{Addr, Context};
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::channel::oneshot;
use futures::{FutureExt, StreamExt};

pub(crate) struct ActorManager<A: Actor> {
    ctx: Context<A>,
    tx: std::sync::Arc<UnboundedSender<ActorEvent<A>>>,
    rx: UnboundedReceiver<ActorEvent<A>>,
    tx_exit: oneshot::Sender<()>,
}

impl<A: Actor> ActorManager<A> {
    pub(crate) fn new() -> Self {
        let (tx_exit, rx_exit) = oneshot::channel();
        let rx_exit = rx_exit.shared();
        let (ctx, rx, tx) = Context::new(Some(rx_exit));
        Self {
            ctx,
            rx,
            tx,
            tx_exit,
        }
    }

    pub(crate) fn address(&self) -> Addr<A> {
        self.ctx.address()
    }

    pub(crate) async fn start_actor(self, mut actor: A) -> Result<Addr<A>> {
        let Self {
            mut ctx,
            mut rx,
            tx,
            tx_exit,
        } = self;

        let rx_exit = ctx.rx_exit.clone();
        let actor_id = ctx.actor_id();

        // Call started
        actor.started(&mut ctx).await?;

        spawn({
            async move {
                while let Some(event) = rx.next().await {
                    match event {
                        ActorEvent::Exec(f) => f(&mut actor, &mut ctx).await,
                        ActorEvent::Stop(_err) => break,
                        ActorEvent::RemoveStream(id) => {
                            if ctx.streams.contains(id) {
                                ctx.streams.remove(id);
                            }
                        }
                    }
                }

                actor.stopped(&mut ctx).await;

                ctx.abort_streams();
                ctx.abort_intervals();

                tx_exit.send(()).ok();
            }
        });

        Ok(Addr {
            actor_id,
            tx,
            rx_exit,
        })
    }
}
