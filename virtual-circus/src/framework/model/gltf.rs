use super::Model;
use crate::utils::tools::{ModelError, Result};
use crate::vulkan::allocator::{Buffer, BufferType, Image, ImagePurpose};
use crate::vulkan::descriptors::{MaterialDescriptorSetInfo, MaterialDescriptorSetLayout, MaterialDescriptorSets, MaterialFlags, MaterialInfo, TextureInfo};
use crate::vulkan::elements::{ImageView, Sampler};
use crate::vulkan::rendering_context::{Attribute, AttributeType, IndexInfo, MeshContext, RenderingContext, VertexInfo};
use crate::vulkan::{Allocator, Device};

use ash::vk;
use glam::*;
use gltf::Document;
use log::{error, warn};

use std::sync::Arc;

pub(crate) struct GltfModel {
  default_scene: Option<usize>,
  scenes: Vec<Scene>,
  nodes: Vec<Node>,
  meshes: Vec<Mesh>,
  materials: Vec<MaterialDetails>,
  material_descriptors: MaterialDescriptorSets,
  textures: Vec<Texture>,
  accessors: Vec<Accessor>,
  buffer_views: Vec<BufferView>,
  buffers: Vec<Buffer>,
  default_color_buffer: Buffer,
  images: Vec<Image>,
}

impl GltfModel {
  pub(crate) fn new(path: &str, allocator: &mut Allocator, descriptor_set_layout: &MaterialDescriptorSetLayout) -> Result<Self> {
    let (gltf_document, buffers, images) = gltf::import(path).map_err(ModelError::GltfError)?;

    let default_scene = gltf_document.default_scene().map(|scene| scene.index());
    let scenes = parse_scenes(&gltf_document);
    let nodes = parse_nodes(&gltf_document);
    let meshes = parse_meshes(&gltf_document);
    let accessors = parse_accessors(&gltf_document);
    let buffer_views = parse_buffer_views(&gltf_document);
    let mut buffers = parse_buffers(allocator, buffers)?;
    let default_color_buffer = buffers.pop().unwrap();
    let images = parse_images(allocator, images)?;
    let textures = parse_textures(&gltf_document, &images)?;

    let (material_infos, materials) = parse_materials(&gltf_document, &textures);
    let material_descriptors = descriptor_set_layout.create_descriptor_sets(allocator, &material_infos)?;

    Ok(Self {
      default_scene,
      scenes,
      nodes,
      meshes,
      materials,
      material_descriptors,
      textures,
      accessors,
      buffer_views,
      buffers,
      default_color_buffer,
      images,
    })
  }
}

impl Model for GltfModel {
  unsafe fn draw(&self, rendering_context: &mut RenderingContext) {
    let Some(scene) = self.default_scene else {
      warn!("tried to render a gltf model with no selected scene");
      return;
    };

    let Some(scene) = self.scenes.get(scene) else {
      error!("selected scene does not exist in this gltf model");
      return;
    };

    rendering_context.bind_descriptor_buffer(&self.material_descriptors);
    self.draw_scene(scene, rendering_context);
  }
}

//------------------------------------------structs----------------------------------------------------------

struct Scene {
  nodes: Vec<usize>,
  name: String,
}

struct Node {
  camera: Option<usize>,
  children: Vec<usize>,
  skin: Option<usize>,
  mesh: Option<usize>,
  matrix: Mat4,
  translation: Vec3,
  rotation: Quat,
  scale: Vec3,
  weights: Option<Vec<f32>>,
  name: String,
}

struct Mesh {
  primitives: Vec<Primitive>,
  weights: Option<Vec<f32>>,
  name: String,
}

struct Materials {
  materials: MaterialDescriptorSets,
  material_details: Vec<MaterialDetails>,
}

struct MaterialDetails {
  color_texcoord: Option<usize>,
  metallic_roughness_texcoord: Option<usize>,
  normals_texcoord: Option<usize>,
  occlusion_texcoord: Option<usize>,
  emissive_texcoord: Option<usize>,
}

struct Texture {
  image_view: ImageView,
  sampler: Sampler,
}

