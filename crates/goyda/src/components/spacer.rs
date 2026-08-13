use crate::components::Component;

pub struct Spacer {
    pub size: i32,
}

impl Spacer {
    /// A fixed-size, invisible gap - `size` pixels along whichever axis the
    /// enclosing [`Stack`](crate::components::Stack) lays out on.
    pub fn new(size: i32) -> Component {
        Component::Spacer(Self { size })
    }
}
