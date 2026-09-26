use std::sync::{Condvar, Mutex};

/// A simple counting semaphore. Like, there are others out there, but this implementation
/// contains everything I need, and it saves me a crate.
pub struct Semaphore {
    lock: Mutex<usize>,
    cvar: Condvar,
}

impl Semaphore {
    /// Create a new semaphore with given size.
    pub fn new(size: usize) -> Self {
        Semaphore { lock: Mutex::new(size), cvar: Condvar::new() }
    }

    /// Aquire a lock to the semaphore. This will block, if there are no slots left to aquire.
    /// The returned SemaphoreGuard acts as RAII release-guard. To release your slot, simply
    /// drop the returned guard.
    pub fn acquire(&self) -> SemaphoreGuard<'_> {
        let mut current_size = self.lock.lock().unwrap();

        current_size = self.cvar.wait_while(current_size, |s| *s == 0).unwrap();
        *current_size -= 1;

        SemaphoreGuard(self)
    }

    fn release(&self) {
        let mut current_size = self.lock.lock().unwrap();

        *current_size += 1;

        self.cvar.notify_one();
    }
}

pub struct SemaphoreGuard<'a>(&'a Semaphore);

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.0.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;

    #[test]
    fn acquire_blocks_until_release() {
        let semaphore = Semaphore::new(1);
        let guard = semaphore.acquire();

        let (tx, rx) = mpsc::channel();
        let thread_sem = &semaphore;

        thread::scope(|s| {
            s.spawn(move || {
                let _guard = thread_sem.acquire();
                tx.send(()).unwrap();
            });

            assert!(rx.try_recv().is_err(), "thread acquired without release");
            drop(guard);
            rx.recv().expect("thread did not acquire after release");
        });
    }
}