struct Primitive {
  attributes: Attributes,
  indices: Option<usize>,
  material: Option<usize>,
  mode: ash::vk::PrimitiveTopology,
  targets: Vec<Attributes>,
}

#[derive(Default)]
struct Attributes {
  position: usize,
  normal: Option<usize>,
  tangent: Option<usize>,
  texcoords: Vec<usize>,
  colors: Vec<usize>,
  joints: Vec<usize>,
  weights: Vec<usize>,
}

struct Accessor {
  buffer_view: usize,
  byte_offset: usize,
  component_type: gltf::accessor::DataType,
  normalized: bool,
  count: usize,
  data_type: gltf::accessor::Dimensions,
  max: Option<Vec<f64>>,
  min: Option<Vec<f64>>,
}

struct BufferView {
  buffer: usize,
  byte_offset: usize,
  byte_length: usize,
  byte_stride: Option<usize>,
  target: Option<gltf::buffer::Target>,
  name: String,
}

//------------------------------------------Model loading----------------------------------------------------

fn parse_scenes(gltf: &Document) -> Vec<Scene> {
  let mut scenes = Vec::new();

  for scene in gltf.scenes() {
    let nodes = scene.nodes().map(|node| node.index()).collect();

    scenes.push(Scene {
      nodes,
      name: scene.name().unwrap_or("Scene").to_owned(),
    });
  }

  scenes
}

fn parse_nodes(gltf: &Document) -> Vec<Node> {
  let mut nodes = Vec::new();

  for node in gltf.nodes() {
    let camera = node.camera().map(|camera| camera.index());
    let children = node.children().map(|node| node.index()).collect();
    let skin = node.skin().map(|skin| skin.index());
    let mesh = node.mesh().map(|mesh| mesh.index());
    let transform = node.transform();
    let matrix = transform.clone().matrix();
    let (translation, rotation, scale) = transform.decomposed();
    let weights = node.weights().map(|weights| weights.to_owned());
    let name = node.name().unwrap_or("Node").to_owned();

    nodes.push(Node {
      camera,
      children,
      skin,
      mesh,
      matrix: Mat4::from_cols_array_2d(&matrix),
      translation: Vec3::from_array(translation),
      rotation: Quat::from_array(rotation),
      scale: Vec3::from_array(scale),
      weights,
      name,
    });
  }

  nodes
}

fn parse_meshes(gltf: &Document) -> Vec<Mesh> {
  let mut meshes = Vec::new();

  for mesh in gltf.meshes() {
    let primitives = parse_primitives(&mesh);
    let weights = mesh.weights().map(|weights| weights.to_owned());
    let name = mesh.name().unwrap_or("Node").to_owned();

    meshes.push(Mesh { primitives, weights, name });
  }

  meshes
}

fn parse_primitives(mesh: &gltf::Mesh) -> Vec<Primitive> {
  let mut primitives = Vec::new();

  for primitive in mesh.primitives() {
    let mut attributes = Attributes::default();
    for attribute in primitive.attributes() {
      match attribute.0 {
        gltf::Semantic::Positions => attributes.position = attribute.1.index(),
        gltf::Semantic::Normals => attributes.normal = Some(attribute.1.index()),
        gltf::Semantic::Tangents => attributes.tangent = Some(attribute.1.index()),
        gltf::Semantic::Colors(_) => attributes.colors.push(attribute.1.index()),
        gltf::Semantic::TexCoords(_) => attributes.texcoords.push(attribute.1.index()),
        gltf::Semantic::Joints(_) => attributes.joints.push(attribute.1.index()),
        gltf::Semantic::Weights(_) => attributes.weights.push(attribute.1.index()),
      }
    }

    let indices = primitive.indices().map(|accessor| accessor.index());
    let material = primitive.material().index();

    let mut targets = Vec::new();
    for target in primitive.morph_targets() {
      let target = Attributes {
        position: target.positions().unwrap().index(),
        normal: target.normals().map(|attribute| attribute.index()),
        tangent: target.tangents().map(|attribute| attribute.index()),
        texcoords: Vec::new(),
        colors: Vec::new(),
        joints: Vec::new(),
        weights: Vec::new(),
      };

      targets.push(target);
    }

    use ash::vk::PrimitiveTopology;
    let mode = match primitive.mode() {
      gltf::mesh::Mode::Points => PrimitiveTopology::POINT_LIST,
      gltf::mesh::Mode::Lines => PrimitiveTopology::LINE_LIST,
      gltf::mesh::Mode::LineLoop => PrimitiveTopology::LINE_LIST,
      gltf::mesh::Mode::LineStrip => PrimitiveTopology::LINE_STRIP,
      gltf::mesh::Mode::Triangles => PrimitiveTopology::TRIANGLE_LIST,
      gltf::mesh::Mode::TriangleStrip => PrimitiveTopology::TRIANGLE_STRIP,
      gltf::mesh::Mode::TriangleFan => PrimitiveTopology::TRIANGLE_FAN,
    };

    primitives.push(Primitive {
      attributes,
      indices,
      material,
      mode,
      targets,
    });
  }

  primitives
}

