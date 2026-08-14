use anyhow::Result;
use std::fs;

use super::AndroidAppLayout;

const MANIFEST_TEMPLATE: &str = include_str!("../../../templates/AndroidManifest.xml");
const GOYDA_JAVA_TEMPLATE: &str = include_str!("../../../templates/Goyda.java");
const MAIN_ACTIVITY_TEMPLATE: &str = include_str!("../../../templates/MainActivity.java");
const HOT_RELOAD_SWAP_RECEIVER_TEMPLATE: &str =
    include_str!("../../../templates/HotReloadSwapReceiver.java");

pub fn generate_assets_from_templates(
    layout: &AndroidAppLayout,
    app_name: &str,
    lib_name: &str,
    release: bool,
) -> Result<()> {
    // `debuggable` gates `adb shell run-as` access to the app's private
    // storage - needed for `goy run android`'s quick-reload path to push a
    // freshly rebuilt `.so` generation there and `System.load()` it into
    // the still-running process (see `goyda-cli`'s android quick-reload).
    // `false` for `--release` so a real release build never ships this.
    let manifest_rendered = MANIFEST_TEMPLATE
        .replace("{package_name}", &layout.package_name)
        .replace("{app_name}", app_name)
        .replace("{debuggable}", if release { "false" } else { "true" });
    fs::write(layout.manifest(), manifest_rendered)?;

    let goyda_rendered = GOYDA_JAVA_TEMPLATE
        .replace("{package_name}", &layout.package_name)
        .replace("{lib_name}", lib_name);
    fs::write(layout.java_src_dir().join("Goyda.java"), goyda_rendered)?;

    let main_activity_rendered =
        MAIN_ACTIVITY_TEMPLATE.replace("{package_name}", &layout.package_name);
    fs::write(
        layout.java_src_dir().join("MainActivity.java"),
        main_activity_rendered,
    )?;

    let hot_reload_swap_rendered =
        HOT_RELOAD_SWAP_RECEIVER_TEMPLATE.replace("{package_name}", &layout.package_name);
    fs::write(
        layout.java_src_dir().join("HotReloadSwapReceiver.java"),
        hot_reload_swap_rendered,
    )?;

    Ok(())
}
