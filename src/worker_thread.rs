use std::thread;
use std::future::Future;
use crossbeam::channel::{self, Sender};

use crate::*;

type TaskFunc<ThreadState, Result> = dyn FnOnce(&mut ThreadState) -> Result + Send + 'static;
type BoxedTaskFunc<ThreadState, Result> = Box<TaskFunc<ThreadState, Result>>;

/// Abstracts a stateful background worker thread, that can run synchronous functions
/// and provide the return value as asynchronous future.
///
/// The thread owns an arbitrary `ThreadState`, which will be passed as
/// a mutable reference to the called function.
/// Thus the asynchronous code can easily access and mutate state when
/// necessary.
/// The worker will only process a single function at a time.
pub struct WorkerThread<ThreadState, Result> {
    sender: Sender<Task<BoxedTaskFunc<ThreadState, Result>, Result>>,
}

impl<ThreadState, Result> Clone for WorkerThread<ThreadState, Result> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<ThreadState, Result> WorkerThread<ThreadState, Result>
where ThreadState: Default + Send + 'static,
      Result: Send + 'static {

    /// Spawns a new worker thread. The state will be initialized with 
    /// [`Default::default()`](https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default).
    pub fn spawn() -> Self {
        Self::spawn_with(Default::default())
    }
}

impl<ThreadState, Result> WorkerThread<ThreadState, Result>
where ThreadState: Send + 'static,
      Result: Send + 'static {

    /// Spawns a new worker thread. The state will be initialized with
    /// `data`.
    pub fn spawn_with(mut data: ThreadState) -> Self {
        let (input_tx, input_rx) = channel::unbounded();

        thread::spawn(move || {
            loop {
                if let Ok(task) = input_rx.recv() {
                    let task: Task<BoxedTaskFunc<ThreadState, Result>, Result> = task;
                    let result = (task.func)(&mut data);
                    task.future.complete(result);
                } else {
                    return;
                }
            }
        });

        Self {
            sender: input_tx,
        }
    }

    /// Pass a synchronous function, so the worker thread can execute it.
    /// Execution will be begin even before the first call to poll.
    pub async fn work_on<F>(&self, func: F) -> Result
    where F: FnOnce(&mut ThreadState) -> Result + Send + 'static {
        self.work_on_boxed_inner(Box::new(func)).await
    }

    /// Like [`work_on`](#method.work_on) but for functions that are already boxed.
    pub async fn work_on_boxed(&self, func: BoxedTaskFunc<ThreadState, Result>) -> Result {
        self.work_on_boxed_inner(func).await
    }

    fn work_on_boxed_inner(&self, func: BoxedTaskFunc<ThreadState, Result>) -> impl Future<Output = Result> {
        let future = MutexFuture::new();
        let future_ = future.clone();

        self.sender.send(Task { func, future }).unwrap();
        future_
    }
}




#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_thread() {
        futures::executor::block_on(test());
    }

    async fn test() {
        let worker = WorkerThread::spawn();
        let add_three = worker.work_on(|num: &mut i64| {
            *num += 3;
            *num
        });

        let mult_two = worker.work_on(|num: &mut i64| {
            *num *= 2;
            *num
        });

        let result1 = add_three.await;
        let result2 = mult_two.await;

        let future3 = worker.work_on(|num: &mut i64| {
            *num *= -1;
            *num
        });

        let result3 = future3.await;

        assert_eq!(result1, 3);
        assert_eq!(result2, 6);
        assert_eq!(result3, -6);
    }
}