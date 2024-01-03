mod buffer;
mod image;

use super::elements::{CommandPool, Fence};
use super::{Device, Vulkan};
use crate::utils::tools::{EngineError, Result};
pub(crate) use buffer::Buffer;
pub(crate) use image::{Image, ImagePurpose};

use ash::vk;
use gpu_allocator::vulkan::{Allocation, AllocationCreateDesc};
use gpu_allocator::{vulkan, MemoryLocation};

use log::{debug, error};
use std::mem::ManuallyDrop;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::Arc;

pub(crate) enum BufferType {
  CpuVisible,
  GpuOnly,
}

//-----------------------------------Allocators-----------------------------------------------
pub(crate) struct Allocator {
  device: Arc<Device>,
  allocator: vulkan::Allocator,
  command_pool: CommandPool,
  staging_buffers: Vec<Buffer>,
  transfer_fence: Fence,
  allocation_sender: ManuallyDrop<Sender<Allocation>>,
  allocation_receiver: Receiver<Allocation>,
}

//TODO: Consecutive command buffers to avoid re-using the same one while it's still being processed
impl Allocator {
  pub(crate) fn new(vulkan: &Vulkan) -> Result<Self> {
    debug!("Creating allocator.");
    let device = vulkan.device.clone();
    let instance_handle = &**device.instance();
    let device_handle = &**device;

    // Create the allocator
    let allocator_create_info = vulkan::AllocatorCreateDesc {
      instance: instance_handle.clone(),
      physical_device: device.physical_device(),
      device: device_handle.clone(),
      debug_settings: Default::default(),
      buffer_device_address: true,
      allocation_sizes: Default::default(),
    };

    let allocator = vulkan::Allocator::new(&allocator_create_info)?;
    let command_pool = CommandPool::new(&device, device.transfer_queue_family_index(), 1)?;
    let transfer_fence = Fence::new(&device, vk::FenceCreateFlags::empty())?;
    let (allocation_sender, allocation_receiver) = std::sync::mpsc::channel();

    let allocator = Self {
      device,
      allocator,
      command_pool,
      staging_buffers: Vec::new(),
      transfer_fence,
      allocation_sender: ManuallyDrop::new(allocation_sender),
      allocation_receiver,
    };
    debug!("Successfully created allocator!");

    // Prepare the command buffer for accepting loading operations
    allocator.begin_recording()?;

    Ok(allocator)
  }

  fn get_command_buffer(&self) -> &vk::CommandBuffer {
    &self.command_pool[0]
  }

  fn begin_recording(&self) -> Result<()> {
    let command_buffer = self.get_command_buffer();
    let begin_info = vk::CommandBufferBeginInfo {
      flags: vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT,
      ..Default::default()
    };
    unsafe { self.device.begin_command_buffer(*command_buffer, &begin_info)? };
    Ok(())
  }

  pub(crate) fn flush(&mut self) {
    match self.process_commands() {
      Ok(_) => (),
      Err(e) => panic!("Failed to process memory transfer commands: {:?}", e),
    }
  }

  fn process_commands(&mut self) -> Result<()> {
    let command_buffer = self.get_command_buffer();
    let transfer_queue = self.device.transfer_queue();

    let submit_info = vk::SubmitInfo {
      command_buffer_count: 1,
      p_command_buffers: command_buffer,
      ..Default::default()
    };

    unsafe {
      self.device.end_command_buffer(*command_buffer)?;
      self.device.queue_submit(transfer_queue, &[submit_info], *self.transfer_fence)?;
      self.device.wait_for_fences(&[*self.transfer_fence], true, u64::MAX)?;
      self.device.reset_fences(&[*self.transfer_fence])?;
      self.device.reset_command_buffer(*command_buffer, vk::CommandBufferResetFlags::empty())?;
      self.clear_staging_buffers();
      self.begin_recording()?;
    };
    Ok(())
  }

  fn clear_staging_buffers(&mut self) {
    self.staging_buffers.drain(..);
  }

  pub(crate) fn process_deallocations(&mut self) -> std::result::Result<(), TryRecvError> {
    loop {
      self.free_allocation(self.allocation_receiver.try_recv()?);
    }
  }

