use super::Allocator;
use super::Device;
use super::Image;
use crate::utils::tools::{EngineError, Result};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;
use log::{error, trace};

use std::mem::ManuallyDrop;
use std::sync::mpsc::Sender;
use std::sync::Arc;

pub(crate) struct Buffer {
  device: Arc<Device>,
  allocation_release_channel: Sender<Allocation>,
  buffer: vk::Buffer,
  allocation: ManuallyDrop<Allocation>,
}

impl Buffer {
  pub(crate) fn data(&mut self) -> &mut [u8] {
    self.allocation.mapped_slice_mut().unwrap()
  }

  #[allow(dead_code)]
  pub(crate) fn offset(&self) -> u64 {
    self.allocation.offset()
  }

  pub(crate) fn size(&self) -> u64 {
    self.allocation.size()
  }

  pub(crate) fn device_address(&self) -> u64 {
    let info = vk::BufferDeviceAddressInfo {
      buffer: self.buffer,
      ..Default::default()
    };
    unsafe { self.device.get_buffer_device_address(&info) }
  }

  pub(super) fn new(allocator: &mut Allocator, size: u64, usage: vk::BufferUsageFlags, location: MemoryLocation) -> Result<Self> {
    unsafe {
      let device = allocator.device.clone();

      // prepare the vulkan buffer
      let buffer_create_info = vk::BufferCreateInfo {
        usage,
        size,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
      };
      let buffer = device.create_buffer(&buffer_create_info, None)?;

      // prepare the memory allocation
      let requirements = device.get_buffer_memory_requirements(buffer);
      let allocate_info = AllocationCreateDesc {
        name: "Buffer",
        requirements,
        location,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
      };

      let allocation = match allocator.allocate(&allocate_info) {
        Ok(allocation) => allocation,
        Err(e) => {
          device.destroy_buffer(buffer, None);
          return Err(e);
        }
      };

      // bind the vulkan buffer and memory allocation together
      match allocator.device.bind_buffer_memory(buffer, allocation.memory(), allocation.offset()) {
        Ok(_) => (),
        Err(e) => {
          device.destroy_buffer(buffer, None);
          allocator.free_allocation(allocation);
          return Err(EngineError::VulkanError(e));
        }
      };

      // construct the final buffer object
      Ok(Self {
        allocation_release_channel: allocator.clone_allocation_sender(),
        device,
        buffer,
        allocation: ManuallyDrop::new(allocation),
      })
    }
  }

  pub(crate) fn load_data(&mut self, data: &[u8]) -> Result<()> {
    trace!("Mapping: {:?}", self.allocation);
    let memory = match self.allocation.mapped_slice_mut() {
      Some(memory) => memory,
      None => return Err(EngineError::CreationError("failed to map the memory of the buffer")),
    };

    if data.len() > memory.len() {
      return Err(EngineError::CreationError("attempted to write more data than the buffer can handle"));
    }

    memory[..data.len()].clone_from_slice(data);
    Ok(())
  }

  pub(super) fn copy_buffer_to_buffer(&mut self, command_buffer: &vk::CommandBuffer, dst_buffer: &Buffer, size: u64) {
    let copy_command = vk::BufferCopy { size, ..Default::default() };
    unsafe { self.device.cmd_copy_buffer(*command_buffer, self.buffer, dst_buffer.buffer, &[copy_command]) };
  }

  pub(super) fn copy_buffer_to_image(&mut self, command_buffer: &vk::CommandBuffer, dst_image: &Image, extent: vk::Extent3D) {
    let copy_command = vk::BufferImageCopy {
      buffer_offset: 0,
      buffer_image_height: 0,
      buffer_row_length: 0,
      image_extent: extent,
      image_offset: vk::Offset3D { x: 0, y: 0, z: 0 },
      image_subresource: vk::ImageSubresourceLayers {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_array_layer: 0,
        layer_count: 1,
        mip_level: 0,
      },
    };

    unsafe {
      self
        .device
        .cmd_copy_buffer_to_image(*command_buffer, self.buffer, **dst_image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &[copy_command]);
    }
  }
}

impl Drop for Buffer {
  fn drop(&mut self) {
    unsafe {
      match self.allocation_release_channel.send(ManuallyDrop::take(&mut self.allocation)) {
        Ok(_) => (),
        Err(_) => error!("Error freeing buffer memory: couldn't return allocation to allocator because the channel is closed!"),
      };
      self.device.destroy_buffer(self.buffer, None);
    };
  }
}

impl std::ops::Deref for Buffer {
  type Target = vk::Buffer;

  fn deref(&self) -> &Self::Target {
    &self.buffer
  }
}
