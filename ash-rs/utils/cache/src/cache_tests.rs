use std::cell::Cell;
use std::num::NonZeroUsize;

use super::BlockingLruCache;
use super::sha1_digest;

#[tokio::test(flavor = "multi_thread")]
async fn stores_reuses_and_removes_values() {
    let cache = BlockingLruCache::new(NonZeroUsize::new(2).expect("non-zero capacity"));
    let calls = Cell::new(0);

    assert_eq!(
        cache.get_or_insert_with("first", || {
            calls.set(calls.get() + 1);
            1
        }),
        1
    );
    assert_eq!(cache.get_or_insert_with("first", || 2), 1);
    assert_eq!(calls.get(), 1);
    assert_eq!(cache.insert("first", 3), Some(1));
    assert_eq!(cache.remove(&"first"), Some(3));
    assert_eq!(cache.get(&"first"), None);
}

#[tokio::test(flavor = "multi_thread")]
async fn evicts_the_least_recently_used_entry() {
    let cache = BlockingLruCache::new(NonZeroUsize::new(2).expect("non-zero capacity"));
    cache.insert("a", 1);
    cache.insert("b", 2);
    assert_eq!(cache.get(&"a"), Some(1));

    cache.insert("c", 3);

    assert_eq!(cache.get(&"b"), None);
    assert_eq!(cache.get(&"a"), Some(1));
    assert_eq!(cache.get(&"c"), Some(3));
}

#[tokio::test(flavor = "multi_thread")]
async fn fallible_factory_only_caches_success() {
    let cache = BlockingLruCache::new(NonZeroUsize::new(1).expect("non-zero capacity"));

    assert_eq!(
        cache.get_or_try_insert_with("key", || Err::<usize, _>("failed")),
        Err("failed")
    );
    assert_eq!(cache.get(&"key"), None);
    assert_eq!(
        cache.get_or_try_insert_with("key", || Ok::<_, &str>(7)),
        Ok(7)
    );
    assert_eq!(
        cache.get_or_try_insert_with("key", || Ok::<_, &str>(8)),
        Ok(7)
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn exposes_mutation_clear_and_guard_operations() {
    let cache = BlockingLruCache::new(NonZeroUsize::new(2).expect("non-zero capacity"));

    cache.with_mut(|inner| {
        inner.put("first", 1);
    });
    assert_eq!(cache.get(&"first"), Some(1));
    drop(cache.blocking_lock().expect("multi-thread runtime guard"));
    cache.clear();
    assert_eq!(cache.get(&"first"), None);
}

#[test]
fn zero_capacity_disables_construction() {
    assert!(BlockingLruCache::<String, String>::try_with_capacity(0).is_none());
    assert!(BlockingLruCache::<String, String>::try_with_capacity(1).is_some());
}

#[test]
fn operations_do_not_persist_without_a_runtime() {
    let cache = BlockingLruCache::new(NonZeroUsize::new(2).expect("non-zero capacity"));
    cache.insert("first", 1);
    assert_eq!(cache.get(&"first"), None);

    assert_eq!(cache.get_or_insert_with("first", || 2), 2);
    assert_eq!(cache.get(&"first"), None);
    assert_eq!(cache.remove(&"first"), None);
    cache.clear();

    let result = cache.with_mut(|inner| {
        inner.put("temporary", 3);
        inner.get(&"temporary").copied()
    });
    assert_eq!(result, Some(3));
    assert_eq!(cache.get(&"temporary"), None);
    assert!(cache.blocking_lock().is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn operations_do_not_panic_on_a_current_thread_runtime() {
    let cache = BlockingLruCache::new(NonZeroUsize::new(1).expect("non-zero capacity"));

    cache.insert("key", 1);

    assert_eq!(cache.get(&"key"), None);
    assert_eq!(cache.get_or_insert_with("key", || 2), 2);
    assert!(cache.blocking_lock().is_none());
}

#[test]
fn computes_the_standard_sha1_digest() {
    assert_eq!(
        sha1_digest(b"abc"),
        [
            0xa9, 0x99, 0x3e, 0x36, 0x47, 0x06, 0x81, 0x6a, 0xba, 0x3e, 0x25, 0x71, 0x78, 0x50,
            0xc2, 0x6c, 0x9c, 0xd0, 0xd8, 0x9d,
        ]
    );
}
