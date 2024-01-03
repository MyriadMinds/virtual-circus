use super::super::elements::ImageView;
use super::Allocator;
use super::Device;
use crate::utils::tools::{EngineError, Result};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc, AllocationScheme};
use gpu_allocator::MemoryLocation;
use log::error;

use std::mem::ManuallyDrop;
use std::sync::mpsc::Sender;
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub(crate) enum ImagePurpose {
  Texture,
  DepthBuffer,
}

impl ImagePurpose {
  pub(super) fn aspect_mask(&self) -> vk::ImageAspectFlags {
    match self {
      ImagePurpose::Texture => vk::ImageAspectFlags::COLOR,
      ImagePurpose::DepthBuffer => vk::ImageAspectFlags::DEPTH,
    }
  }
}

pub(crate) struct Image {
  device: Arc<Device>,
  allocation_release_channel: Sender<Allocation>,
  image: vk::Image,
  allocation: ManuallyDrop<Allocation>,
  format: vk::Format,
  aspect_mask: vk::ImageAspectFlags,
}

impl Image {
  pub(super) fn new(allocator: &mut Allocator, image_info: vk::ImageCreateInfo, aspect_mask: vk::ImageAspectFlags) -> Result<Self> {
    unsafe {
      let image = allocator.device.create_image(&image_info, None)?;

      let requirements = allocator.device.get_image_memory_requirements(image);
      let allocate_info = AllocationCreateDesc {
        name: "Image",
        requirements,
        location: MemoryLocation::GpuOnly,
        linear: true,
        allocation_scheme: AllocationScheme::GpuAllocatorManaged,
      };

      let allocation = match allocator.allocate(&allocate_info) {
        Ok(allocation) => allocation,
        Err(e) => {
          allocator.device.destroy_image(image, None);
          return Err(e);
        }
      };

      match allocator.device.bind_image_memory(image, allocation.memory(), allocation.offset()) {
        Ok(_) => (),
        Err(e) => {
          allocator.device.destroy_image(image, None);
          allocator.free_allocation(allocation);
          return Err(EngineError::VulkanError(e));
        }
      };

      Ok(Self {
        device: allocator.device.clone(),
        allocation_release_channel: allocator.clone_allocation_sender(),
        image,
        allocation: ManuallyDrop::new(allocation),
        format: image_info.format,
        aspect_mask,
      })
    }
  }

  pub(crate) fn make_image_view(&self) -> Result<ImageView> {
    ImageView::new(&self.device, &self.image, &self.format, self.aspect_mask)
  }

  pub(super) fn prepare_image_for_transfer(&mut self, command_buffer: &vk::CommandBuffer, aspect_mask: vk::ImageAspectFlags) {
    let image_barrier = vk::ImageMemoryBarrier {
      src_access_mask: vk::AccessFlags::NONE,
      dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
      old_layout: vk::ImageLayout::UNDEFINED,
      new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
      src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      image: self.image,
      subresource_range: vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
      },
      ..Default::default()
    };

    unsafe {
      self.device.cmd_pipeline_barrier(
        *command_buffer,
        vk::PipelineStageFlags::TOP_OF_PIPE,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[image_barrier],
      );
    }
  }

  pub(super) fn transition_image(&mut self, command_buffer: &vk::CommandBuffer, purpose: ImagePurpose) {
    let new_layout;
    let aspect_mask;

    match purpose {
      ImagePurpose::Texture => {
        new_layout = vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL;
        aspect_mask = vk::ImageAspectFlags::COLOR;
      }
      ImagePurpose::DepthBuffer => {
        new_layout = vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL;
        aspect_mask = vk::ImageAspectFlags::DEPTH;
      }
    }

    let image_barrier = vk::ImageMemoryBarrier {
      src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
      dst_access_mask: vk::AccessFlags::NONE,
      old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
      new_layout,
      src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      image: self.image,
      subresource_range: vk::ImageSubresourceRange {
        aspect_mask,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
      },
      ..Default::default()
    };

    unsafe {
      self.device.cmd_pipeline_barrier(
        *command_buffer,
        vk::PipelineStageFlags::TRANSFER,
        vk::PipelineStageFlags::TRANSFER,
        vk::DependencyFlags::empty(),
        &[],
        &[],
        &[image_barrier],
      );
    }
  }
}

impl Drop for Image {
  fn drop(&mut self) {
    unsafe {
      match self.allocation_release_channel.send(ManuallyDrop::take(&mut self.allocation)) {
        Ok(_) => (),
        Err(_) => error!("Error freeing buffer memory: couldn't return allocation to allocator because the channel is closed!"),
      };
      self.device.destroy_image(self.image, None);
    };
  }
}

impl std::ops::Deref for Image {
  type Target = vk::Image;

  fn deref(&self) -> &Self::Target {
    &self.image
  }
}
