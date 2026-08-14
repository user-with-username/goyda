use crate::components::LayoutDirection;
use crate::core::events::Update;

pub trait Backend {
    type PlatformView: Clone + 'static;
    type Updater: BackendUpdater<PlatformView = Self::PlatformView> + 'static;

    fn create_text(&mut self, content: &str) -> Self::PlatformView;
    fn create_button(&mut self, text: &str) -> Self::PlatformView;
    fn create_image(&mut self, asset: &crate::components::Asset) -> Self::PlatformView;
    fn create_stack(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView;
    fn create_scroll_view(
        &mut self,
        direction: LayoutDirection,
        spacing: i32,
        children: Vec<Self::PlatformView>,
    ) -> Self::PlatformView;
    /// Every child positioned at its own `Axis::OffsetX`/`Axis::OffsetY`,
    /// stacked by `Axis::ZIndex` - `position: absolute` in CSS terms.
    fn create_overlay(&mut self, children: Vec<Self::PlatformView>) -> Self::PlatformView;
    fn create_text_input(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView;
    fn create_textarea(&mut self, placeholder: &str, initial_text: &str) -> Self::PlatformView;
    fn create_checkbox(&mut self, label: &str, checked: bool) -> Self::PlatformView;
    /// A single option in a mutually-exclusive `group` - selecting it
    /// deselects every other control created with the same `group` string.
    fn create_radio_button(
        &mut self,
        group: &str,
        label: &str,
        selected: bool,
    ) -> Self::PlatformView;
    fn create_switch(&mut self, checked: bool) -> Self::PlatformView;
    fn create_progress(&mut self, value: f32) -> Self::PlatformView;
    fn create_spacer(&mut self, size: i32) -> Self::PlatformView;
    fn create_divider(&mut self) -> Self::PlatformView;
    fn clone_updater(&self) -> Self::Updater;
    fn apply_style(&mut self, view: &Self::PlatformView, style: crate::components::StyleProperty);
}

pub trait BackendUpdater {
    type PlatformView;
    fn apply_update(&mut self, view: &Self::PlatformView, update: Update);
}
