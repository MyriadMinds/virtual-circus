use super::{Converter, ConverterError, Result};

use asset_lib as ast;
use log::{error, info};
use nalgebra_glm as glm;
use num_traits::{AsPrimitive, FromPrimitive};

use std::any::Any;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

enum DataType {
  I8,
  U8,
  I16,
  U16,
  U32,
  F32,
}

pub struct GLTFConverter {
  document: gltf::Document,
  buffers: Vec<gltf::buffer::Data>,
  _images: Vec<gltf::image::Data>,
  file_name: String,
  output_dir: String,
  models: Vec<ast::Model>,
  scenes: Vec<ast::Scene>,
}

impl Converter for GLTFConverter {
  fn parse_file(src_file: &str, output_dir: &str) {
    let (document, buffers, images) = match gltf::import(src_file) {
      Ok(contents) => contents,
      Err(e) => {
        error!("Failed to open GLTF file: {}", e);
        return;
      }
    };

    let mut file = PathBuf::new();
    file.push(src_file);
    let file_name = file.file_stem().unwrap().to_str().unwrap().to_owned();

    let mut converter = Self {
      document,
      buffers,
      _images: images,
      file_name,
      output_dir: output_dir.to_owned(),
      models: Vec::new(),
      scenes: Vec::new(),
    };

    converter.parse_models();
    converter.parse_scenes();
    converter.write_files();
  }
}

impl GLTFConverter {
  fn parse_models(&mut self) {
    let meshes = self.document.meshes();

    for mesh in meshes {
      let model = match self.parse_model(&mesh) {
        Ok(model) => model,
        Err(e) => {
          error!("Failed to convert gltf mesh: {}", e);
          continue;
        }
      };

      self.models.push(model);
    }
  }

  fn parse_model(&self, mesh: &gltf::Mesh) -> Result<ast::Model> {
    let mut model = ast::Model::default();

    let index = mesh.index();
    model.name = mesh.name().map(|name| name.to_owned()).unwrap_or(format!("Model_{index}"));

    for primitive in mesh.primitives() {
      let (vertices, indices) = self.parse_primitive(&primitive)?;
      model.add_mesh(&vertices, &indices)?;
    }

    model.id = hash_model(&model);

    Ok(model)
  }

  fn parse_primitive(&self, primitive: &gltf::Primitive) -> Result<(Vec<ast::Vertex>, Vec<u32>)> {
    let accessors = primitive.attributes();

    let mut attributes = Attributes::default();
    for accessor in accessors {
      match accessor.0 {
        gltf::Semantic::Positions => attributes.position = self.parse_accessor(&accessor.1, glm::Vec3::from([0.0, 0.0, 0.0]))?,
        gltf::Semantic::Normals => attributes.normals = self.parse_accessor(&accessor.1, glm::Vec3::from([0.0, 0.0, 0.0]))?,
        gltf::Semantic::Tangents => attributes.tangents = self.parse_accessor(&accessor.1, glm::Vec4::from([0.0, 0.0, 0.0, 0.0]))?,
        gltf::Semantic::Colors(_) => (),
        gltf::Semantic::TexCoords(_) => (),
        gltf::Semantic::Joints(_) => (),
        gltf::Semantic::Weights(_) => (),
      }
    }

    if attributes.position.len() == 0 {
      return Err(ConverterError::ParsingError("primitive has no position data!"));
    }

    attributes.fill_missing();

    if !attributes.attributes_are_equal() {
      return Err(ConverterError::ParsingError("primitive attributes do not have equal length!"));
    };

    let mut vertices = Vec::with_capacity(attributes.position.len());
    for (i, position) in attributes.position.into_iter().enumerate() {
      let normal = attributes.normals[i];
      let tangent = attributes.tangents[i];

      let vertex = ast::Vertex { position, normal, tangent };

      vertices.push(vertex);
    }

    let mut indices = if let Some(indices) = primitive.indices() {
      self.parse_accessor(&indices, glm::UVec1::from([0]))?.iter().map(|index| index.x).collect()
    } else {
      convert_to_indices(&mut vertices)
    };

    match primitive.mode() {
      gltf::mesh::Mode::Points => todo!(),
      gltf::mesh::Mode::Lines => todo!(),
      gltf::mesh::Mode::LineLoop => todo!(),
      gltf::mesh::Mode::LineStrip => todo!(),
      gltf::mesh::Mode::Triangles => (),
      gltf::mesh::Mode::TriangleStrip => indices = convert_indices_from_strip(indices),
      gltf::mesh::Mode::TriangleFan => indices = convert_indices_from_fan(indices),
    }

    Ok((vertices, indices))
  }