fn parse_materials<'a>(gltf: &Document, textures: &'a [Texture]) -> (Vec<MaterialDescriptorSetInfo<'a>>, Vec<MaterialDetails>) {
  let materials = gltf.materials();
  let mut material_details = Vec::with_capacity(materials.len());
  let mut material_descriptor_set_infos = Vec::with_capacity(materials.len());

  for material in materials {
    let descriptor_set_info = parse_material_descriptor_set_info(&material, textures);
    let details = parse_material_details(&material);

    material_descriptor_set_infos.push(descriptor_set_info);
    material_details.push(details);
  }

  (material_descriptor_set_infos, material_details)
}

fn parse_material_details(material: &gltf::Material) -> MaterialDetails {
  MaterialDetails {
    color_texcoord: material.pbr_metallic_roughness().base_color_texture().map(|texture| texture.tex_coord() as usize),
    metallic_roughness_texcoord: material.pbr_metallic_roughness().metallic_roughness_texture().map(|texture| texture.tex_coord() as usize),
    normals_texcoord: material.normal_texture().map(|texture| texture.tex_coord() as usize),
    occlusion_texcoord: material.occlusion_texture().map(|texture| texture.tex_coord() as usize),
    emissive_texcoord: material.emissive_texture().map(|texture| texture.tex_coord() as usize),
  }
}

fn parse_material_descriptor_set_info<'a>(material: &gltf::Material, textures: &'a [Texture]) -> MaterialDescriptorSetInfo<'a> {
  let material_info = parse_material_info(material);

  let texture = match material.pbr_metallic_roughness().base_color_texture() {
    Some(texture) => convert_texture(&textures[texture.texture().index()]),
    None => convert_texture(textures.last().unwrap()),
  };

  let metallic_roughness_texture = match material.pbr_metallic_roughness().metallic_roughness_texture() {
    Some(texture) => convert_texture(&textures[texture.texture().index()]),
    None => convert_texture(textures.last().unwrap()),
  };

  let normal_texture = match material.normal_texture() {
    Some(texture) => convert_texture(&textures[texture.texture().index()]),
    None => convert_texture(textures.last().unwrap()),
  };

  let occlusion_texture = match material.occlusion_texture() {
    Some(texture) => convert_texture(&textures[texture.texture().index()]),
    None => convert_texture(textures.last().unwrap()),
  };

  let emissive_texture = match material.emissive_texture() {
    Some(texture) => convert_texture(&textures[texture.texture().index()]),
    None => convert_texture(textures.last().unwrap()),
  };

  MaterialDescriptorSetInfo {
    material_info,
    texture,
    metallic_roughness_texture,
    normal_texture,
    occlusion_texture,
    emissive_texture,
  }
}

