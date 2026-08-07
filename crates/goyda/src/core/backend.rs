use crate::components::LayoutDirection;
use crate::core::events::Update;

pub trait Backend {
    type PlatformView: Clone + 'static;
    type Updater: BackendUpdater<PlatformView = Self::PlatformView> + 'static;

    fn create_text(&mut self, content: &str) -> Self::PlatformView;
    fn create_button(&mut self, text: &str) -> Self::PlatformView;
    fn create_stack(&mut self, direction: LayoutDirection, spacing: i32, children: Vec<Self::PlatformView>) -> Self::PlatformView;
    fn clone_updater(&self) -> Self::Updater;
    fn apply_style(&mut self, view: &Self::PlatformView, style: crate::components::StyleProperty);

}

pub trait BackendUpdater {
    type PlatformView;
    fn apply_update(&mut self, view: &Self::PlatformView, update: Update);
}
