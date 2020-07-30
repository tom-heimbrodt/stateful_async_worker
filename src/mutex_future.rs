use std::sync::{Arc, Mutex};
use std::future::Future;
use std::task::{Poll, Context, Waker};
use std::pin::Pin;

pub struct MutexFuture<T> {
    value: Arc<Mutex<MutexFutureInner<T>>>,
}

// deriving Clone doesn't work, since it would require T: Clone
impl<T> Clone for MutexFuture<T> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
        }
    }
}

struct MutexFutureInner<T> {
    waker: Option<Waker>,
    result: Option<T>,
}

impl<T> MutexFuture<T> {
    pub fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(MutexFutureInner {
                waker: None,
                result: None,
            })),
        }
    }

    pub fn complete(&self, value: T) {
        let waker;
        {
            let mut inner = self.value.lock().unwrap();
            inner.result = Some(value);
            waker = inner.waker.take();
        }
        // wake up atfer lock has been dropped
        waker.unwrap().wake();
    }
}

impl<T> Future for MutexFuture<T> {
    type Output = T;

    fn poll(self: Pin<&mut Self>, context: &mut Context) -> Poll<Self::Output> {
        let mut inner = self.value.lock().unwrap();

        if let Some(value) = inner.result.take() {
            Poll::Ready(value)
        } else {
            inner.waker = Some(context.waker().clone());
            Poll::Pending
        }
    }
}