fn parse_material_info(material: &gltf::Material) -> MaterialInfo {
  let pbr = material.pbr_metallic_roughness();
  let base_color_factor = Vec4::from(pbr.base_color_factor());
  let metallic_roughness_factor = Vec2::from([pbr.metallic_factor(), pbr.roughness_factor()]);
  let normals_scale_factor = material.normal_texture().map(|texture| texture.scale()).unwrap_or(0.0);
  let occlusion_strength_factor = material.occlusion_texture().map(|texture| texture.strength()).unwrap_or(0.0);
  let emissive_factor = Vec3A::from(material.emissive_factor());
  let alpha_cutoff = material.alpha_cutoff().unwrap_or(0.5);

  let mut material_flags = MaterialFlags::none();
  match material.alpha_mode() {
    gltf::material::AlphaMode::Opaque => material_flags |= MaterialFlags::AlphaModeOpaque,
    gltf::material::AlphaMode::Mask => material_flags |= MaterialFlags::AlphaModeMask,
    gltf::material::AlphaMode::Blend => material_flags |= MaterialFlags::AlphaModeBlend,
  }
  if material.double_sided() {
    material_flags |= MaterialFlags::DoubleSided
  };
  if pbr.metallic_roughness_texture().is_some() {
    material_flags |= MaterialFlags::HasMetallicRougnessTexture
  };
  if material.normal_texture().is_some() {
    material_flags |= MaterialFlags::HasNormalTexture
  };
  if material.occlusion_texture().is_some() {
    material_flags |= MaterialFlags::HasOcclusionTexture
  };
  if material.emissive_texture().is_some() {
    material_flags |= MaterialFlags::HasEmmisiveTexture
  };

  MaterialInfo {
    base_color_factor,
    metallic_roughness_factor,
    normals_scale_factor,
    occlusion_strength_factor,
    emissive_factor,
    alpha_cutoff,
    material_flags,
  }
}

fn convert_texture(texture: &Texture) -> TextureInfo {
  TextureInfo {
    image_view: &texture.image_view,
    sampler: &texture.sampler,
  }
}

fn parse_textures(gltf: &gltf::Document, images: &[Image]) -> Result<Vec<Texture>> {
  let textures = gltf.textures();
  let mut finished_textures = Vec::with_capacity(textures.len());

  for texture in textures {
    let image_view = images[texture.source().index()].make_image_view()?;
    let device = image_view.get_device();
    let sampler = parse_sampler(&texture.sampler(), &device)?;

    finished_textures.push(Texture { image_view, sampler });
  }

  let default_image_view = images.last().unwrap().make_image_view()?;
  let device = default_image_view.get_device();
  let default_sampler = Sampler::new(
    &device,
    vk::Filter::NEAREST,
    vk::Filter::NEAREST,
    vk::SamplerMipmapMode::NEAREST,
    vk::SamplerAddressMode::REPEAT,
    vk::SamplerAddressMode::REPEAT,
  )?;
  finished_textures.push(Texture {
    image_view: default_image_view,
    sampler: default_sampler,
  });

  Ok(finished_textures)
}

