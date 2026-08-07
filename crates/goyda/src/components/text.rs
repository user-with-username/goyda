use crate::components::Component;
use std::rc::Rc;

pub struct Text {
    pub compute: Rc<dyn Fn() -> String>,
}

impl Text {
    pub fn new(compute: impl Fn() -> String + 'static) -> Component {
        Component::Text(Self { compute: Rc::new(compute) })
    }
}
