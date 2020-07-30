Stateful async worker provides worker thread structures to allow
the execution of synchronous functions or closures in the background
and asynchronously access the result as future.

To execute multiple functions at the same time
[`ThreadPool`](src/thread_pool.rs) is
helpful. For inherently single threaded operations like disk I/O
[`WorkerThread`](src/worker_thread.rs) should be sufficient.

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