fn parse_sampler(sampler: &gltf::texture::Sampler, device: &Arc<Device>) -> Result<Sampler> {
  let mag_filter = sampler.mag_filter().unwrap_or(gltf::texture::MagFilter::Linear);
  let mag_filter = match mag_filter {
    gltf::texture::MagFilter::Nearest => vk::Filter::NEAREST,
    gltf::texture::MagFilter::Linear => vk::Filter::LINEAR,
  };

  let min_filter = sampler.min_filter().unwrap_or(gltf::texture::MinFilter::Linear);
  let mipmap_mode = match min_filter {
    gltf::texture::MinFilter::Nearest => vk::SamplerMipmapMode::NEAREST,
    gltf::texture::MinFilter::Linear => vk::SamplerMipmapMode::NEAREST,
    gltf::texture::MinFilter::NearestMipmapNearest => vk::SamplerMipmapMode::NEAREST,
    gltf::texture::MinFilter::LinearMipmapNearest => vk::SamplerMipmapMode::NEAREST,
    gltf::texture::MinFilter::NearestMipmapLinear => vk::SamplerMipmapMode::LINEAR,
    gltf::texture::MinFilter::LinearMipmapLinear => vk::SamplerMipmapMode::LINEAR,
  };
  let min_filter = match min_filter {
    gltf::texture::MinFilter::Nearest => vk::Filter::NEAREST,
    gltf::texture::MinFilter::Linear => vk::Filter::LINEAR,
    gltf::texture::MinFilter::NearestMipmapNearest => vk::Filter::NEAREST,
    gltf::texture::MinFilter::LinearMipmapNearest => vk::Filter::LINEAR,
    gltf::texture::MinFilter::NearestMipmapLinear => vk::Filter::NEAREST,
    gltf::texture::MinFilter::LinearMipmapLinear => vk::Filter::LINEAR,
  };

  let address_mode_u = match sampler.wrap_s() {
    gltf::texture::WrappingMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
    gltf::texture::WrappingMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
    gltf::texture::WrappingMode::Repeat => vk::SamplerAddressMode::REPEAT,
  };

  let address_mode_v = match sampler.wrap_t() {
    gltf::texture::WrappingMode::ClampToEdge => vk::SamplerAddressMode::CLAMP_TO_EDGE,
    gltf::texture::WrappingMode::MirroredRepeat => vk::SamplerAddressMode::MIRRORED_REPEAT,
    gltf::texture::WrappingMode::Repeat => vk::SamplerAddressMode::REPEAT,
  };

  Sampler::new(device, mag_filter, min_filter, mipmap_mode, address_mode_u, address_mode_v)
}

fn parse_accessors(gltf: &Document) -> Vec<Accessor> {
  let mut accessors = Vec::new();

  for accessor in gltf.accessors() {
    let buffer_view = accessor.view().expect("GLTF models with sparse accessors are not yet supported!").index();
    let byte_offset = accessor.offset();
    let component_type = accessor.data_type();
    let normalized = accessor.normalized();
    let count = accessor.count();
    let data_type = accessor.dimensions();
    let max = accessor.max(); // Get the field
    let max = max.and_then(|field| field.as_array().map(|vector| vector.to_owned())); // Turn it into an array
    let max = max.and_then(|vector| vector.into_iter().map(|value| value.as_f64()).collect::<Option<Vec<_>>>()); // Turn all the values inside into floats
    let min = accessor.min(); // Get the field
    let min = min.and_then(|field| field.as_array().map(|vector| vector.to_owned())); // Turn it into an array
    let min = min.and_then(|vector| vector.into_iter().map(|value| value.as_f64()).collect::<Option<Vec<_>>>()); // Turn all the values inside into floats

    accessors.push(Accessor {
      buffer_view,
      byte_offset,
      component_type,
      normalized,
      count,
      data_type,
      max,
      min,
    })
  }

  accessors
}

fn parse_buffer_views(gltf: &Document) -> Vec<BufferView> {
  let mut buffer_views = Vec::new();

  for buffer_view in gltf.views() {
    let buffer = buffer_view.buffer().index();
    let byte_offset = buffer_view.offset();
    let byte_length = buffer_view.length();
    let byte_stride = buffer_view.stride();
    let target = buffer_view.target();
    let name = buffer_view.name().unwrap_or("Buffer View").to_owned();

    buffer_views.push(BufferView {
      buffer,
      byte_offset,
      byte_length,
      byte_stride,
      target,
      name,
    })
  }

  buffer_views
}

fn parse_buffers(allocator: &mut Allocator, buffers: Vec<gltf::buffer::Data>) -> Result<Vec<Buffer>> {
  let mut finished_buffers = Vec::new();

  for buffer in buffers {
    let data = buffer.0;
    let buffer = allocator.create_buffer_from_data(&data, vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER, BufferType::GpuOnly)?;

    finished_buffers.push(buffer);
  }

  let color_default_data = bincode::serialize(&Vec4::new(1.0, 1.0, 1.0, 1.0)).unwrap();
  let buffer = allocator.create_buffer_from_data(&color_default_data, vk::BufferUsageFlags::VERTEX_BUFFER, BufferType::GpuOnly)?;

  finished_buffers.push(buffer);

  Ok(finished_buffers)
}

