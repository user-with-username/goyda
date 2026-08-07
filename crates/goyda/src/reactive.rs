use std::cell::RefCell;
use std::rc::Rc;
use crate::core::Backend;
use crate::core::events::Update;
use crate::core::backend::BackendUpdater;

thread_local! {
    static RUNNING_EFFECT: RefCell<Option<Rc<RefCell<dyn FnMut()>>>> = RefCell::new(None);
    static EFFECTS: RefCell<Vec<Rc<RefCell<dyn FnMut()>>>> = RefCell::new(Vec::new());
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

impl<T: Clone + 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone(),
            subscribers: self.subscribers.clone(),
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
