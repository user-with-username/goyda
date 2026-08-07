use anyhow::Result;
use std::fs;

use super::AndroidAppLayout;

const MANIFEST_TEMPLATE: &str = include_str!("../../../templates/AndroidManifest.xml");
const GOYDA_JAVA_TEMPLATE: &str = include_str!("../../../templates/Goyda.java");
const MAIN_ACTIVITY_TEMPLATE: &str = include_str!("../../../templates/MainActivity.java");

pub fn generate_assets_from_templates(
    layout: &AndroidAppLayout,
    app_name: &str,
    lib_name: &str,
) -> Result<()> {
    let manifest_rendered = MANIFEST_TEMPLATE
        .replace("{package_name}", &layout.package_name)
        .replace("{app_name}", app_name);
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

    Ok(())
}
