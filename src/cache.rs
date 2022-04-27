use std::{
    collections::HashMap,
    sync::{PoisonError, RwLock},
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Error, Clone, Debug)]
pub enum CachingError {
    #[error("poison error")]
    PoisonError,
}

impl<T> From<PoisonError<T>> for CachingError {
    fn from(_: PoisonError<T>) -> Self {
        Self::PoisonError
    }
}

#[derive(Clone)]
pub struct CacheEntry<T: Clone> {
    pub updated: Instant,
    pub value: T,
}

pub struct Cache<T: Clone> {
    timeout: Duration,
    inner: RwLock<HashMap<String, CacheEntry<T>>>,
}

impl<T: Clone> CacheEntry<T> {
    fn new(value: T) -> Self {
        CacheEntry {
            updated: Instant::now(),
            value,
        }
    }
}

impl<T: Clone> Cache<T> {
    pub fn new(timeout: Duration) -> Self {
        Cache {
            timeout,
            inner: RwLock::new(HashMap::new()),
        }
    }

    pub fn get(
        &self,
        key: &str,
        expiration: Option<Duration>,
    ) -> Result<Option<CacheEntry<T>>, CachingError> {
        let inner = self.inner.read()?;

        if let Some(entry) = inner.get(key) {
            let now = Instant::now();
            let age = now - entry.updated;
            let threshold = expiration.unwrap_or_else(|| self.timeout);

            if age <= threshold {
                return Ok(Some(entry.clone()));
            }
        }

        Ok(None)
    }

    pub fn set(&self, key: String, value: T) -> Result<(), CachingError> {
        let mut inner = self.inner.write()?;
        inner.insert(key, CacheEntry::new(value));
        Ok(())
    }
}
