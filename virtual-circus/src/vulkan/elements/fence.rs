use super::super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::debug;

use std::sync::Arc;

pub(crate) struct Fence {
  device: Arc<Device>,
  fence: vk::Fence,
}

impl Fence {
  pub(crate) fn new(device: &Arc<Device>, flags: vk::FenceCreateFlags) -> Result<Self> {
    debug!("Creating fence");

    let create_info = vk::FenceCreateInfo { flags, ..Default::default() };
    let fence = unsafe { device.create_fence(&create_info, None)? };

    debug!("Fence successfully created!");
    Ok(Self { device: device.clone(), fence })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for Fence {
  fn drop(&mut self) {
    debug!("Destroying fence.");
    unsafe { self.device.destroy_fence(self.fence, None) };
  }
}

impl std::ops::Deref for Fence {
  type Target = vk::Fence;

  fn deref(&self) -> &Self::Target {
    &self.fence
  }
}
