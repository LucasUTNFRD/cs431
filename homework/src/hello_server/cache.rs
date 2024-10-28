//! Thread-safe key/value cache.

use std::collections::hash_map::{Entry, HashMap};
use std::hash::Hash;
use std::sync::{Arc, Mutex, RwLock};

/// Cache that remembers the result for each key.
#[derive(Debug)]
pub struct Cache<K, V> {
    /// The cache should be thread-safe and support multiple readers and writers concurrently.
    /// Uses a `RwLock` to allow multiple readers or one writer at a time.
    /// Stores a `HashMap` of keys to `Arc<Mutex<Option<V>>>` to allow for mutable access to the
    /// value.
    /// Arc allows sharing the mutex across threads.
    /// Mutex provides exclusive access to the value.
    /// Option<V> allows for the value to be `None` if it has not been inserted yet.
    inner: RwLock<HashMap<K, Arc<Mutex<Option<V>>>>>,
}

impl<K, V> Default for Cache<K, V> {
    fn default() -> Self {
        Self {
            inner: RwLock::new(HashMap::new()),
        }
    }
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    /// Retrieve the value or insert a new one created by `f`.
    ///
    /// An invocation to this function should not block another invocation with a different key. For
    /// example, if a thread calls `get_or_insert_with(key1, f1)` and another thread calls
    /// `get_or_insert_with(key2, f2)` (`key1≠key2`, `key1,key2∉cache`) concurrently, `f1` and `f2`
    /// should run concurrently.
    ///
    /// On the other hand, since `f` may consume a lot of resource (= money), it's undesirable to
    /// duplicate the work. That is, `f` should be run only once for each key. Specifically, even
    /// for concurrent invocations of `get_or_insert_with(key, f)`, `f` is called only once per key.
    ///
    /// Hint: the [`Entry`] API may be useful in implementing this function.
    ///
    /// [`Entry`]: https://doc.rust-lang.org/stable/std/collections/hash_map/struct.HashMap.html#method.entry
    pub fn get_or_insert_with<F: FnOnce(K) -> V>(&self, key: K, f: F) -> V {
        // todo!()
        // acquire the write lock and check if the key is already in the cache, if not already in
        // the cache create a place holder for the value and insert it into the cache
        let mut cache = self.inner.write().unwrap();
        let mut value = cache
            .entry(key.clone())
            .or_insert_with(|| Arc::new(Mutex::new(None)))
            .clone();
        //release the write lock
        drop(cache);
        //acquire the lock and if the val is a placeholder, compute the value and store it in the
        // cache
        let mut slot = value.lock().unwrap();
        if let Some(val) = slot.as_ref() {
            val.clone()
        } else {
            let new_val = f(key.clone());
            *slot = Some(new_val.clone());
            new_val
        }
    }
}

// // Thread 1                          | Thread 2
// check_cache("image.jpg")            | check_cache("image.jpg")
// -> No entry, create placeholder     | -> Found placeholder (None)
// -> Set placeholder to None          | -> Wait for Thread 1
// -> Start downloading               |
// -> Store result in placeholder     |
// -> Return image                    | -> Get completed result
//                                   | -> Return same image

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_or_insert_with() {
        let cache = Cache::default();
        let result = cache.get_or_insert_with("key1", |k| k.to_string());
        assert_eq!(result, "key1");
    }
}