  fn parse_accessor<const C: usize, T>(&self, accessor: &gltf::Accessor, default: glm::TVec<T, C>) -> Result<Vec<glm::TVec<T, C>>>
  where
    T: 'static + Default + Clone + Copy + FromPrimitive + Any,
    i8: AsPrimitive<T>,
    u8: AsPrimitive<T>,
    i16: AsPrimitive<T>,
    u16: AsPrimitive<T>,
    u32: AsPrimitive<T>,
    f32: AsPrimitive<T>,
  {
    let count = accessor.count();
    let data_type = convert_accessor_data_type(&accessor.data_type());
    let component_width = get_component_width(&accessor.dimensions());
    let element_size = get_data_type_size(&data_type);
    let component_size = component_width * element_size;

    // Getting base data of the accessor
    let mut base_components = match accessor.view() {
      Some(buffer_view) => {
        let stride = buffer_view.stride().unwrap_or(component_size);
        let buffer_offset = accessor.offset() + buffer_view.offset();
        let buffer = self.buffers.get(buffer_view.buffer().index()).ok_or(ConverterError::MissingResource)?;
        let buffer = &buffer[buffer_offset..buffer_offset + stride * count];

        parse_buffer_view(buffer, &data_type, element_size, stride, default)?
      }
      None => vec![default.clone(); count],
    };

    // checking if there's extra sparse information and applying it to base data
    if let Some(sparse) = accessor.sparse() {
      let count = sparse.count() as usize;

      // values
      let values = sparse.values();
      let buffer_view = values.view();

      let stride = component_size;
      let buffer_offset = values.offset() as usize + buffer_view.offset();
      let buffer = self.buffers.get(buffer_view.buffer().index()).ok_or(ConverterError::MissingResource)?;
      let buffer = &buffer[buffer_offset..buffer_offset + stride * count];

      let values = parse_buffer_view(&buffer, &data_type, element_size, stride, default)?;

      // indices
      let indices = sparse.indices();
      let data_type = convert_index_data_type(&indices.index_type());
      let buffer_view = indices.view();

      let stride = get_data_type_size(&data_type);
      let buffer_offset = indices.offset() as usize + buffer_view.offset();
      let buffer = self.buffers.get(buffer_view.buffer().index()).ok_or(ConverterError::MissingResource)?;
      let buffer = &buffer[buffer_offset..buffer_offset + stride * count];

      let indices = parse_buffer_view::<1, u32>(buffer, &data_type, stride, stride, glm::UVec1::from([0]))?;

      for (value_index, base_data_index) in indices.iter().enumerate() {
        base_components[base_data_index.x as usize] = values[value_index];
      }
    }

    if accessor.normalized() {
      todo!();
      // for mut element in base_components.iter_mut() {
      //   renormalize(&mut element, &data_type);
      // }
    }

    Ok(base_components)
  }

  fn parse_scenes(&mut self) {
    let scenes = self.document.scenes();

    for scene in scenes {
      let scene = match self.parse_scene(&scene) {
        Ok(scene) => scene,
        Err(e) => {
          error!("Failed to convert a gltf scene: {}", e);
          continue;
        }
      };

      self.scenes.push(scene);
    }
  }

  fn parse_scene(&self, scene: &gltf::Scene) -> Result<ast::Scene> {
    let mut parsed_scene = ast::Scene::default();

    let index = scene.index();
    parsed_scene.name = scene.name().map(|name| name.to_string()).unwrap_or(format!("Scene_{index}"));

    let nodes = scene.nodes();
    for node in nodes {
      let node = self.parse_node(&mut parsed_scene, &node)?;
      let index = parsed_scene.insert_node(node);
      parsed_scene.insert_parent_node(index);
    }

    Ok(parsed_scene)
  }

