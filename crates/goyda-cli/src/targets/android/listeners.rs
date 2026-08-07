use anyhow::Result;
use std::fs;
use std::path::Path;

use goyda::android::classgen::{self, ListenerSpec};

use super::AndroidAppLayout;

pub fn write_runtime_listener_classes(layout: &AndroidAppLayout) -> Result<()> {
    for spec in inventory::iter::<ListenerSpec> {
        let class_relative_path = Path::new(spec.class_name).with_extension("class");
        let path = layout.classes_dir().join(class_relative_path);

        if let Some(p) = path.parent() {
            fs::create_dir_all(p)?;
        }

        fs::write(&path, classgen::build_listener_class(spec))?;
    }
    Ok(())
}
