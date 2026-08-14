//! Tests for `src/lib.rs`: `Page` and `find_page`.

use goyda::{Component, Page, find_page};

fn home() -> Component {
    Component::text(|| "home".into())
}
fn about() -> Component {
    Component::text(|| "about".into())
}

goyda::inventory::submit! { Page::new("/", home) }
goyda::inventory::submit! { Page::new("/about", about) }

#[test]
fn finds_the_page_registered_for_an_exact_route() {
    let page = find_page("/about").expect("page should be registered");
    assert_eq!(page.route, "/about");
    match (page.factory)() {
        Component::Text(text) => assert_eq!((text.compute)(), "about"),
        _ => panic!("expected Component::Text"),
    }
}

#[test]
fn falls_back_to_the_root_route_for_an_unregistered_path() {
    let page = find_page("/does-not-exist").expect("should fall back to \"/\"");
    assert_eq!(page.route, "/");
}

#[test]
fn page_new_pairs_route_and_factory() {
    let page = Page::new("/x", home);
    assert_eq!(page.route, "/x");
    assert!(matches!((page.factory)(), Component::Text(_)));
}
