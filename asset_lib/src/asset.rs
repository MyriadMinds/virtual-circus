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
  Pipeline = 3,
}

impl AssetType {
  pub fn name(&self) -> &'static str {
    match self {
      AssetType::Model => "Model",
      AssetType::Scene => "Scene",
      AssetType::Pipeline => "Pipeline",
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

  fn save_to_writer<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
    let writer = std::io::BufWriter::new(writer);
    bincode::serialize_into(writer, &self)?;
    Ok(())
  }

  fn read_from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
    let reader = std::io::BufReader::new(reader);
    let asset: AssetFile = bincode::deserialize_from(reader)?;
    Ok(asset)
  }

  pub fn asset_type(&self) -> AssetType {
    self.asset_type
  }
}

pub struct AssetArchive {
  zip_writer: zip::ZipWriter<File>,
}

impl AssetArchive {
  pub fn new(path: &str) -> Result<Self> {
    let file = File::create(path)?;
    let zip_writer = zip::ZipWriter::new(file);

    Ok(Self { zip_writer })
  }

  pub fn add_asset_file(&mut self, asset_file: AssetFile, filename: &str) -> Result<()> {
    let options = zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    self.zip_writer.start_file(filename, options)?;
    asset_file.save_to_writer(&mut self.zip_writer)?;
    Ok(())
  }

  pub fn finish(&mut self) -> Result<()> {
    self.zip_writer.finish()?;
    Ok(())
  }

  pub fn get_assets(path: &str) -> Result<Vec<AssetFile>> {
    let file = File::open(path)?;
    let mut zip_reader = zip::ZipArchive::new(file)?;
    let names = zip_reader.file_names().map(|name| name.to_owned()).collect::<Vec<String>>();
    let mut assets = Vec::new();

    for name in names {
      let asset = zip_reader.by_name(&name)?;
      let asset = AssetFile::read_from_reader(asset)?;
      assets.push(asset);
    }

    Ok(assets)
  }
}
