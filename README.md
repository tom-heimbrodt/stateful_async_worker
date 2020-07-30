[![Docs.rs](https://docs.rs/stateful_async_worker/badge.svg)](https://docs.rs/stateful_async_worker)
[![Crates.io](https://img.shields.io/crates/v/stateful_async_worker)](https://crates.io/crates/stateful_async_worker)
[![Crates.io](https://img.shields.io/crates/d/stateful_async_worker)](https://crates.io/crates/stateful_async_worker)
[![License](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](https://github.com/tom-heimbrodt/stateful_async_worker/blob/master/LICENSE)

Stateful async worker provides worker thread structures to allow
the execution of synchronous functions or closures in the background
and asynchronously access the result as future.

To execute multiple functions at the same time
[`ThreadPool`](https://docs.rs/stateful_async_worker/*/stateful_async_worker/struct.ThreadPool.html) is
helpful. For inherently single threaded operations like disk I/O
[`WorkerThread`](https://docs.rs/stateful_async_worker/*/stateful_async_worker/struct.WorkerThread.html) should be sufficient.

# Example
```
   use stateful_async_worker::WorkerThread;

   async fn example() -> u64 {
       // Create a worker thread that wraps a number.
       let worker = WorkerThread::spawn_with(0u64);

       // Now you can run closures as futures in the background!
       let add_three = worker.work_on(|num: &mut u64| {
           // Do some sophisticated computations here ...
           *num += 3;
           *num
       });
       add_three.await
   }
```