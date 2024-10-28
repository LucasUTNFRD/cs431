//! Thread pool that joins all thread when dropped.

// NOTE: Crossbeam channels are MPMC, which means that you don't need to wrap the receiver in
// Arc<Mutex<..>>. Just clone the receiver and give it to each worker thread.
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crossbeam_channel::{unbounded, Sender};

struct Job(Box<dyn FnOnce() + Send + 'static>);

#[derive(Debug)]
struct Worker {
    _id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Drop for Worker {
    /// When dropped, the thread's `JoinHandle` must be `join`ed.  If the worker panics, then this
    /// function should panic too.
    ///
    /// NOTE: The thread is detached if not `join`ed explicitly.
    fn drop(&mut self) {
        if let Some(thread) = self.thread.take() {
            thread.join().expect("worker thread panicked");
        }
    }
}

/// Internal data structure for tracking the current job status. This is shared by worker closures
/// via `Arc` so that the workers can report to the pool that it started/finished a job.
#[derive(Debug, Default)]
struct ThreadPoolInner {
    job_count: Mutex<usize>,
    empty_condvar: Condvar,
}

impl ThreadPoolInner {
    /// Increment the job count.
    fn start_job(&self) {
        // todo!()
        let mut job_count = self.job_count.lock().unwrap();
        *job_count += 1;
    }

    /// Decrement the job count.
    fn finish_job(&self) {
        let mut job_count = self.job_count.lock().unwrap();
        *job_count -= 1;
        if *job_count == 0 {
            self.empty_condvar.notify_all();
        }
    }

    /// Wait until the job count becomes 0.
    ///
    /// NOTE: We can optimize this function by adding another field to `ThreadPoolInner`, but let's
    /// not care about that in this homework.
    fn wait_empty(&self) {
        let mut job_count = self.job_count.lock().unwrap();
        while *job_count > 0 {
            job_count = self.empty_condvar.wait(job_count).unwrap();
        }
    }
}

/// Thread pool.
#[derive(Debug)]
pub struct ThreadPool {
    _workers: Vec<Worker>,
    job_sender: Option<Sender<Job>>,
    pool_inner: Arc<ThreadPoolInner>,
}

impl ThreadPool {
    /// Create a new ThreadPool with `size` threads.
    ///
    /// # Panics
    ///
    /// Panics if `size` is 0.
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (job_sender, job_receiver) = unbounded();

        let pool_inner = Arc::new(ThreadPoolInner {
            job_count: Mutex::new(0),
            empty_condvar: Condvar::new(),
        });

        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            let receiver = job_receiver.clone();
            let pool_inner_clone = Arc::clone(&pool_inner);
            let thread = thread::spawn(move || loop {
                let Job(x) = match receiver.recv() {
                    Ok(s) => s,
                    Err(_) => break,
                };
                x();
                pool_inner_clone.finish_job();
            });
            let worker = Worker {
                _id: id,
                thread: Some(thread),
            };
            workers.push(worker);
        }
        ThreadPool {
            _workers: workers,
            job_sender: Some(job_sender),
            pool_inner,
        }
    }

    /// Execute a new job in the thread pool.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Job(Box::new(f));
        if let Some(sender) = &self.job_sender {
            self.pool_inner.start_job();
            sender.send(job).unwrap();
        }
    }

    /// Block the current thread until all jobs in the pool have been executed.
    ///
    /// NOTE: This method has nothing to do with `JoinHandle::join`.
    pub fn join(&self) {
        // todo!()
        self.pool_inner.wait_empty();
    }
}

impl Drop for ThreadPool {
    /// When dropped, all worker threads' `JoinHandle` must be `join`ed. If the thread panicked,
    /// then this function should panic too.
    fn drop(&mut self) {
        if let Some(sender) = &self.job_sender {
            // drop(sender);
            let _ = sender;
        }
        // todo!()
    }
}
