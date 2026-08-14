use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use crate::core::Backend;
use crate::core::events::Update;
use crate::core::backend::BackendUpdater;

thread_local! {
    static RUNNING_EFFECT: RefCell<Option<Rc<RefCell<dyn FnMut()>>>> = RefCell::new(None);
    static EFFECTS: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>> = RefCell::new(Vec::new());
    static PERSISTENT_SIGNALS: RefCell<HashMap<String, Rc<dyn Any>>> = RefCell::new(HashMap::new());
}

#[cfg(target_arch = "wasm32")]
thread_local! {
    static DUMP_FNS: RefCell<HashMap<String, Box<dyn Fn() -> Option<String>>>> = RefCell::new(HashMap::new());
    static RESTORED_STATE: RefCell<Option<HashMap<String, String>>> = RefCell::new(None);
}

/// A reactive value: reading it inside an effect (such as a component's
/// dynamic property) subscribes that effect to future changes, and setting
/// it reruns every subscriber.
///
/// ```ignore
/// let count = Signal::new(0);
/// count.set(count.get() + 1);
/// ```
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>>,
}

impl<T: Clone + 'static> Signal<T> {
    /// Creates a signal holding `val`.
    pub fn new(val: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(val)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

    /// Reads the current value, subscribing the enclosing effect (if any)
    /// to future changes.
    pub fn get(&self) -> T {
        RUNNING_EFFECT.with(|current| {
            if let Some(effect) = &*current.borrow() {
                let mut subs = self.subscribers.borrow_mut();
                if !subs.iter().any(|s| Rc::ptr_eq(s, effect)) {
                    subs.push(effect.clone());
                }
            }
        });
        self.value.borrow().clone()
    }

    /// Replaces the value and notifies subscribers.
    pub fn set(&self, new_val: T) {
        *self.value.borrow_mut() = new_val;
        self.notify();
    }

    /// Mutates the value in place via `f` and notifies subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        f(&mut *self.value.borrow_mut());
        self.notify();
    }

    fn notify(&self) {
        let subs = self.subscribers.borrow().clone();
        for sub in subs {
            run_with_effect(sub.clone());
        }
    }
}

impl<T: Clone + PartialEq + 'static> Signal<T> {
    /// Mutates the value via `f` and notifies subscribers only if the value
    /// actually changed.
    pub fn call<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        RUNNING_EFFECT.with(|current| {
            if let Some(effect) = &*current.borrow() {
                let mut subs = self.subscribers.borrow_mut();
                if !subs.iter().any(|s| Rc::ptr_eq(s, effect)) {
                    subs.push(effect.clone());
                }
            }
        });
        let before = self.value.borrow().clone();
        let result = f(&mut *self.value.borrow_mut());
        if *self.value.borrow() != before {
            self.notify();
        }
        result
    }
}

