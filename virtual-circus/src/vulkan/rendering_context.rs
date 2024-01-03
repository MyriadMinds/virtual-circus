use crate::utils::constants::*;

use super::descriptors::{DescriptorSet, DescriptorSets};
use super::elements::PipelineLayout;
use super::Device;

use ash::vk;
use glam::*;
use serde::Serialize;

const ATTRIBUTE_COUNT: usize = 9;

const POSITION_BINDING: u32 = 0;
const NORMAL_BINDING: u32 = 1;
const TANGENT_BINDING: u32 = 2;
const TEXCOORD_BINDING: u32 = 3;
const MATCOORD_BINDING: u32 = 4;
const NORMCOORD_BINDING: u32 = 5;
const OCCLISIONCOORD_BINDING: u32 = 6;
const EMISSIONCOORD_BINDING: u32 = 7;
const COLOR_BINDING: u32 = 8;

#[derive(Serialize)]
pub(crate) struct PushConstant {
  pub(crate) time: f32,
  pub(crate) matrix: Mat4,
}

pub(crate) enum AttributeType {
  Position,
  Normal,
  Tangent,
  Texcoord,
  Matcoord,
  Normcoord,
  Occlusioncoord,
  Emissivecoord,
  Color,
}

impl AttributeType {
  fn get_binding(&self) -> u32 {
    match self {
      AttributeType::Position => POSITION_BINDING,
      AttributeType::Normal => NORMAL_BINDING,
      AttributeType::Tangent => TANGENT_BINDING,
      AttributeType::Texcoord => TEXCOORD_BINDING,
      AttributeType::Matcoord => MATCOORD_BINDING,
      AttributeType::Normcoord => NORMCOORD_BINDING,
      AttributeType::Occlusioncoord => OCCLISIONCOORD_BINDING,
      AttributeType::Emissivecoord => EMISSIONCOORD_BINDING,
      AttributeType::Color => COLOR_BINDING,
    }
  }
}

pub(crate) struct Attribute {
  pub(crate) buffer: vk::Buffer,
  pub(crate) buffer_offset: vk::DeviceSize,
  pub(crate) attribute_format: vk::Format,
  pub(crate) attribute_offset: u32,
  pub(crate) attribute_stride: u32,
  pub(crate) count: u32,
}

impl Attribute {
  fn get_binding_description(&self, binding: u32) -> vk::VertexInputBindingDescription2EXT {
    vk::VertexInputBindingDescription2EXT {
      binding,
      stride: self.attribute_stride,
      input_rate: vk::VertexInputRate::VERTEX,
      divisor: 1,
      ..Default::default()
    }
  }

  fn get_attribute_description(&self, binding: u32) -> vk::VertexInputAttributeDescription2EXT {
    vk::VertexInputAttributeDescription2EXT {
      location: binding,
      binding,
      format: self.attribute_format,
      offset: self.attribute_offset,
      ..Default::default()
    }
  }
}

impl Default for Attribute {
  fn default() -> Self {
    Self {
      buffer: vk::Buffer::null(),
      buffer_offset: 0,
      attribute_format: vk::Format::UNDEFINED,
      attribute_offset: 0,
      attribute_stride: 0,
      count: 0,
    }
  }
}

pub(crate) struct VertexInfo {
  buffers: [vk::Buffer; ATTRIBUTE_COUNT],
  bindings: [vk::VertexInputBindingDescription2EXT; ATTRIBUTE_COUNT],
  attributes: [vk::VertexInputAttributeDescription2EXT; ATTRIBUTE_COUNT],
  offsets: [vk::DeviceSize; ATTRIBUTE_COUNT],
  count: u32,
}

impl VertexInfo {
  pub(crate) fn add_attribute(&mut self, attribute: Attribute, attribute_type: AttributeType) {
    let binding = attribute_type.get_binding();
    let index = binding as usize;

    self.buffers[index] = attribute.buffer;
    self.bindings[index] = attribute.get_binding_description(binding);
    self.attributes[index] = attribute.get_attribute_description(binding);
    self.offsets[index] = attribute.buffer_offset;
    self.count = attribute.count;
  }
}

