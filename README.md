# goyda

A Rust UI toolkit that compiles the same app to Windows, Android, and web (wasm), with hot reload while you work.

```rust
use goyda::prelude::{page, stack, Component, Color};

#[page("/")]
pub fn counter_page() -> Component {
    let mut count = 0;

    stack! {
        direction: Vertical,
        spacing: 16,

        text { "Count: ", count }
            .color(Color::GRAY),

        button {
            text: "+1",
            on_click: count += 1,
        }
            .background(Color::GREEN)
            .px(4)
            .py(2)
            .rounded(2)
    }
    .p(24)
}
```

Install the CLI:

```bash
cargo install goy
```

Scaffold a project:

```bash
goy new my-app
cd my-app
goy run windows
```

`r` hot-reloads, `R` does a full reload, `q` quits.

## License

MIT