impl<T: Clone + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            subscribers: self.subscribers.clone(),
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: Clone + 'static> Signal<T> {
    /// Like [`Signal::new`], but keeps its value across the signal being
    /// un-mounted and re-mounted under the same `key` (for example,
    /// navigating away from a page and back). `init` is only used the
    /// first time a given key is seen.
    pub fn new_keyed(key: &'static str, init: T) -> Self {
        let value = PERSISTENT_SIGNALS.with(|store| {
            let mut store = store.borrow_mut();
            if let Some(existing) = store.get(key).and_then(|rc| rc.clone().downcast::<RefCell<T>>().ok()) {
                existing
            } else {
                let fresh = Rc::new(RefCell::new(init));
                store.insert(key.to_string(), fresh.clone());
                fresh
            }
        });

        Self {
            value,
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

#[cfg(target_arch = "wasm32")]
impl<T: Clone + serde::Serialize + serde::de::DeserializeOwned + 'static> Signal<T> {
    /// Like [`Signal::new`], but keeps its value across the signal being
    /// un-mounted and re-mounted under the same `key`, and survives a page
    /// hot reload. `init` is only used the first time a given key is seen.
    pub fn new_keyed(key: &'static str, init: T) -> Self {
        let value = PERSISTENT_SIGNALS.with(|store| {
            let mut store = store.borrow_mut();
            if let Some(existing) = store.get(key).and_then(|rc| rc.clone().downcast::<RefCell<T>>().ok()) {
                return existing;
            }

            let restored = RESTORED_STATE.with(|r| {
                r.borrow().as_ref().and_then(|m| m.get(key)).and_then(|json| serde_json::from_str::<T>(json).ok())
            });
            let fresh = Rc::new(RefCell::new(restored.unwrap_or(init)));
            store.insert(key.to_string(), fresh.clone());

            let dump_value = fresh.clone();
            DUMP_FNS.with(|d| {
                d.borrow_mut().insert(key.to_string(), Box::new(move || serde_json::to_string(&*dump_value.borrow()).ok()));
            });

            fresh
        });

        Self {
            value,
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

/// Snapshots every keyed signal's current value into a JSON string, for
/// restoring with [`install_state`] after a hot reload.
#[cfg(target_arch = "wasm32")]
pub fn dump_state() -> String {
    DUMP_FNS.with(|d| {
        let fns = d.borrow();
        let map: HashMap<&str, String> = fns.iter().filter_map(|(k, f)| f().map(|v| (k.as_str(), v))).collect();
        serde_json::to_string(&map).unwrap_or_default()
    })
}

/// Restores keyed signal values previously captured with [`dump_state`].
#[cfg(target_arch = "wasm32")]
pub fn install_state(json: &str) {
    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(json) {
        RESTORED_STATE.with(|r| *r.borrow_mut() = Some(map));
    }
}

fn run_with_effect(effect: Rc<RefCell<dyn FnMut()>>) {
    let prev = RUNNING_EFFECT.with(|current| current.borrow().clone());
    
    RUNNING_EFFECT.with(|current| {
        *current.borrow_mut() = Some(effect.clone());
    });

    effect.borrow_mut()();

    RUNNING_EFFECT.with(|current| {
        *current.borrow_mut() = prev;
    });
}

/// A computed value that recalculates whenever any [`Signal`] it reads
/// changes.
///
/// ```ignore
/// let count = Signal::new(1);
/// let doubled = Memo::new({
///     let count = count.clone();
///     move || count.get() * 2
/// });
/// ```
pub struct Memo<T> {
    signal: Signal<T>,
    _effect: Rc<RefCell<dyn FnMut()>>,
}

impl<T: Clone + 'static> Memo<T> {
    /// Creates a memo whose value is `f()`, recomputed whenever a signal
    /// read inside `f` changes.
    pub fn new(mut f: impl FnMut() -> T + 'static) -> Self {
        let signal = Signal::new(f());
        let signal_clone = signal.clone();

        let effect = Rc::new(RefCell::new(move || {
            let new_value = f();
            signal_clone.set(new_value);
        }));

        EFFECTS.with(|effects| {
            effects.borrow_mut().push(effect.clone());
        });

        run_with_effect(effect.clone());

        Self { signal, _effect: effect }
    }

    /// Reads the current value, subscribing the enclosing effect (if any)
    /// to future changes.
    pub fn get(&self) -> T {
        self.signal.get()
    }
}

impl<T: Clone + 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal.clone(),
            _effect: self._effect.clone(),
        }
    }
}

/// Runs `f` immediately and again every time a [`Signal`] it reads changes.
pub fn create_effect(f: impl FnMut() + 'static) {
    let effect_closure = Rc::new(RefCell::new(f));
    
    EFFECTS.with(|effects| {
        effects.borrow_mut().push(effect_closure.clone());
    });

    run_with_effect(effect_closure);
}

/// Applies a computed value to a mounted view, and keeps it in sync
/// whenever a [`Signal`] read inside `compute` changes.
pub fn reactive<V, U, B, T: 'static>(backend: &B, view: &V, compute: Rc<dyn Fn() -> T + 'static>, mut make_update: U)
where
    V: Clone + 'static,
    U: FnMut(T) -> Update + 'static,
    B: Backend<PlatformView = V>,
{
    let view_clone = view.clone();
    let mut backend_updater = backend.clone_updater();
    let compute_clone = compute.clone();

    create_effect(move || {
        let new_value = (compute_clone)();
        let update = make_update(new_value);
        backend_updater.apply_update(&view_clone, update);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keyed_signal_survives_remount() {
        let s1 = Signal::new_keyed("page#0", 0i32);
        s1.set(42);
        drop(s1);

        // A later "remount" (fresh Signal::new_keyed call, same key) should
        // pick up 42, not fall back to init.
        let s2 = Signal::new_keyed("page#0", 0i32);
        assert_eq!(s2.get(), 42);
    }

    #[test]
    fn keyed_signal_new_key_uses_init() {
        let s = Signal::new_keyed("page#never_before_seen_key", 7i32);
        assert_eq!(s.get(), 7);
    }
}
