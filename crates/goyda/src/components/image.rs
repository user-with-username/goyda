use crate::components::{Asset, Component};

pub struct Image {
    pub asset: Asset,
}

impl Image {
    pub fn new(asset: impl Into<Asset>) -> Component {
        Component::Image(Self {
            asset: asset.into(),
        })
    }
}
