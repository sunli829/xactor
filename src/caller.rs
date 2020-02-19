use crate::{Message, Result};
use std::future::Future;
use std::pin::Pin;

pub(crate) type CallerFn<T> = Box<
    dyn Fn(T) -> Pin<Box<dyn Future<Output = Result<<T as Message>::Result>> + Send + 'static>>
        + 'static,
>;

pub(crate) type SenderFn<T> = Box<dyn Fn(T) -> Result<()> + 'static + Send>;

/// Caller of a specific message type
pub struct Caller<T: Message>(pub(crate) CallerFn<T>);

impl<T: Message> Caller<T> {
    pub async fn call(&mut self, msg: T) -> Result<T::Result> {
        self.0(msg).await
    }
}

/// Sender of a specific message type
pub struct Sender<T: Message>(pub(crate) SenderFn<T>);

impl<T: Message<Result = ()>> Sender<T> {
    pub fn send(&mut self, msg: T) -> Result<()> {
        self.0(msg)
    }
}
