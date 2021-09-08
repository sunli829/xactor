#[cfg(feature = "generic-mailbox")]
mod mailbox {
    // This module really exists to group everything under the feature
    use futures::channel::mpsc;
    use futures::{Sink, Stream};

    /// Marker trait representing containers usable as the message queue of an [`Actor`](crate::actor::Actor)
    ///
    /// The underlying message type is kept private as an implementation detail, so the implementing types must be generic over `T : Send`
    pub trait MessageQueue {
        type Tx<T: Send>: MessageSender<T>;
        type Rx<T: Send>: MessageReceiver<T>;

        /// Create a linked pair [`MessageSender`] and [`MessageReceiver`]
        fn create<T: Send>() -> (Self::Tx<T>, Self::Rx<T>);
    }

    pub trait MessageSender<T>:
        Clone + Send + Sync + Sink<T, Error = <Self as MessageSender<T>>::Error> + Unpin
    where
        T: Send,
    {
        type Error: std::error::Error + Sync + Send;
    }

    pub trait MessageReceiver<T>: Send + Sync + Stream<Item = T> + Unpin
    where
        T: Send,
    {
    }

    /// Default Mailbox, implemented by an [unbounded queue](futures::channel::mpsc::unbounded)
    ///
    /// This was also the original behaviour in previous versions of this crate.
    pub struct UnboundedMailbox;

    impl MessageQueue for UnboundedMailbox {
        type Tx<A: Send> = mpsc::UnboundedSender<A>;
        type Rx<B: Send> = mpsc::UnboundedReceiver<B>;
        fn create<T>() -> (mpsc::UnboundedSender<T>, mpsc::UnboundedReceiver<T>) {
            mpsc::unbounded()
        }
    }
    impl<T: Send> MessageSender<T> for mpsc::UnboundedSender<T> {
        type Error = mpsc::SendError;
    }
    impl<T: Send> MessageReceiver<T> for mpsc::UnboundedReceiver<T> {}

    /// Same as the default [UnboundedMailbox], except that there is a bounded number of items, with sends potentially blocking.
    /// See [mpsc::channel] for details.
    pub struct BoundedMailbox<const BUFFER: usize> {}

    impl<const BUFFER: usize> MessageQueue for BoundedMailbox<BUFFER> {
        type Tx<A: Send> = mpsc::Sender<A>;
        type Rx<B: Send> = mpsc::Receiver<B>;
        fn create<T>() -> (mpsc::Sender<T>, mpsc::Receiver<T>) {
            mpsc::channel(BUFFER)
        }
    }
    impl<T: Send> MessageSender<T> for mpsc::Sender<T> {
        type Error = mpsc::SendError;
    }
    impl<T: Send> MessageReceiver<T> for mpsc::Receiver<T> {}
}

#[cfg(feature = "generic-mailbox")]
pub use mailbox::*;
#[cfg(feature = "generic-mailbox")]
mod mailbox_private {
    // Need to have a separate module because the use::* would expose these publicly
    use super::mailbox::*;
    use crate::addr::ActorEvent;
    use crate::Actor;
    pub(crate) type EventReceiver<A> = <<A as Actor>::Mailbox as MessageQueue>::Rx<ActorEvent<A>>;
    pub(crate) type EventSender<A> = <<A as Actor>::Mailbox as MessageQueue>::Tx<ActorEvent<A>>;
}

#[cfg(not(feature = "generic-mailbox"))]
mod mailbox_private {
    use crate::addr::ActorEvent;
    use futures::channel::mpsc::*;
    pub(crate) type EventReceiver<A> = UnboundedReceiver<ActorEvent<A>>;
    pub(crate) type EventSender<A> = UnboundedSender<ActorEvent<A>>;
}

pub(crate) use mailbox_private::{EventReceiver, EventSender};
