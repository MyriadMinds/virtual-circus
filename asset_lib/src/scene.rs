use super::{Asset, AssetError, AssetFile, AssetType, Result};

use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

const SCENE_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default)]
pub struct Scene {
  pub name: String,
  models: Vec<String>,
  nodes: Vec<Node>,
  parent_nodes: Vec<usize>,
}

#[derive(Serialize, Deserialize, Default)]
pub struct Node {
  pub transform: glm::Mat4,
  pub childrem: Vec<usize>,
  pub models: Vec<usize>,
}

impl Scene {
  pub fn load_scene(asset: AssetFile) -> Result<Self> {
    if asset.asset_type != AssetType::Scene {
      return Err(AssetError::IncorrectType("Scene", asset.asset_type.name()));
    }

    if asset.version < SCENE_VERSION {
      return Err(AssetError::OldVersion);
    }

    let scene: Self = serde_json::from_str(&asset.json)?;
    Ok(scene)
  }

  pub fn insert_model(&mut self, model_name: &str) -> usize {
    self.models.push(model_name.to_owned());
    self.models.len() - 1
  }

  pub fn insert_node(&mut self, node: Node) -> usize {
    self.nodes.push(node);
    self.nodes.len() - 1
  }

  pub fn insert_parent_node(&mut self, node: usize) {
    self.parent_nodes.push(node);
  }

  pub fn models(&self) -> &[String] {
    self.models.as_ref()
  }

  pub fn nodes(&self) -> &[Node] {
    self.nodes.as_ref()
  }

  pub fn parent_nodes(&self) -> &[usize] {
    self.parent_nodes.as_ref()
  }
}

impl Asset for Scene {
  fn convert_to_asset(self) -> Result<AssetFile> {
    let json = serde_json::to_string(&self)?;
    Ok(AssetFile {
      asset_type: AssetType::Scene,
      version: SCENE_VERSION,
      json,
      blob: Vec::new(),
    })
  }
}
