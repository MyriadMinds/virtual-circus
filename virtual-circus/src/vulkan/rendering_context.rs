use super::descriptors::{DescriptorSet, DescriptorSets};
use super::elements::PipelineLayout;
use super::Device;
use crate::framework::Model;
use crate::utils::constants::*;
use crate::utils::tools::Result;

use ash::vk;
use nalgebra_glm::*;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct PushConstant {
  pub(crate) time: f32,
  pub(crate) matrix: Mat4,
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

  pub(crate) fn draw_model(&self, model: &Model) {
    unsafe {
      for mesh in &model.meshes {
        self.device.cmd_bind_vertex_buffers(*self.command_buffer, 0, &[*model.buffer], &[mesh.vertex_offset as u64]);

        self.device.cmd_bind_index_buffer(*self.command_buffer, *model.buffer, mesh.index_offset as u64, vk::IndexType::UINT32);
        self.device.cmd_draw_indexed(*self.command_buffer, mesh.index_count, 1, 0, 0, 0);
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

  pub(crate) fn complete_rendering_command(&mut self) {
    unsafe { self.device.cmd_end_rendering(*self.command_buffer) };
  }

  pub(crate) fn end_command_buffer(&mut self) -> Result<()> {
    unsafe { Ok(self.device.end_command_buffer(*self.command_buffer)?) }
  }

  pub(crate) fn command_buffer(&self) -> &vk::CommandBuffer {
    self.command_buffer
  }
}
