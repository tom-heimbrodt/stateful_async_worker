//! Stateful async worker provides worker thread structures to allow
//! the execution of synchronous functions or closures in the background
//! and asynchronously access the result as future.
//! 
//! To execute multiple functions at the same time
//! [`ThreadPool`](struct.ThreadPool.html) is
//! helpful. For inherently single threaded operations like disk I/O
//! [`WorkerThread`](struct.WorkerThread.html) should be sufficient.
//!
//! # Example
//! ```
//!    use stateful_async_worker::WorkerThread;
//!
//!    async fn example() -> u64 {
//!        // Create a worker thread that wraps a number.
//!        let worker = WorkerThread::spawn_with(0u64);
//!
//!        // Now you can run closures as futures in the background!
//!        let add_three = worker.work_on(|num: &mut u64| {
//!            // Do some sophisticated computations here ...
//!            *num += 3;
//!            *num
//!        });
//!        add_three.await
//!    }
//! ```
mod mutex_future;
use mutex_future::*;

mod worker_thread;
pub use worker_thread::*;

mod thread_pool;
pub use thread_pool::*;

struct Task<F, Result> {
    func: F,
    future: MutexFuture<Result>,
}