  fn parse_node(&self, scene: &mut ast::Scene, node: &gltf::Node) -> Result<ast::Node> {
    let children = node.children();
    let mut parsed_node = ast::Node::default();

    parsed_node.transform = glm::Mat4::from(node.transform().matrix());
    parsed_node.name = "Node".to_owned();

    if let Some(mesh) = node.mesh() {
      let model_name = self.models.get(mesh.index()).ok_or(ConverterError::MissingResource)?.name.clone();
      let model_id = self.models.get(mesh.index()).ok_or(ConverterError::MissingResource)?.id.clone();
      let index = scene.insert_model(model_id);
      parsed_node.model = Some(index);
      parsed_node.name = model_name;
    };

    for node in children {
      let node = self.parse_node(scene, &node)?;
      let index = scene.insert_node(node);
      parsed_node.children.push(index);
    }

    Ok(parsed_node)
  }

  fn write_files(mut self) {
    let output_dir = self.output_dir;
    let file_name = self.file_name;
    let archive_name = format!("{output_dir}/{file_name}.ast");
    let mut archive = match ast::AssetArchive::new(&archive_name) {
      Ok(archive) => {
        info!("Created asset archive: {}", archive_name);
        archive
      }
      Err(e) => {
        error!("Failed to create asset archive for gltf file: {}", e);
        return;
      }
    };

    for model in self.models.drain(..) {
      let model_name = model.name.to_owned();
      let model_name = format!("{model_name}.mesh");
      info!("Adding gltf model to archive: {}", model_name);
      save_asset(model, &model_name, &mut archive);
    }

    for scene in self.scenes.drain(..) {
      let scene_name = scene.name.to_owned();
      let scene_name = format!("{scene_name}.scn");
      info!("Adding gltf scene to archive: {}", scene_name);
      save_asset(scene, &scene_name, &mut archive);
    }

    archive.finish().unwrap();
  }
}

//----------------------------Helpers--------------------------------------

fn save_asset(asset: impl ast::Asset, asset_name: &str, archive: &mut ast::AssetArchive) {
  let asset = match asset.convert_to_asset() {
    Ok(asset) => asset,
    Err(e) => {
      error!("Failed to convert to asset file: {}", e);
      return;
    }
  };

  match archive.add_asset_file(asset, asset_name) {
    Ok(_) => (),
    Err(e) => error!("Failed to save asset to archive: {}", e),
  }
}

#[derive(Default)]
struct Attributes {
  position: Vec<glm::Vec3>,
  normals: Vec<glm::Vec3>,
  tangents: Vec<glm::Vec4>,
}

impl Attributes {
  fn attributes_are_equal(&self) -> bool {
    self.position.len() == self.normals.len() && self.position.len() == self.tangents.len()
  }

  fn fill_missing(&mut self) {
    let count = self.position.len();
    if self.normals.len() == 0 {
      self.normals = vec![glm::Vec3::from([0.0, 0.0, 0.0]); count]
    };
    if self.tangents.len() == 0 {
      self.tangents = vec![glm::Vec4::from([0.0, 0.0, 0.0, 0.0]); count]
    }
  }
}

fn get_component_width(dimension: &gltf::accessor::Dimensions) -> usize {
  match dimension {
    gltf::accessor::Dimensions::Scalar => 1,
    gltf::accessor::Dimensions::Vec2 => 2,
    gltf::accessor::Dimensions::Vec3 => 3,
    gltf::accessor::Dimensions::Vec4 => 4,
    gltf::accessor::Dimensions::Mat2 => 2,
    gltf::accessor::Dimensions::Mat3 => 3,
    gltf::accessor::Dimensions::Mat4 => 4,
  }
}

fn get_data_type_size(data_type: &DataType) -> usize {
  match data_type {
    DataType::I8 => 1,
    DataType::U8 => 1,
    DataType::I16 => 2,
    DataType::U16 => 2,
    DataType::U32 => 4,
    DataType::F32 => 4,
  }
}

fn convert_accessor_data_type(data_type: &gltf::accessor::DataType) -> DataType {
  match data_type {
    gltf::accessor::DataType::I8 => DataType::I8,
    gltf::accessor::DataType::U8 => DataType::U8,
    gltf::accessor::DataType::I16 => DataType::I16,
    gltf::accessor::DataType::U16 => DataType::U16,
    gltf::accessor::DataType::U32 => DataType::U32,
    gltf::accessor::DataType::F32 => DataType::F32,
  }
}

fn convert_index_data_type(data_type: &gltf::accessor::sparse::IndexType) -> DataType {
  match data_type {
    gltf::accessor::sparse::IndexType::U8 => DataType::U8,
    gltf::accessor::sparse::IndexType::U16 => DataType::U16,
    gltf::accessor::sparse::IndexType::U32 => DataType::U32,
  }
}

