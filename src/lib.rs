use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

struct Job {
    f: Box<dyn FnOnce() + Send + 'static>,
}

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let workers = (0..size)
            .map(|id| Worker::new(id, Arc::clone(&receiver)))
            .collect();

        Self {
            workers,
            sender: Some(sender),
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + 'static + Send,
    {
        let job = Job { f: Box::new(f) };
        if let Some(sender) = self.sender.as_ref() {
            sender.send(job).unwrap();
        }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in self.workers.drain(..) {
            drop(worker);
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
    id: usize,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = Some(thread::spawn(move || loop {
            let job = receiver.lock().unwrap().recv();

            if let Ok(job) = job {
                (job.f)();
            } else {
                println!("Worker {} is exiting", id);
                break;
            }
        }));

        Self { thread, id }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        println!("Worker {} is dropping", self.id);
        if let Some(thread) = self.thread.take() {
            thread.join().unwrap();
        }
    }
}
