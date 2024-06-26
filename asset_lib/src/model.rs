use super::{Asset, AssetError, AssetFile, AssetType, Result};

use nalgebra_glm as glm;
use serde::{Deserialize, Serialize};

use std::hash::{Hash, Hasher};

const MODEL_VERSION: u32 = 1;

#[derive(Serialize, Deserialize, Default, Hash)]
pub struct Model {
  pub name: String,
  pub id: u128,
  pub meshes: Vec<Mesh>,

  #[serde(skip)]
  pub blob: Vec<u8>,
}

impl Model {
  pub fn new(name: &str, id: u128) -> Self {
    Self {
      name: name.to_owned(),
      id,
      ..Default::default()
    }
  }

  pub fn load_model(asset: AssetFile) -> Result<Self> {
    if asset.asset_type != AssetType::Model {
      return Err(AssetError::IncorrectType("Model", asset.asset_type.name()));
    }

    if asset.version < MODEL_VERSION {
      return Err(AssetError::OldVersion);
    }

    let mut model: Self = serde_json::from_str(&asset.json)?;
    model.blob = asset.blob;

    Ok(model)
  }

  pub fn add_mesh(&mut self, vertices: &[Vertex], indices: &[u32]) -> Result<()> {
    let vertex_count = vertices.len() as u32;
    let vertex_offset = self.blob.len() as u32;
    let mut vertex_data = bincode::serialize(&vertices)?;
    self.blob.extend_from_slice(&mut vertex_data[8..]);

    let index_count = indices.len() as u32;
    let index_offset = self.blob.len() as u32;
    let mut index_data = bincode::serialize(&indices)?;
    self.blob.extend_from_slice(&mut index_data[8..]);

    let mesh = Mesh {
      vertex_count,
      vertex_offset,
      index_count,
      index_offset,
    };
    self.meshes.push(mesh);
    Ok(())
  }
}

impl Asset for Model {
  fn convert_to_asset(self) -> Result<AssetFile> {
    let json = serde_json::to_string(&self)?;
    Ok(AssetFile {
      asset_type: AssetType::Model,
      version: MODEL_VERSION,
      json,
      blob: self.blob,
    })
  }
}

#[derive(Serialize, Deserialize, Hash)]
pub struct Mesh {
  pub vertex_count: u32,  // amount if vertices in the mesh
  pub vertex_offset: u32, // offset into the buffer where the vertices begin
  pub index_count: u32,   // amount of indices
  pub index_offset: u32,  // offset into the buffer where the indices begin
}

#[derive(Serialize, Clone, Copy, PartialEq)]
pub struct Vertex {
  pub position: glm::Vec3,
  pub normal: glm::Vec3,
  pub tangent: glm::Vec4,
}

/// Note: you should never use this type for any calcuations. This is just a shim for putting normal Vertex types into hashmaps.
#[derive(Clone, Copy, PartialEq)]
pub struct HashableVertex {
  pub vertex: Vertex,
}

impl Eq for HashableVertex {}

impl Hash for HashableVertex {
  fn hash<H: Hasher>(&self, state: &mut H) {
    let bytes = bincode::serialize(&self.vertex).unwrap();
    bytes.iter().for_each(|byte| state.write_u8(*byte));
  }
}

impl From<Vertex> for HashableVertex {
  fn from(value: Vertex) -> Self {
    Self { vertex: value }
  }
}
