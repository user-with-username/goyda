use goyda::prelude::{page, stack, Component, Color};

const COLOR_PRIMARY: Color = Color::Custom(0xFF3949AB);
const COLOR_MUTED: Color = Color::GRAY;
const COLOR_SUCCESS: Color = Color::GREEN;
const COLOR_DANGER: Color = Color::Custom(0xFFE53935);

const FONT_SIZE_2XL: i32 = 24;

#[page("/")]
pub fn counter_page() -> Component {
    let mut count = 0;
    let mut doubled = count * 2;
    // A plain Vec-backed signal — read and mutated through ordinary Vec
    // methods (`.len()`, `.push()`), no `.get()`/`.update()` required.
    let mut history: Vec<i32> = Vec::new();
    eprintln!("Rendering counter_page with count");

    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Current: ", count }
            .color(COLOR_PRIMARY)
            .font_size(FONT_SIZE_2XL),

        text { "Doubled: ", doubled }
            .color(COLOR_MUTED),

        // Comparisons used to be silently mangled into an assignment
        // because `count == 0` and `count = 0` looked the same to the
        // token scanner. `==`/`!=`/`<=`/`>=` are now read correctly.
        text { "At zero: ", count == 0 }
            .color(COLOR_MUTED),

        // No `.get()` needed: the macro routes this through `Signal::call`,
        // which just runs `.len()` under a mutable borrow and finds nothing
        // changed, so it reads silently — no method-name list involved.
        text { "Logged so far: ", history.len() }
            .color(COLOR_MUTED),

        button {
            text: "+1",
            on_click: count += 1,
        }
            .background(COLOR_SUCCESS)
            .px(4)
            .py(2)
            .rounded(2),

        button {
            text: "-1",
            on_click: count -= 1,
        }
            .background(COLOR_DANGER)
            .px(4)
            .py(2)
            .rounded(2),

        // `push` isn't in some hardcoded "mutating methods" list — there
        // isn't one anymore. `Signal::call` runs it under a mutable borrow,
        // compares the value before/after, and notifies because it changed.
        button {
            text: "Log count",
            on_click: history.push(count),
        }
            .background(COLOR_PRIMARY)
            .px(4)
            .py(2)
            .rounded(2),

        // The right-hand side of `=`/`+=` is now re-scanned too, so
        // referencing another reactive var there (not just a literal)
        // works: this resets `count` to `doubled`'s current value.
        button {
            text: "Reset to doubled",
            on_click: count = doubled,
        }
            .padding(8, 16)
            .border_color(COLOR_MUTED)
            .border_width(1)
            .opacity(0.8)
    }
    .p(6)
    .background(COLOR_DANGER)
}