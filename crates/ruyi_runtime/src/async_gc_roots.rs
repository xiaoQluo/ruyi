/**
 * GC root registration for suspended async tasks.
 *
 * Tracks `TaskId` values for tasks that have parked on an `await` point
 * (or any other suspension). The collector consults this registry during
 * the mark phase so that GC objects referenced only from a suspended
 * future are not reclaimed.
 *
 * The module exposes a tiny C ABI (`ruyi_async_register_root` /
 * `ruyi_async_unregister_root`) so the runtime can hook the
 * suspend/resume paths without taking a dependency on the higher-level
 * `async_runtime` scheduler API.
 *
 * @author Ruyi Team
 * @date 2026-07-11
 */
use std::collections::HashSet;
use std::sync::Mutex;

static ROOT_IDS: Mutex<Option<HashSet<usize>>> = Mutex::new(None);

/// Initialise the registry exactly once. Subsequent calls are no-ops,
/// which keeps the entry points idempotent under repeated FFI calls.
pub fn init() {
    let mut guard = ROOT_IDS.lock().unwrap();
    if guard.is_none() {
        *guard = Some(HashSet::new());
    }
}

/// Register a task id as a current GC root. Safe to call multiple times
/// for the same id; the entry point is idempotent.
pub fn register(task_id: usize) {
    init();
    ROOT_IDS
        .lock()
        .unwrap()
        .as_mut()
        .expect("registry initialised by init()")
        .insert(task_id);
}

/// Remove a task id from the registry. No-op if the id was not registered.
pub fn unregister(task_id: usize) {
    if let Some(set) = ROOT_IDS.lock().unwrap().as_mut() {
        set.remove(&task_id);
    }
}

/// Snapshot the currently registered task ids. The returned vector is
/// ordered by insertion (HashSet iteration order) and is consumed by the
/// collector to filter the per-task GC pointer scan.
pub fn snapshot() -> Vec<usize> {
    ROOT_IDS
        .lock()
        .unwrap()
        .as_ref()
        .map(|set| set.iter().copied().collect())
        .unwrap_or_default()
}

/// Reset the registry. Intended for use between tests; not exposed via FFI.
#[cfg(test)]
pub fn reset_for_tests() {
    let mut guard = ROOT_IDS.lock().unwrap();
    if let Some(set) = guard.as_mut() {
        set.clear();
    }
}

/// FFI entry point — register a suspended task id as a GC root.
///
/// # Safety
/// The caller must ensure `task_id` originates from `TaskId.0` of a task
/// that is currently parked (i.e. returned `Poll::Pending`).
#[no_mangle]
pub extern "C" fn ruyi_async_register_root(task_id: usize) {
    register(task_id);
}

/// FFI entry point — remove a task id from the GC root registry. Called
/// when a task resumes or completes so its future's pointer fields no
/// longer extend the lifetime of any GC object.
#[no_mangle]
pub extern "C" fn ruyi_async_unregister_root(task_id: usize) {
    unregister(task_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_unregister_track_ids() {
        reset_for_tests();
        register(42);
        register(43);
        let snap = snapshot();
        assert!(snap.contains(&42));
        assert!(snap.contains(&43));

        unregister(42);
        let snap = snapshot();
        assert!(!snap.contains(&42));
        assert!(snap.contains(&43));

        reset_for_tests();
        assert!(snapshot().is_empty());
    }

    #[test]
    fn unregister_unknown_is_noop() {
        reset_for_tests();
        unregister(9999);
        assert!(snapshot().is_empty());
    }
}