fn parse_images(allocator: &mut Allocator, images: Vec<gltf::image::Data>) -> Result<Vec<Image>> {
  let mut finished_images = Vec::new();

  let default_image_info = vk::ImageCreateInfo {
    format: vk::Format::R32G32B32A32_SFLOAT,
    tiling: vk::ImageTiling::OPTIMAL,
    usage: vk::ImageUsageFlags::SAMPLED,
    image_type: vk::ImageType::TYPE_2D,
    samples: vk::SampleCountFlags::TYPE_1,
    mip_levels: 1,
    array_layers: 1,
    extent: vk::Extent3D { width: 1, height: 1, depth: 1 },
    ..Default::default()
  };

  for image in images {
    let data = image.pixels;
    let image_info = vk::ImageCreateInfo {
      format: convert_image_format(image.format),
      extent: vk::Extent3D {
        width: image.width,
        height: image.height,
        depth: 1,
      },
      ..default_image_info
    };

    let image = allocator.create_image(&data, image_info, ImagePurpose::Texture)?;

    finished_images.push(image);
  }

  let default_image_data = Vec4::new(1.0, 1.0, 1.0, 1.0);
  let default_image_data = bincode::serialize(&default_image_data).unwrap();
  let default_image = allocator.create_image(&default_image_data, default_image_info, ImagePurpose::Texture)?;
  finished_images.push(default_image);

  Ok(finished_images)
}

//------------------------------------------Model drawing----------------------------------------------------

impl GltfModel {
  fn draw_scene(&self, scene: &Scene, rendering_context: &mut RenderingContext) {
    for node in &scene.nodes {
      let Some(node) = self.nodes.get(*node) else {
        error!("selected node does not exist in this gltf model, skipping...");
        continue;
      };

      self.draw_node(node, rendering_context, Mat4::IDENTITY);
    }
  }

  fn draw_node(&self, node: &Node, rendering_context: &mut RenderingContext, matrix: Mat4) {
    let matrix = matrix.clone().mul_mat4(&node.matrix);

    // if let Some(camera) = node.camera {
    //   todo!();
    // }

    // if let Some(skin) = node.skin {
    //   todo!();
    // }

    if let Some(mesh) = node.mesh {
      if let Some(mesh) = self.meshes.get(mesh) {
        rendering_context.cmd_push_constants(&matrix);
        self.draw_mesh(mesh, rendering_context, node.weights.as_ref())
      } else {
        error!("selected mesh does not exist in this gltf model, skipping...");
      }
    }

    for node in &node.children {
      let Some(node) = self.nodes.get(*node) else {
        error!("selected node does not exist in this gltf model, skipping...");
        continue;
      };

      self.draw_node(node, rendering_context, matrix);
    }
  }

  fn draw_mesh(&self, mesh: &Mesh, rendering_context: &mut RenderingContext, weights: Option<&Vec<f32>>) {
    let weights = match weights {
      Some(_) => weights,
      None => mesh.weights.as_ref(),
    };

    for primitive in &mesh.primitives {
      if let Err(error) = self.draw_primitive(primitive, rendering_context, weights) {
        error!("failed to draw primitive: {:?}", error);
        continue;
      };
    }
  }

  fn draw_primitive(&self, primitive: &Primitive, rendering_context: &mut RenderingContext, weights: Option<&Vec<f32>>) -> Result<()> {
    let mut vertex_info = self.parse_attributes(&primitive.attributes)?;
    let index_info = if let Some(indices) = primitive.indices { Some(self.parse_indices(indices)?) } else { None };

    if let Some(material_index) = primitive.material {
      let material = self.materials.get(material_index).ok_or(ModelError::NoResource("Tried to access material that is not present"))?;
      rendering_context.set_descriptor_set(&self.material_descriptors[material_index]);
      self.parse_material(material, &primitive.attributes, &mut vertex_info)?;
    }

    let mesh_context = MeshContext {
      vertex_info,
      index_info,
      topology: primitive.mode,
    };

    rendering_context.draw_mesh(mesh_context);
    Ok(())
  }