  pub(crate) fn cleanup(&mut self) {
    // Keep handling deallocations until all channel producers have dropped their senders, meaning all buffers and images should now be cleaned up.
    unsafe { ManuallyDrop::drop(&mut self.allocation_sender) };

    loop {
      if let Err(TryRecvError::Disconnected) = self.process_deallocations() {
        break;
      }
    }
  }

  pub(crate) fn create_buffer(&mut self, size: u64, usage: vk::BufferUsageFlags, buffer_type: BufferType) -> Result<Buffer> {
    match buffer_type {
      BufferType::CpuVisible => Buffer::new(self, size, usage, MemoryLocation::CpuToGpu),
      BufferType::GpuOnly => Buffer::new(self, size, usage, MemoryLocation::GpuOnly),
    }
  }

  pub(crate) fn create_buffer_from_data(&mut self, data: &[u8], usage: vk::BufferUsageFlags, buffer_type: BufferType) -> Result<Buffer> {
    let size = data.len() as u64;

    match buffer_type {
      BufferType::CpuVisible => {
        let mut buffer = Buffer::new(self, size, usage, MemoryLocation::CpuToGpu)?;
        buffer.load_data(data)?;
        Ok(buffer)
      }
      BufferType::GpuOnly => {
        let mut staging_buffer = Buffer::new(self, size, vk::BufferUsageFlags::TRANSFER_SRC, MemoryLocation::CpuToGpu)?;
        let final_buffer = Buffer::new(self, size, usage | vk::BufferUsageFlags::TRANSFER_DST, MemoryLocation::GpuOnly)?;

        let command_buffer = self.get_command_buffer();
        staging_buffer.load_data(data)?;
        staging_buffer.copy_buffer_to_buffer(command_buffer, &final_buffer, size);

        self.staging_buffers.push(staging_buffer);

        Ok(final_buffer)
      }
    }
  }

  pub(crate) fn create_image(&mut self, data: &[u8], image_info: vk::ImageCreateInfo, purpose: ImagePurpose) -> Result<Image> {
    let transfer_image_info = vk::ImageCreateInfo {
      initial_layout: vk::ImageLayout::UNDEFINED,
      usage: image_info.usage | vk::ImageUsageFlags::TRANSFER_DST,
      ..image_info
    };

    let mut final_image = Image::new(self, transfer_image_info, purpose.aspect_mask())?;
    final_image.prepare_image_for_transfer(self.get_command_buffer(), purpose.aspect_mask());

    // Make sure we're dealing with an image type that should be filled with data.
    use ImagePurpose as IP;
    match purpose {
      IP::Texture => self.fill_image(data, &final_image, image_info.extent)?,
      IP::DepthBuffer => (),
    };

    final_image.transition_image(self.get_command_buffer(), purpose);

    Ok(final_image)
  }

  fn fill_image(&mut self, data: &[u8], image: &Image, extent: vk::Extent3D) -> Result<()> {
    let size = data.len() as u64;
    if size == 0 {
      return Err(EngineError::CreationError("requested image type with contents but provided no data to fill the image"));
    }

    let mut staging_buffer = Buffer::new(self, size, vk::BufferUsageFlags::TRANSFER_SRC, MemoryLocation::CpuToGpu)?;
    staging_buffer.load_data(data)?;
    staging_buffer.copy_buffer_to_image(self.get_command_buffer(), image, extent);
    self.staging_buffers.push(staging_buffer);

    Ok(())
  }

  fn allocate(&mut self, allocation_info: &AllocationCreateDesc) -> Result<Allocation> {
    Ok(self.allocator.allocate(allocation_info)?)
  }

  pub(self) fn clone_allocation_sender(&self) -> Sender<Allocation> {
    ManuallyDrop::into_inner(self.allocation_sender.clone())
  }

  pub(self) fn free_allocation(&mut self, allocation: Allocation) {
    match self.allocator.free(allocation) {
      Ok(_) => (),
      Err(e) => error!("Error freeing buffer memory: {}", e.to_string()),
    };
  }
}