impl Default for VertexInfo {
  fn default() -> Self {
    let position = Attribute {
      attribute_format: vk::Format::R32G32B32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec3>() as u32,
      ..Default::default()
    };
    let normal = Attribute {
      attribute_format: vk::Format::R32G32B32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec3>() as u32,
      ..Default::default()
    };
    let tangent = Attribute {
      attribute_format: vk::Format::R32G32B32A32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec4>() as u32,
      ..Default::default()
    };
    let texcoord = Attribute {
      attribute_format: vk::Format::R32G32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec2>() as u32,
      ..Default::default()
    };
    let matcoord = Attribute {
      attribute_format: vk::Format::R32G32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec2>() as u32,
      ..Default::default()
    };
    let normcoord = Attribute {
      attribute_format: vk::Format::R32G32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec2>() as u32,
      ..Default::default()
    };
    let occlusioncoord = Attribute {
      attribute_format: vk::Format::R32G32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec2>() as u32,
      ..Default::default()
    };
    let emissioncoord = Attribute {
      attribute_format: vk::Format::R32G32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec2>() as u32,
      ..Default::default()
    };
    let color = Attribute {
      attribute_format: vk::Format::R32G32B32A32_SFLOAT,
      attribute_stride: std::mem::size_of::<glam::Vec4>() as u32,
      ..Default::default()
    };

    let buffers = [vk::Buffer::null(); ATTRIBUTE_COUNT];
    let bindings = [vk::VertexInputBindingDescription2EXT::default(); ATTRIBUTE_COUNT];
    let attributes = [vk::VertexInputAttributeDescription2EXT::default(); ATTRIBUTE_COUNT];
    let offsets = [vk::DeviceSize::default(); ATTRIBUTE_COUNT];

    let mut vertex_info = Self {
      buffers,
      bindings,
      attributes,
      offsets,
      count: 0,
    };

    vertex_info.add_attribute(position, AttributeType::Position);
    vertex_info.add_attribute(normal, AttributeType::Normal);
    vertex_info.add_attribute(tangent, AttributeType::Tangent);
    vertex_info.add_attribute(texcoord, AttributeType::Texcoord);
    vertex_info.add_attribute(matcoord, AttributeType::Matcoord);
    vertex_info.add_attribute(normcoord, AttributeType::Normcoord);
    vertex_info.add_attribute(occlusioncoord, AttributeType::Occlusioncoord);
    vertex_info.add_attribute(emissioncoord, AttributeType::Emissivecoord);
    vertex_info.add_attribute(color, AttributeType::Color);

    vertex_info
  }
}

pub(crate) struct IndexInfo {
  pub(crate) buffer: vk::Buffer,
  pub(crate) count: u32,
  pub(crate) offset: vk::DeviceSize,
  pub(crate) index_type: vk::IndexType,
}

#[derive(Default)]
pub(crate) struct MeshContext {
  pub(crate) vertex_info: VertexInfo,
  pub(crate) index_info: Option<IndexInfo>,
  pub(crate) topology: vk::PrimitiveTopology,
}

pub(crate) struct RenderingContext<'a> {
  device: &'a Device,
  command_buffer: &'a vk::CommandBuffer,
  pipeline_layout: &'a PipelineLayout,
  descriptor_buffer_bindings: [Option<vk::DescriptorBufferBindingInfoEXT>; DESCRIPTOR_SET_COUNT],
  descriptor_buffer_offsets: [Option<u64>; DESCRIPTOR_SET_COUNT],
  time: f32,
}

impl<'a> RenderingContext<'a> {
  pub(crate) fn new(device: &'a Device, command_buffer: &'a vk::CommandBuffer, pipeline_layout: &'a PipelineLayout, time: f32) -> Self {
    Self {
      device,
      command_buffer,
      pipeline_layout,
      descriptor_buffer_bindings: [None; DESCRIPTOR_SET_COUNT],
      descriptor_buffer_offsets: [None; DESCRIPTOR_SET_COUNT],
      time,
    }
  }