  fn parse_attributes(&self, attribute: &Attributes) -> Result<VertexInfo> {
    let mut vertex_info = VertexInfo::default();

    let position = self.parse_attribute(attribute.position)?;
    let count = position.count;
    vertex_info.add_attribute(position, AttributeType::Position);

    if let Some(normal) = attribute.normal {
      vertex_info.add_attribute(self.parse_attribute(normal)?, AttributeType::Normal);
    }

    if let Some(tangent) = attribute.tangent {
      vertex_info.add_attribute(self.parse_attribute(tangent)?, AttributeType::Tangent);
    }

    if let Some(color) = attribute.colors.first() {
      vertex_info.add_attribute(self.parse_attribute(*color)?, AttributeType::Color);
    } else {
      let color = Attribute {
        buffer: *self.default_color_buffer,
        buffer_offset: 0,
        attribute_format: vk::Format::R32G32B32A32_SFLOAT,
        attribute_offset: 0,
        attribute_stride: 0, // if stride is 0, vulkan will always use the first point of data the buffer has for every vertex.
        count,
      };

      vertex_info.add_attribute(color, AttributeType::Color);
    }

    Ok(vertex_info)
  }

  fn parse_attribute(&self, attribute: usize) -> Result<Attribute> {
    let accessor = self.accessors.get(attribute).ok_or(ModelError::NoResource("tried using accessor with invalid index"))?;
    let buffer_view = self
      .buffer_views
      .get(accessor.buffer_view)
      .ok_or(ModelError::NoResource("tried using buffer view with invalid index"))?;
    let buffer = self.buffers.get(buffer_view.buffer).ok_or(ModelError::NoResource("tried using buffer with invalid index"))?;

    let format = parse_format(accessor.component_type, accessor.data_type)?;
    let stride = if let Some(stride) = buffer_view.byte_stride {
      stride as u32
    } else {
      parse_stride(accessor.component_type, accessor.data_type)?
    };
    Ok(Attribute {
      buffer: **buffer,
      buffer_offset: buffer_view.byte_offset as u64,
      attribute_format: format,
      attribute_offset: accessor.byte_offset as u32,
      attribute_stride: stride,
      count: accessor.count as u32,
    })
  }

  fn parse_material(&self, material: &MaterialDetails, attributes: &Attributes, vertex_info: &mut VertexInfo) -> Result<()> {
    if let Some(color_texture) = material.color_texcoord {
      vertex_info.add_attribute(self.get_texcoord(color_texture, attributes)?, AttributeType::Texcoord);
    }

    if let Some(metallic_roughness_texture) = material.metallic_roughness_texcoord {
      vertex_info.add_attribute(self.get_texcoord(metallic_roughness_texture, attributes)?, AttributeType::Matcoord);
    }

    if let Some(norm_texture) = material.normals_texcoord {
      vertex_info.add_attribute(self.get_texcoord(norm_texture, attributes)?, AttributeType::Normcoord);
    }
    if let Some(occlusion_texture) = material.occlusion_texcoord {
      vertex_info.add_attribute(self.get_texcoord(occlusion_texture, attributes)?, AttributeType::Occlusioncoord);
    }
    if let Some(emissive_texture) = material.emissive_texcoord {
      vertex_info.add_attribute(self.get_texcoord(emissive_texture, attributes)?, AttributeType::Emissivecoord);
    }

    Ok(())
  }

  fn get_texcoord(&self, texcoord: usize, attributes: &Attributes) -> Result<Attribute> {
    if let Some(texcoord) = attributes.texcoords.get(texcoord) {
      self.parse_attribute(*texcoord)
    } else {
      Err(ModelError::NoResource("Tried accessing a texcoord that doesn't exist!"))?
    }
  }

