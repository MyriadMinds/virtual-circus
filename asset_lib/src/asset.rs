use super::Result;

use serde::{Deserialize, Serialize};

use std::fs::File;

pub trait Asset {
  fn convert_to_asset(self) -> Result<AssetFile>;
}

#[derive(Serialize, Deserialize, Copy, Clone, PartialEq, Eq)]
pub enum AssetType {
  Model = 1,
  Scene = 2,
}

impl AssetType {
  pub fn name(&self) -> &'static str {
    match self {
      AssetType::Model => "Model",
      AssetType::Scene => "Scene",
    }
  }
}

#[derive(Serialize, Deserialize)]
pub struct AssetFile {
  pub(crate) asset_type: AssetType,
  pub(crate) version: u32,
  pub(crate) json: String,
  pub(crate) blob: Vec<u8>,
}

impl AssetFile {
  pub fn save_to_file(self, path: &str) -> Result<()> {
    let file = File::create(path)?;
    bincode::serialize_into(file, &self)?;

    Ok(())
  }

  pub fn load_from_file(path: &str) -> Result<Self> {
    let file = File::open(path)?;
    let asset: AssetFile = bincode::deserialize_from(file)?;
    Ok(asset)
  }
}