  pub(crate) fn draw_mesh(&self, mesh: MeshContext) {
    unsafe {
      self.device.cmd_set_primitive_topology(*self.command_buffer, mesh.topology);
      self.device.cmd_set_vertex_input(*self.command_buffer, &mesh.vertex_info.bindings, &mesh.vertex_info.attributes);
      self.device.cmd_bind_vertex_buffers(*self.command_buffer, 0, &mesh.vertex_info.buffers, &mesh.vertex_info.offsets);

      if let Some(index_info) = mesh.index_info {
        self.device.cmd_bind_index_buffer(*self.command_buffer, index_info.buffer, index_info.offset, index_info.index_type);
        self.device.cmd_draw_indexed(*self.command_buffer, index_info.count, 1, 0, 0, 0);
      } else {
        self.device.cmd_draw(*self.command_buffer, mesh.vertex_info.count, 1, 0, 0);
      }
    }
  }

  pub(crate) fn cmd_push_constants(&self, matrix: &Mat4) {
    let push_constant = PushConstant { time: self.time, matrix: *matrix };
    let constant_data = bincode::serialize(&push_constant).unwrap();

    unsafe {
      self
        .device
        .cmd_push_constants(*self.command_buffer, **self.pipeline_layout, vk::ShaderStageFlags::VERTEX, 0, &constant_data)
    }
  }

  pub(crate) fn bind_descriptor_buffer(&mut self, descriptor_sets: &impl DescriptorSets) {
    let (buffer_info, binding_slot) = descriptor_sets.get_descriptor_buffer_info();
    self.descriptor_buffer_bindings[binding_slot] = Some(buffer_info);
    self.bind_descriptor_buffers();
  }

  fn bind_descriptor_buffers(&self) {
    let bindings = self.descriptor_buffer_bindings.iter().copied().flatten().collect::<Vec<vk::DescriptorBufferBindingInfoEXT>>();

    unsafe { self.device.cmd_bind_descriptor_buffers(*self.command_buffer, &bindings) };
    self.set_descriptor_sets();
  }

  pub(crate) fn set_descriptor_set(&mut self, descriptor_set: &impl DescriptorSet) {
    let (offset, binding_slot) = descriptor_set.get_descriptor_set_info();
    self.descriptor_buffer_offsets[binding_slot] = Some(offset);
    self.set_descriptor_sets();
  }

  fn set_descriptor_sets(&self) {
    let mut buffer_index = 0;

    let binding_slot = GLOBAL_DESCRIPTOR_BINDING;
    if self.descriptor_buffer_bindings[binding_slot].is_some() {
      if let Some(offset) = self.descriptor_buffer_offsets[binding_slot] {
        self.set_descriptor_offset(binding_slot as u32, buffer_index, offset);
      }
      // Even if there was no offset to configure for this descriptor set, we still found a binding so we need to progress the buffer index
      buffer_index += 1;
    }

    let binding_slot = MATERIAL_DESCRIPTOR_BINDING;
    if self.descriptor_buffer_bindings[binding_slot].is_some() {
      if let Some(offset) = self.descriptor_buffer_offsets[binding_slot] {
        self.set_descriptor_offset(binding_slot as u32, buffer_index, offset);
      }
      // Even if there was no offset to configure for this descriptor set, we still found a binding so we need to progress the buffer index
      buffer_index += 1;
    }
  }

  fn set_descriptor_offset(&self, descriptor_binding_slot: u32, buffer_index: u32, offset: u64) {
    unsafe {
      self.device.cmd_set_descriptor_buffer_offsets(
        *self.command_buffer,
        vk::PipelineBindPoint::GRAPHICS,
        **self.pipeline_layout,
        descriptor_binding_slot,
        &[buffer_index],
        &[offset],
      )
    }
  }
}
