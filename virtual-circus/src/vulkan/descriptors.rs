mod global_descriptor_set;
mod material_descriptor_set;

pub(crate) use global_descriptor_set::{GlobalDescriptorSetInfo, GlobalDescriptorSetLayout, GlobalDescriptorSets};
pub(crate) use material_descriptor_set::{MaterialDescriptorSetInfo, MaterialDescriptorSetLayout, MaterialDescriptorSets, MaterialFlags, MaterialInfo, TextureInfo};

use super::allocator::{Buffer, BufferType};
use super::Allocator;
use super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::{error, trace};
use std::sync::Arc;

pub(crate) trait DescriptorSet {
  fn get_descriptor_set_info(&self) -> (u64, usize);
}

pub(crate) trait DescriptorSets {
  fn get_descriptor_buffer_info(&self) -> (vk::DescriptorBufferBindingInfoEXT, usize);
}

//---------------------------------------Descriptor Set Layout--------------------------------------------
struct DescriptorSetLayoutImpl {
  device: Arc<Device>,
  descriptor_set_layout: vk::DescriptorSetLayout,
  layout_size: u64,
  binding_offsets: Vec<u64>,
  buffer_usage: vk::BufferUsageFlags,
}

impl DescriptorSetLayoutImpl {
  fn new(device: &Arc<Device>, bindings: &[vk::DescriptorSetLayoutBinding]) -> Result<Self> {
    // Figure out the usage flags of buffers that would back this descriptor set layout
    use vk::BufferUsageFlags as UF;
    use vk::DescriptorType as DT;
    let mut buffer_usage = UF::SHADER_DEVICE_ADDRESS;
    for binding in bindings {
      match binding.descriptor_type {
        DT::UNIFORM_BUFFER => buffer_usage |= UF::RESOURCE_DESCRIPTOR_BUFFER_EXT,
        DT::COMBINED_IMAGE_SAMPLER => buffer_usage |= UF::SAMPLER_DESCRIPTOR_BUFFER_EXT | UF::RESOURCE_DESCRIPTOR_BUFFER_EXT,
        _ => error!("Unsupported descriptor type used!"),
      }
    }

    // Create the layout and figure out its size in memory
    let layout_create_info = vk::DescriptorSetLayoutCreateInfo {
      flags: vk::DescriptorSetLayoutCreateFlags::DESCRIPTOR_BUFFER_EXT,
      binding_count: bindings.len() as u32,
      p_bindings: bindings.as_ptr(),
      ..Default::default()
    };

    let descriptor_set_layout = unsafe { device.create_descriptor_set_layout(&layout_create_info, None)? };
    let layout_size = unsafe { device.get_descriptor_set_layout_size(descriptor_set_layout) };

    // get the memory offsets of all descriptors in this layout
    let binding_count = bindings.len();
    let mut binding_offsets = Vec::with_capacity(binding_count);
    for binding in 0..binding_count {
      let offset = unsafe { device.get_descriptor_set_layout_binding_offset(descriptor_set_layout, binding as u32) };
      binding_offsets.push(offset);
    }

    Ok(Self {
      device: device.clone(),
      descriptor_set_layout,
      layout_size,
      binding_offsets,
      buffer_usage,
    })
  }

  fn create_descriptor_sets(&self, allocator: &mut Allocator, count: usize) -> Result<(Buffer, Vec<DescriptorSetImpl>)> {
    let backing_buffer = allocator.create_buffer(self.layout_size * count as u64, self.buffer_usage, BufferType::CpuVisible)?;

    let mut descriptor_sets = Vec::with_capacity(count);
    for i in 0..count {
      let buffer_offset = self.layout_size * i as u64;
      let descriptor_offsets = self.binding_offsets.clone();
      let descriptor_set = DescriptorSetImpl::new(&self.device, buffer_offset, descriptor_offsets);
      descriptor_sets.push(descriptor_set);
    }

    Ok((backing_buffer, descriptor_sets))
  }
}

impl Drop for DescriptorSetLayoutImpl {
  fn drop(&mut self) {
    trace!("Destroying descriptor set layout: {:?}", self.descriptor_set_layout);
    unsafe { self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None) };
  }
}

impl std::ops::Deref for DescriptorSetLayoutImpl {
  type Target = vk::DescriptorSetLayout;

  fn deref(&self) -> &Self::Target {
    &self.descriptor_set_layout
  }
}

//-----------------------------------Descriptor Set---------------------------------------------------

struct DescriptorSetImpl {
  device: Arc<Device>,
  buffer_offset: u64,
  descriptor_offsets: Vec<u64>,
}

impl DescriptorSetImpl {
  fn new(device: &Arc<Device>, buffer_offset: u64, descriptor_offsets: Vec<u64>) -> Self {
    Self {
      device: device.clone(),
      buffer_offset,
      descriptor_offsets,
    }
  }

  fn write_descriptor(&self, descriptor_infos: &[vk::DescriptorGetInfoEXT], descriptor_buffer: &mut Buffer) {
    let device_properties = unsafe { self.device.get_physical_device_descriptor_buffer_properties() };

    for (i, descriptor_offset) in self.descriptor_offsets.iter().enumerate() {
      let descriptor_info = descriptor_infos.get(i).expect("Not enough provided writes for all descriptors in a set!");

      use vk::DescriptorType as DT;
      let descriptor_type_size = match descriptor_info.ty {
        DT::UNIFORM_BUFFER => device_properties.uniform_buffer_descriptor_size,
        DT::COMBINED_IMAGE_SAMPLER => device_properties.combined_image_sampler_descriptor_size,
        _ => panic!("Unsuported descriptor type used in write!"),
      };

      let descriptor_offset = (self.buffer_offset + descriptor_offset) as usize;
      let descriptor_buffer_region = descriptor_buffer.data();
      let descriptor_buffer_region = descriptor_buffer_region[descriptor_offset..descriptor_offset + descriptor_type_size].as_mut();

      unsafe { self.device.get_descriptor(descriptor_info, descriptor_buffer_region) }
      trace!("Descriptor contents: {:?}", descriptor_buffer_region);
    }
  }

  fn get_descriptor_set_offset(&self) -> u64 {
    self.buffer_offset
  }
}
