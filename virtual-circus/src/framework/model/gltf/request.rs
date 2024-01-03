use super::{Accessor, Attributes, BufferView, GltfModel, Mesh, Node, Primitive, Scene};
use crate::utils::tools::{EngineError, ModelError, Result};
use crate::vulkan::allocator::{Buffer, Image, ImagePurpose};
use crate::vulkan::Allocator;

use ash::vk;
use glam::*;
use gltf::Document;

use std::sync::mpsc::{Receiver, TryRecvError};

pub(crate) struct GltfModelRequest {
  default_scene: Option<usize>,
  scenes: Vec<Scene>,
  nodes: Vec<Node>,
  meshes: Vec<Mesh>,
  accessors: Vec<Accessor>,
  buffer_views: Vec<BufferView>,
  buffers: Option<Vec<Buffer>>,
  images: Option<Vec<Image>>,
  buffer_requests: Receiver<Vec<Buffer>>,
  image_requests: Receiver<Vec<Image>>,
}

impl GltfModelRequest {
  pub(crate) fn new(path: &str, allocator: &Allocator) -> Result<Self> {
    let (gltf_document, buffers, images) = gltf::import(path).map_err(|error| ModelError::GltfError(error))?;
    let buffer_requests = parse_buffers(buffers);
    let buffer_requests = allocator.create_buffers(buffer_requests)?;
    let image_requests = parse_images(images);
    let image_requests = allocator.create_images(image_requests)?;

    let default_scene = gltf_document.default_scene().map(|scene| scene.index());
    let scenes = parse_scenes(&gltf_document);
    let nodes = parse_nodes(&gltf_document);
    let meshes = parse_meshes(&gltf_document);
    let accessors = parse_accessors(&gltf_document);
    let buffer_views = parse_buffer_views(&gltf_document);

    Ok(Self {
      default_scene,
      scenes,
      nodes,
      meshes,
      accessors,
      buffer_views,
      buffers: None,
      images: None,
      buffer_requests,
      image_requests,
    })
  }

  pub(crate) fn can_be_finalized(&mut self) -> Result<()> {
    if let None = self.buffers {
      match self.buffer_requests.try_recv() {
        Ok(buffers) => self.buffers = Some(buffers),
        Err(TryRecvError::Empty) => return Err(EngineError::ResourceNotReady),
        Err(TryRecvError::Disconnected) => return Err(EngineError::CreationError("model did not receive buffers, state corrupted")),
      }
    }

    if let None = self.images {
      match self.image_requests.try_recv() {
        Ok(images) => self.images = Some(images),
        Err(TryRecvError::Empty) => return Err(EngineError::ResourceNotReady),
        Err(TryRecvError::Disconnected) => return Err(EngineError::CreationError("model did not receive images, state corrupted")),
      }
    }

    Ok(())
  }

  pub(crate) fn finalize(self) -> Result<GltfModel> {
    let Some(buffers) = self.buffers else {
      return Err(EngineError::ResourceNotReady);
    };

    let Some(images) = self.images else {
      return Err(EngineError::ResourceNotReady);
    };

    Ok(GltfModel {
      default_scene: self.default_scene,
      scenes: self.scenes,
      nodes: self.nodes,
      meshes: self.meshes,
      accessors: self.accessors,
      buffer_views: self.buffer_views,
      buffers,
      images,
    })
  }
}

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
    let attributes = Attributes {
      position: primitive.attributes().find(|attr| attr.0 == gltf::Semantic::Positions).unwrap().1.index(),
      normal: primitive.attributes().find(|attr| attr.0 == gltf::Semantic::Normals).map(|attr| attr.1.index()),
      tangent: primitive.attributes().find(|attr| attr.0 == gltf::Semantic::Tangents).map(|attr| attr.1.index()),
      texcoord_0: primitive.attributes().find(|attr| attr.0 == gltf::Semantic::TexCoords(0)).map(|attr| attr.1.index()),
      color_0: primitive.attributes().find(|attr| attr.0 == gltf::Semantic::Colors(0)).map(|attr| attr.1.index()),
    };

    let indices = primitive.indices().map(|accessor| accessor.index());
    let material = primitive.material().index();

    let mut targets = Vec::new();
    for target in primitive.morph_targets() {
      let target = Attributes {
        position: target.positions().unwrap().index(),
        normal: target.normals().map(|attribute| attribute.index()),
        tangent: target.tangents().map(|attribute| attribute.index()),
        texcoord_0: None,
        color_0: None,
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
    let max = max.map(|field| field.as_array().map(|vector| vector.to_owned())).flatten(); // Turn it into an array
    let max = max.map(|vector| vector.into_iter().map(|value| value.as_f64()).collect::<Option<Vec<_>>>()).flatten(); // Turn all the values inside into floats
    let min = accessor.min(); // Get the field
    let min = min.map(|field| field.as_array().map(|vector| vector.to_owned())).flatten(); // Turn it into an array
    let min = min.map(|vector| vector.into_iter().map(|value| value.as_f64()).collect::<Option<Vec<_>>>()).flatten(); // Turn all the values inside into floats

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

fn parse_buffers(buffers: Vec<gltf::buffer::Data>) -> Vec<BufferInfo> {
  let mut requests = Vec::new();

  for buffer in buffers {
    let data = buffer.0;
    let buffer_request = BufferInfo {
      data,
      usage: vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER,
    };

    requests.push(buffer_request);
  }

  let color_default_data = bincode::serialize(&Vec4::new(1.0, 1.0, 1.0, 1.0)).unwrap();
  let buffer_request = BufferInfo {
    data: color_default_data,
    usage: vk::BufferUsageFlags::VERTEX_BUFFER,
  };

  requests.push(buffer_request);

  requests
}

fn parse_images(images: Vec<gltf::image::Data>) -> Vec<ImageInfo> {
  let mut requests = Vec::new();

  for image in images {
    let data = image.pixels;
    let image_create_info = vk::ImageCreateInfo {
      format: convert_image_format(image.format),
      tiling: vk::ImageTiling::OPTIMAL,
      usage: vk::ImageUsageFlags::SAMPLED,
      image_type: vk::ImageType::TYPE_2D,
      samples: vk::SampleCountFlags::TYPE_1,
      mip_levels: 1,
      array_layers: 1,
      extent: vk::Extent3D {
        width: image.width,
        height: image.height,
        depth: 1,
      },
      ..Default::default()
    };

    let image_request = ImageInfo {
      data,
      image_create_info,
      purpose: ImagePurpose::Texture,
    };

    requests.push(image_request);
  }

  requests
}

fn convert_image_format(format: gltf::image::Format) -> vk::Format {
  match format {
    gltf::image::Format::R8 => vk::Format::R8_UINT,
    gltf::image::Format::R8G8 => vk::Format::R8G8_UINT,
    gltf::image::Format::R8G8B8 => vk::Format::R8G8B8_UINT,
    gltf::image::Format::R8G8B8A8 => vk::Format::R8G8B8A8_UINT,
    gltf::image::Format::R16 => vk::Format::R16_UINT,
    gltf::image::Format::R16G16 => vk::Format::R16G16_UINT,
    gltf::image::Format::R16G16B16 => vk::Format::R16G16B16_UINT,
    gltf::image::Format::R16G16B16A16 => vk::Format::R16G16B16A16_UINT,
    gltf::image::Format::R32G32B32FLOAT => vk::Format::R32G32B32_SFLOAT,
    gltf::image::Format::R32G32B32A32FLOAT => vk::Format::R32G32B32A32_SFLOAT,
  }
}
