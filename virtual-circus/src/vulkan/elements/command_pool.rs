use super::super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::debug;

use std::ops::Index;
use std::sync::Arc;

pub(crate) struct CommandPool {
  device: Arc<Device>,
  command_pool: vk::CommandPool,
  command_buffers: Vec<vk::CommandBuffer>,
}

impl CommandPool {
  pub(crate) fn new(device: &Arc<Device>, queue_family_index: u32, count: u32) -> Result<Self> {
    debug!("Creating command pool.");
    let create_info = vk::CommandPoolCreateInfo {
      queue_family_index,
      flags: vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER,
      ..Default::default()
    };

    let command_pool = unsafe { device.create_command_pool(&create_info, None)? };

    let command_buffers_create_info = vk::CommandBufferAllocateInfo {
      command_pool,
      level: vk::CommandBufferLevel::PRIMARY,
      command_buffer_count: count,
      ..Default::default()
    };

    let command_buffers = unsafe { device.allocate_command_buffers(&command_buffers_create_info)? };
    debug!("Successfully created command pool!");
    Ok(Self {
      device: device.clone(),
      command_pool,
      command_buffers,
    })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for CommandPool {
  fn drop(&mut self) {
    debug!("Destroying command pool.");
    unsafe { self.device.destroy_command_pool(self.command_pool, None) };
  }
}

impl std::ops::Deref for CommandPool {
  type Target = vk::CommandPool;

  fn deref(&self) -> &Self::Target {
    &self.command_pool
  }
}

impl Index<usize> for CommandPool {
  type Output = vk::CommandBuffer;

  fn index(&self, index: usize) -> &Self::Output {
    &self.command_buffers[index]
  }
}
