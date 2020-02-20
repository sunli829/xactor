use futures::task::{AtomicWaker, Context, Poll};
use futures::Future;
use once_cell::sync::OnceCell;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub struct System {
    waker: AtomicWaker,
    count: AtomicUsize,
}

impl System {
    fn current() -> &'static System {
        static SYSTEM: OnceCell<System> = OnceCell::new();
        SYSTEM.get_or_init(|| Default::default())
    }

    pub(crate) fn inc_count() {
        System::current().count.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn dec_count() {
        System::current().count.fetch_sub(1, Ordering::Relaxed);
        System::current().waker.wake();
    }

    pub async fn wait_all() {
        WaitAll.await
    }
}

struct WaitAll;

impl Future for WaitAll {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let sys = System::current();
        if sys.count.load(Ordering::Relaxed) > 0 {
            sys.waker.register(cx.waker());
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}
