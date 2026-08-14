use anyhow::Result;
use std::path::{Path, PathBuf};

use crate::targets::{BuildContext, Platform, PlatformTarget};
use crate::term;

mod build;
mod env;
mod layout;
mod serve;
mod wasm_bindgen;

pub struct WebTarget;

impl PlatformTarget for WebTarget {
    fn id(&self) -> Platform {
        Platform::Web
    }

    fn supported_triples(&self) -> &'static [&'static str] {
        &["wasm32-unknown-unknown"]
    }

    fn build(&self, ctx: &BuildContext) -> Result<PathBuf> {
        term::header("UI", &[&ctx.target_triple]);
        build::build_web_app(ctx)
    }

    fn run(&self, _ctx: &BuildContext, artifact_path: &Path) -> Result<()> {
        serve::serve_and_open(artifact_path)
    }
}
