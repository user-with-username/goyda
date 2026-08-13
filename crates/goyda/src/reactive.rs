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
    /// Every [`Signal::new_keyed`] call's value cell, kept alive for the
    /// life of the process (not just the current mount) so that leaving a
    /// `#[page(...)]` and navigating back to it - or, on windows, a
    /// hot-reload dylib swap (see `goyda::windows::hot_swap_dylib`) -
    /// resumes with the same live value instead of starting over. Only the
    /// value cell is kept, never the whole [`Signal`]: the *subscribers*
    /// list has to be fresh every mount regardless, since the old list is
    /// full of effect closures capturing the previous (now torn-down) view
    /// tree - reusing those would try to update `View`s/`HWND`s that no
    /// longer exist.
    static PERSISTENT_SIGNALS: RefCell<HashMap<String, Rc<dyn Any>>> = RefCell::new(HashMap::new());
}

pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    subscribers: Rc<RefCell<Vec<Rc<RefCell<dyn FnMut()>>>>>,
}

impl<T: Clone + 'static> Signal<T> {
    pub fn new(val: T) -> Self {
        Self {
            value: Rc::new(RefCell::new(val)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }

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

    pub fn set(&self, new_val: T) {
        *self.value.borrow_mut() = new_val;
        self.notify();
    }

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

impl<T: Clone + 'static> Signal<T> {
    /// Like [`Signal::new`], except `key` (a stable identity - the same
    /// string every time this same `#[page(...)]` declaration is mounted,
    /// baked in at compile time by `goyda_derive::transform_reactive_fn`)
    /// lets the value survive being un-mounted and re-mounted: `init` only
    /// actually gets used the *first* time this key is ever seen (a genuine
    /// cold start, or a declaration that's new since the last mount);
    /// every later mount of the same page reuses the exact same
    /// `Rc<RefCell<T>>` a previous mount already created, picking up
    /// wherever that value was last left, no serialization involved - it
    /// never actually stopped existing, just stopped being *displayed*
    /// while some other page was mounted instead.
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

pub struct Memo<T> {
    signal: Signal<T>,
    _effect: Rc<RefCell<dyn FnMut()>>,
}

impl<T: Clone + 'static> Memo<T> {
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

pub fn create_effect(f: impl FnMut() + 'static) {
    let effect_closure = Rc::new(RefCell::new(f));
    
    EFFECTS.with(|effects| {
        effects.borrow_mut().push(effect_closure.clone());
    });

    run_with_effect(effect_closure);
}

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
