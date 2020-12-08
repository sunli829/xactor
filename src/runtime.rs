#![allow(unused_imports)]

#[cfg(not(any(feature = "runtime-tokio", feature = "runtime-async-std")))]
compile_error!("one of 'runtime-async-std' or 'runtime-tokio' features must be enabled");

#[cfg(all(feature = "runtime-tokio", feature = "runtime-async-std"))]
compile_error!("only one of 'runtime-async-std' or 'runtime-tokio' features must be enabled");

#[cfg(feature = "runtime-async-std")]
pub use async_std::{future::timeout, task::block_on, task::sleep, task::spawn};

use futures::Future;
#[cfg(feature = "runtime-tokio")]
pub use tokio::{task::spawn, time::sleep, time::timeout};

#[cfg(feature = "runtime-tokio")]
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(future)
}
