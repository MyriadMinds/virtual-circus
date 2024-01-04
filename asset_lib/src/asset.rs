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

  fn save_to_writer<W: std::io::Write>(self, writer: &mut W) -> Result<()> {
    bincode::serialize_into(writer, &self)?;
    Ok(())
  }

  fn read_from_reader<R: std::io::Read>(reader: R) -> Result<Self> {
    let asset: AssetFile = bincode::deserialize_from(reader)?;
    Ok(asset)
  }
}

pub struct AssetArchiveWriter {
  zip_writer: zip::ZipWriter<File>,
}

impl AssetArchiveWriter {
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
}

pub struct AssetArchiveReader {
  zip_reader: zip::ZipArchive<File>,
}

impl AssetArchiveReader {
  pub fn open(path: &str) -> Result<Self> {
    let file = File::open(path)?;
    let zip_reader = zip::ZipArchive::new(file)?;

    Ok(Self { zip_reader })
  }

  pub fn file_names(&self) -> impl Iterator<Item = &str> {
    self.zip_reader.file_names()
  }

  pub fn get_asset(&mut self, name: &str) -> Result<AssetFile> {
    let asset = self.zip_reader.by_name(name)?;
    let asset = AssetFile::read_from_reader(asset)?;
    Ok(asset)
  }
}
