use bus::Bus;
use log::debug;
use std::{
    collections::HashMap,
    fmt::Display,
    hash::Hash,
    sync::{mpsc::RecvError, Arc, Mutex, PoisonError},
};
use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum CoalescingError {
    #[error("poison error")]
    PoisonError,
    #[error("bus closed")]
    RecvError(#[from] RecvError),
}

impl<T> From<PoisonError<T>> for CoalescingError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

pub struct Coalescer<K, T>
where
    // that's a lot of Clones, I know
    K: Clone + Display + Eq + Hash,
    T: Clone + Sync,
{
    // it would be nice to use RwLock instead of Mutex here,
    // but RwLock just won't cut it. we need to check whether
    // the key exists in HashMap and insert it if it doesn't
    // under a single lock, otherwise a race condition might occur
    inflight: Mutex<HashMap<K, Arc<Mutex<Bus<T>>>>>,
}

impl<Key, ReturnType> Coalescer<Key, ReturnType>
where
    Key: Clone + Display + Eq + Hash,
    ReturnType: Clone + Sync,
{
    pub fn new() -> Self {
        Coalescer {
            inflight: Mutex::new(HashMap::new()),
        }
    }

    pub fn execute<F: FnOnce() -> ReturnType>(
        &self,
        key: Key,
        f: F,
    ) -> Result<ReturnType, CoalescingError> {
        let mut inflight = self.inflight.lock()?;

        if let Some(entry) = inflight.get(&key) {
            let mut rx = { entry.lock()?.add_rx() };
            drop(inflight);

            debug!("task with key \"{}\" is already in progress", &key);

            return Ok(rx.recv()?.clone());
        }

        let bus = Arc::new(Mutex::new(Bus::<ReturnType>::new(1)));
        inflight.insert(key.clone(), bus.clone());
        drop(inflight);

        debug!("starting a new task with key \"{}\"", &key);

        let result = f();

        {
            // this is in a separate scope because
            // the guard must be dropped ASAP
            self.inflight.lock()?.remove(&key);
        }

        bus.lock()?.broadcast(result.clone());

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::{thread_rng, RngCore};
    use std::{
        thread::{self, JoinHandle},
        time::Duration,
    };

    #[test]
    fn test_coalesces_tasks() {
        fn do_work() -> String {
            let random_number = thread_rng().next_u32();
            thread::sleep(Duration::from_millis(200));
            random_number.to_string()
        }

        let coalescer = Arc::new(Coalescer::<String, String>::new());

        let mut threads: Vec<JoinHandle<String>> = vec![];

        for _ in 0..3 {
            let coalescer = Arc::clone(&coalescer);

            let handle = thread::spawn(move || {
                let result = coalescer.execute("some-key".into(), || {
                    return do_work();
                });

                result.expect("fatal coalescing error")
            });

            threads.push(handle)
        }

        let mut results: Vec<String> = vec![];

        for handle in threads {
            results.push(handle.join().expect("fatal join error"));
        }

        let first: &String = &(results[0]);

        for item in &results {
            assert_eq!(first, item);
        }
    }
}
