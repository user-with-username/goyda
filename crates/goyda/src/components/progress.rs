use crate::components::Component;
use std::rc::Rc;

pub struct Progress {
    pub compute: Rc<dyn Fn() -> f32>,
}

impl Progress {
    /// A horizontal progress bar/scrubber - clicking or dragging it sets a
    /// new value directly (report that back to your own state with
    /// `.on_value_changed(...)`), same as a native `<input type=range>`,
    /// Android `SeekBar`, or Windows trackbar. `compute` is re-run whenever
    /// a [`Signal`] or [`Memo`](crate::reactive::Memo) it reads changes, the
    /// same reactive binding [`crate::components::Text`] uses - values are
    /// clamped to `0.0..=1.0` by each backend.
    ///
    /// [`Signal`]: crate::reactive::Signal
    pub fn new(compute: impl Fn() -> f32 + 'static) -> Component {
        Component::Progress(Self { compute: Rc::new(compute) })
    }

    /// A fixed, non-reactive progress value.
    pub fn value(value: f32) -> Component {
        Self::new(move || value)
    }
}
