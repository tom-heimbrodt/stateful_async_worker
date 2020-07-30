use std::thread;
use std::future::Future;
use std::sync::Arc;
use crossbeam::channel::{self, Sender};

use crate::*;

type TaskFunc<ThreadState, Result> = dyn FnOnce(&ThreadState)
    -> Result + Send + 'static;
type BoxedTaskFunc<ThreadState, Result> = Box<TaskFunc<ThreadState, Result>>;

/// Abstracts a pool of stateful background worker threads, that can run
/// synchronous functions and provide the return value as asynchronous future.
/// 
/// The thread pool owns an arbitrary `ThreadState`, which will be passed as
/// a reference to the called function.
/// Thus the asynchronous code can easily access and mutate state when
/// necessary.
///
/// Unlike a single worker thread, the thread pool can process multiple
/// function at a time.
/// This, however, forces the closures to take a non-mutable
/// reference to the shared state.
/// For mutable access interior mutability, e.g. via RwLock, has to be used.
pub struct ThreadPool<ThreadState, Result> {
    sender: Sender<Task<BoxedTaskFunc<ThreadState, Result>, Result>>,
}

impl<ThreadState, Result> Clone for ThreadPool<ThreadState, Result> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

impl<ThreadState, Result> ThreadPool<ThreadState, Result>
where ThreadState: Default + Send + Sync + 'static,
      Result: Send + 'static {

    /// Spawns a new thread for every logical CPU core. The state will be
    /// initialized with
    /// [`Default::default()`](https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default).
    pub fn spawn() -> Self {
        Self::spawn_with(Arc::new(Default::default()))
    }

    /// Spawns exactly `thread_count` threads. The state will be
    /// initialized with
    /// [`Default::default()`](https://doc.rust-lang.org/std/default/trait.Default.html#tymethod.default).
    pub fn spawn_exactly(thread_count: usize) -> Self {
        Self::spawn_exactly_with(Arc::new(Default::default()), thread_count)
    }
}

impl<ThreadState, Result> ThreadPool<ThreadState, Result>
where ThreadState: Sync + Send + 'static,
      Result: Send + 'static {

    /// Spawns a new thread for every logical CPU core. The state will be
    /// initialized with `data`.
    pub fn spawn_with<T>(data: T) -> Self
    where T: Into<Arc<ThreadState>> {
        Self::spawn_exactly_with(data, num_cpus::get())
    }

    /// Spawns exactly `thread_count` threads. The state will be
    /// initialized with `data`.
    pub fn spawn_exactly_with<T>(data: T, thread_count: usize) -> Self
    where T: Into<Arc<ThreadState>> {
        let (input_tx, input_rx) = channel::unbounded();
        let data = data.into();

        for _ in 0..thread_count {
            let input_rx = input_rx.clone();
            let data = Arc::clone(&data);
            thread::spawn(move || {
                loop {
                    if let Ok(task) = input_rx.recv() {
                        let task: Task<BoxedTaskFunc<ThreadState, Result>, Result> = task;
                        let result = (task.func)(&*data);
                        task.future.complete(result);
                    } else {
                        return;
                    }
                }
            });
        }

        Self {
            sender: input_tx,
        }
    }

    /// Pass a synchronous function, so the thread pool can execute it.
    /// Execution will be begin even before the first call to poll.
    pub async fn work_on<F>(&self, func: F) -> Result
    where F: FnOnce(&ThreadState) -> Result + Send + 'static {
        self.work_on_boxed_inner(Box::new(func)).await
    }

    /// Like [`work_on`](#method.work_on) but for functions that are already boxed.
    pub async fn work_on_boxed(&self, func: BoxedTaskFunc<ThreadState, Result>) -> Result {
        self.work_on_boxed_inner(func).await
    }

    fn work_on_boxed_inner(&self, func: BoxedTaskFunc<ThreadState, Result>)
     -> impl Future<Output = Result> {
        let future = MutexFuture::new();
        let future_ = future.clone();

        self.sender.send(Task { func, future }).unwrap();
        future_
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_thread_pool_concurrency() {
        futures::executor::block_on(test_concurrency());
    }

    async fn test_concurrency() {
        let worker = ThreadPool::spawn_exactly(3);

        let long_computation1 = worker.work_on(|num: &i64| {
            thread::sleep(Duration::from_millis(100));
            *num
        });

        let long_computation2 = worker.work_on(|num: &i64| {
            thread::sleep(Duration::from_millis(100));
            *num
        });

        let long_computation3 = worker.work_on(|num: &i64| {
            thread::sleep(Duration::from_millis(50));
            *num
        });

        let start = Instant::now();

        let (a, b, c) = futures::future::join3(long_computation1, long_computation2, long_computation3).await;
        assert_eq!(a, 0);
        assert_eq!(b, 0);
        assert_eq!(c, 0);

        let elapsed = start.elapsed();
        assert!(elapsed.as_millis() < 120); // if the test took longer, the task weren't executed in parallel
    }

    #[test]
    fn test_thread_pool_rwlock() {
        futures::executor::block_on(test_rwlock());
    }

    use std::sync::RwLock;

    async fn test_rwlock() {
        let worker = ThreadPool::spawn_exactly_with(RwLock::new(0), 4);

        let long_computation1 = worker.work_on(|num: &RwLock<u64>| {
            thread::sleep(Duration::from_millis(80));
            let mut num = num.write().unwrap();
            *num += 1;
            *num

        });

        let long_computation2 = worker.work_on(|num| {
            thread::sleep(Duration::from_millis(100));
            let mut num = num.write().unwrap();
            *num += 1;
            *num
        });

        let long_computation3 = worker.work_on(|num| {
            thread::sleep(Duration::from_millis(60));
            let mut num = num.write().unwrap();
            *num += 1;
            *num
        });

        let long_computation4 = worker.work_on(|num| {
            thread::sleep(Duration::from_millis(20));
            let mut num = num.write().unwrap();
            *num += 1;
            *num
        });

        let (a, b, c, d) = futures::future::join4(
            long_computation1, 
            long_computation2, 
            long_computation3, 
            long_computation4).await;
        println!("{:?}", (a, b, c, d));
        assert_eq!(a, 3);
        assert_eq!(b, 4);
        assert_eq!(c, 2);
        assert_eq!(d, 1);
    }
}