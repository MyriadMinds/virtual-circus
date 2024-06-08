use super::{Asset, AssetFile, AssetType, Result};

use serde::{Deserialize, Serialize};

const PIPELINE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Debug)]
pub struct Blending {
  pub test: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Pipeline {
  pub name: String,
  pub vertex_shader: Vec<u8>,
  pub fragment_shader: Vec<u8>,
  pub blending: Blending,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PipelineManifest {
  pub name: String,
  pub vertex_shader: String,
  pub fragment_shader: String,
  pub blending: Blending,
}

impl Asset for Pipeline {
  fn convert_to_asset(self) -> Result<AssetFile> {
    let json = serde_json::to_string(&self)?;
    Ok(AssetFile {
      asset_type: AssetType::Pipeline,
      version: PIPELINE_VERSION,
      json,
      blob: Vec::new(),
    })
  }
}
