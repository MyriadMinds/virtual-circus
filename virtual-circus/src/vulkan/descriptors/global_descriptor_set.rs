use super::super::allocator::{Buffer, BufferType};
use super::super::{Allocator, Device};
use super::{DescriptorSet, DescriptorSetImpl, DescriptorSetLayoutImpl, DescriptorSets};
use crate::utils::constants::GLOBAL_DESCRIPTOR_BINDING;
use crate::utils::tools::Result;

use ash::vk;
use glam::*;
use log::debug;
use serde::Serialize;

use std::mem::size_of;
use std::ops::{Index, IndexMut};
use std::sync::Arc;

#[derive(Serialize, Default, Debug)]
pub(crate) struct GlobalDescriptorSetInfo {
  pub(crate) model: Mat4,
  pub(crate) view: Mat4,
  pub(crate) projection: Mat4,
}

//---------------------------------Layout--------------------------------------------------

pub(crate) struct GlobalDescriptorSetLayout {
  descriptor_set_layout: DescriptorSetLayoutImpl,
}

impl GlobalDescriptorSetLayout {
  pub(crate) fn new(device: &Arc<Device>) -> Result<Self> {
    let bindings = [vk::DescriptorSetLayoutBinding {
      binding: 0,
      descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
      descriptor_count: 1,
      stage_flags: vk::ShaderStageFlags::VERTEX,
      p_immutable_samplers: std::ptr::null(),
    }];

    let descriptor_set_layout = DescriptorSetLayoutImpl::new(device, &bindings)?;
    Ok(Self { descriptor_set_layout })
  }

  pub(crate) fn create_descriptor_sets(&self, allocator: &mut Allocator, count: usize) -> Result<GlobalDescriptorSets> {
    let (descriptor_buffer, descriptor_sets) = self.descriptor_set_layout.create_descriptor_sets(allocator, count)?;
    GlobalDescriptorSets::new(allocator, descriptor_buffer, descriptor_sets)
  }
}

impl std::ops::Deref for GlobalDescriptorSetLayout {
  type Target = vk::DescriptorSetLayout;

  fn deref(&self) -> &Self::Target {
    &self.descriptor_set_layout
  }
}

//---------------------------------Descriptor Sets-------------------------------------------------

pub(crate) struct GlobalDescriptorSets {
  descriptor_buffer: Buffer,
  descriptor_sets: Vec<GlobalDescriptorSet>,
}

impl GlobalDescriptorSets {
  fn new(allocator: &mut Allocator, mut descriptor_buffer: Buffer, descriptor_set_impls: Vec<DescriptorSetImpl>) -> Result<Self> {
    let mut descriptor_sets = Vec::with_capacity(descriptor_set_impls.len());
    for descriptor_set_impl in descriptor_set_impls {
      descriptor_sets.push(GlobalDescriptorSet::new(allocator, &mut descriptor_buffer, descriptor_set_impl)?);
    }

    Ok(Self { descriptor_buffer, descriptor_sets })
  }
}

impl DescriptorSets for GlobalDescriptorSets {
  fn get_descriptor_buffer_info(&self) -> (vk::DescriptorBufferBindingInfoEXT, usize) {
    let binding_info = vk::DescriptorBufferBindingInfoEXT {
      address: self.descriptor_buffer.device_address(),
      usage: vk::BufferUsageFlags::RESOURCE_DESCRIPTOR_BUFFER_EXT,
      ..Default::default()
    };

    (binding_info, GLOBAL_DESCRIPTOR_BINDING)
  }
}

impl Index<usize> for GlobalDescriptorSets {
  type Output = GlobalDescriptorSet;

  fn index(&self, index: usize) -> &Self::Output {
    &self.descriptor_sets[index]
  }
}

impl IndexMut<usize> for GlobalDescriptorSets {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    &mut self.descriptor_sets[index]
  }
}

//---------------------------------Descriptor Set--------------------------------------------------
pub(crate) struct GlobalDescriptorSet {
  descriptor_set: DescriptorSetImpl,
  buffer: Buffer,
}

impl GlobalDescriptorSet {
  fn new(allocator: &mut Allocator, descriptor_buffer: &mut Buffer, descriptor_set: DescriptorSetImpl) -> Result<Self> {
    let usage = vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
    let buffer = allocator.create_buffer(size_of::<GlobalDescriptorSetInfo>() as u64, usage, BufferType::CpuVisible)?;

    let data = vk::DescriptorAddressInfoEXT {
      address: buffer.device_address(),
      range: buffer.size(),
      format: vk::Format::UNDEFINED,
      ..Default::default()
    };

    let get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::UNIFORM_BUFFER,
      data: vk::DescriptorDataEXT { p_uniform_buffer: [data].as_ptr() },
      ..Default::default()
    };

    descriptor_set.write_descriptor(&[get_info], descriptor_buffer);

    Ok(Self { descriptor_set, buffer })
  }

  pub(crate) fn update_descriptor(&mut self, info: GlobalDescriptorSetInfo) -> Result<()> {
    debug!("descriptor data: {:?}", info);
    let data = bincode::serialize(&info).unwrap();
    self.buffer.load_data(&data)
  }
}

impl DescriptorSet for GlobalDescriptorSet {
  fn get_descriptor_set_info(&self) -> (u64, usize) {
    (self.descriptor_set.get_descriptor_set_offset(), GLOBAL_DESCRIPTOR_BINDING)
  }
}
