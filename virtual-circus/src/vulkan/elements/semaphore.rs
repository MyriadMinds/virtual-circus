use super::super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::debug;

use std::sync::Arc;

pub(crate) struct Semaphore {
  device: Arc<Device>,
  semaphore: vk::Semaphore,
}

impl Semaphore {
  pub(crate) fn new(device: &Arc<Device>) -> Result<Self> {
    debug!("Creating semaphore.");

    let create_info = vk::SemaphoreCreateInfo::default();
    let semaphore = unsafe { device.create_semaphore(&create_info, None)? };

    debug!("Semaphore successfully created!");
    Ok(Self { device: device.clone(), semaphore })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for Semaphore {
  fn drop(&mut self) {
    debug!("Destroying semaphores.");
    unsafe { self.device.destroy_semaphore(self.semaphore, None) };
  }
}

impl std::ops::Deref for Semaphore {
  type Target = vk::Semaphore;

  fn deref(&self) -> &Self::Target {
    &self.semaphore
  }
}