fn parse_buffer_view<const C: usize, T>(data: &[u8], data_type: &DataType, element_size: usize, stride: usize, default: glm::TVec<T, C>) -> Result<Vec<glm::TVec<T, C>>>
where
  T: 'static + Default + Clone + Copy + FromPrimitive,
  i8: AsPrimitive<T>,
  u8: AsPrimitive<T>,
  i16: AsPrimitive<T>,
  u16: AsPrimitive<T>,
  u32: AsPrimitive<T>,
  f32: AsPrimitive<T>,
{
  let mut components = Vec::with_capacity(data.len() / stride);

  for component in data.chunks_exact(stride) {
    let elements_data = component.chunks_exact(element_size);

    let mut component = default.clone();
    for (i, element_bytes) in elements_data.enumerate() {
      if i > C {
        break;
      };

      // Search here for data conversion errors
      let failure = ConverterError::ParsingError("failed to parse vertex attribute bytes!");
      let element = match data_type {
        DataType::I8 => i8::from_le_bytes(element_bytes.try_into().or(Err(failure))?).as_(),
        DataType::U8 => u8::from_le_bytes(element_bytes.try_into().or(Err(failure))?).as_(),
        DataType::I16 => i16::from_le_bytes(element_bytes.try_into().or(Err(failure))?).as_(),
        DataType::U16 => u16::from_le_bytes(element_bytes.try_into().or(Err(failure))?).as_(),
        DataType::U32 => u32::from_le_bytes(element_bytes.try_into().or(Err(failure))?).as_(),
        DataType::F32 => f32::from_le_bytes(element_bytes.try_into().or(Err(failure))?).as_(),
      };

      component[i] = element;
    }
    components.push(component);
  }

  Ok(components)
}

fn convert_to_indices(vertices: &mut Vec<ast::Vertex>) -> Vec<u32> {
  let mut indices = Vec::new();
  let mut new_vertices: Vec<ast::Vertex> = Vec::new();
  let mut hash_map: HashMap<ast::HashableVertex, u32> = HashMap::new();

  for vertex in vertices.iter() {
    match hash_map.get_key_value(&(ast::HashableVertex::from(*vertex))) {
      Some(index) => indices.push(*index.1),
      None => {
        let index = new_vertices.len() as u32;
        new_vertices.push(*vertex);
        hash_map.insert(ast::HashableVertex::from(*vertex), index);
      }
    }
  }

  vertices.truncate(new_vertices.len());
  vertices.swap_with_slice(&mut new_vertices);

  indices
}

fn convert_indices_from_strip(indices: Vec<u32>) -> Vec<u32> {
  if indices.len() < 3 {
    return indices;
  }

  let mut new_indices = Vec::with_capacity(indices.len() * 3);
  let mut indices = indices.iter().enumerate();

  let (_, mut first_index) = indices.next().unwrap();
  let (_, mut second_index) = indices.next().unwrap();

  for (even, index) in indices {
    if even % 2 == 0 {
      new_indices.push(*first_index);
      new_indices.push(*second_index);
      new_indices.push(*index);
    } else {
      new_indices.push(*second_index);
      new_indices.push(*first_index);
      new_indices.push(*index);
    }

    first_index = second_index;
    second_index = index;
  }

  new_indices
}

fn convert_indices_from_fan(indices: Vec<u32>) -> Vec<u32> {
  if indices.len() < 3 {
    return indices;
  }

  let mut new_indices = Vec::with_capacity(indices.len() * 3);
  let mut indices = indices.iter();

  let first_index = indices.next().unwrap();
  let mut second_index = indices.next().unwrap();

  for index in indices {
    new_indices.push(*first_index);
    new_indices.push(*second_index);
    new_indices.push(*index);

    second_index = index;
  }

  new_indices
}

// fn renormalize<T>(value: &mut T, data_type: &DataType) {
//   match data_type {
//     DataType::I8 => {
//       if TypeId::of::<T>() == TypeId::of::<i8>() {
//         return;
//       }
//     }

//     DataType::U8 => todo!(),
//     DataType::I16 => todo!(),
//     DataType::U16 => todo!(),
//     DataType::U32 => todo!(),
//     DataType::F32 => todo!(),
//   }
// }

fn hash_model(model: &ast::Model) -> u128 {
  let mut hasher = DefaultHasher::new();
  model.hash(&mut hasher);
  hasher.finish() as u128
}