  fn parse_indices(&self, indices: usize) -> Result<IndexInfo> {
    let accessor = self.accessors.get(indices).ok_or(ModelError::NoResource("tried using accessor with invalid index"))?;
    let buffer_view = self
      .buffer_views
      .get(accessor.buffer_view)
      .ok_or(ModelError::NoResource("tried using buffer view with invalid index"))?;
    let buffer = self.buffers.get(buffer_view.buffer).ok_or(ModelError::NoResource("tried using buffer with invalid index"))?;

    use gltf::accessor::DataType as DT;
    let index_type = match accessor.component_type {
      DT::I8 => vk::IndexType::UINT8_EXT,
      DT::U16 => vk::IndexType::UINT16,
      DT::U32 => vk::IndexType::UINT32,
      _ => Err(ModelError::InvalidField("mesh indicies have an invalid data format"))?,
    };

    Ok(IndexInfo {
      buffer: **buffer,
      count: accessor.count as u32,
      offset: (accessor.byte_offset + buffer_view.byte_offset) as u64,
      index_type,
    })
  }
}

fn convert_image_format(format: gltf::image::Format) -> vk::Format {
  match format {
    gltf::image::Format::R8 => vk::Format::R8_SRGB,
    gltf::image::Format::R8G8 => vk::Format::R8G8_SRGB,
    gltf::image::Format::R8G8B8 => vk::Format::R8G8B8_SRGB,
    gltf::image::Format::R8G8B8A8 => vk::Format::R8G8B8A8_SRGB,
    gltf::image::Format::R16 => vk::Format::R16_UINT,
    gltf::image::Format::R16G16 => vk::Format::R16G16_UINT,
    gltf::image::Format::R16G16B16 => vk::Format::R16G16B16_UINT,
    gltf::image::Format::R16G16B16A16 => vk::Format::R16G16B16A16_UINT,
    gltf::image::Format::R32G32B32FLOAT => vk::Format::R32G32B32_SFLOAT,
    gltf::image::Format::R32G32B32A32FLOAT => vk::Format::R32G32B32A32_SFLOAT,
  }
}

fn parse_format(component: gltf::accessor::DataType, data_type: gltf::accessor::Dimensions) -> Result<vk::Format> {
  use gltf::accessor::DataType as DT;
  use gltf::accessor::Dimensions as DIM;

  match (component, data_type) {
    (DT::F32, DIM::Vec2) => Ok(vk::Format::R32G32_SFLOAT),
    (DT::F32, DIM::Vec3) => Ok(vk::Format::R32G32B32_SFLOAT),
    (DT::F32, DIM::Vec4) => Ok(vk::Format::R32G32B32A32_SFLOAT),
    (DT::U16, DIM::Vec2) => Ok(vk::Format::R16G16_UNORM),
    (DT::U16, DIM::Vec3) => Ok(vk::Format::R16G16B16_UNORM),
    (DT::U16, DIM::Vec4) => Ok(vk::Format::R16G16B16A16_UNORM),
    (DT::U8, DIM::Vec2) => Ok(vk::Format::R8G8_UNORM),
    (DT::U8, DIM::Vec3) => Ok(vk::Format::R8G8B8_UNORM),
    (DT::U8, DIM::Vec4) => Ok(vk::Format::R8G8B8A8_UNORM),
    (_, _) => Err(ModelError::InvalidField("mesh primitive has an impossible format"))?,
  }
}

fn parse_stride(component: gltf::accessor::DataType, data_type: gltf::accessor::Dimensions) -> Result<u32> {
  use gltf::accessor::DataType as DT;
  use gltf::accessor::Dimensions as DIM;

  match (component, data_type) {
    (DT::F32, DIM::Vec2) => Ok(8),
    (DT::F32, DIM::Vec3) => Ok(12),
    (DT::F32, DIM::Vec4) => Ok(16),
    (DT::U16, DIM::Vec2) => Ok(4),
    (DT::U16, DIM::Vec3) => Ok(6),
    (DT::U16, DIM::Vec4) => Ok(8),
    (DT::U8, DIM::Vec2) => Ok(2),
    (DT::U8, DIM::Vec3) => Ok(3),
    (DT::U8, DIM::Vec4) => Ok(4),
    (_, _) => Err(ModelError::InvalidField("mesh primitive has an impossible stride"))?,
  }
}
