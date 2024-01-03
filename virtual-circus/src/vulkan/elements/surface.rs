use super::super::Device;
use crate::utils::tools::Result;

use ash::vk::{self, Handle};
use glfw::Window;
use log::debug;

use std::sync::Arc;

pub(crate) struct Surface {
  device: Arc<Device>,
  surface: vk::SurfaceKHR,
}

impl Surface {
  pub(crate) fn new(window: &Window, device: &Arc<Device>) -> Result<Self> {
    debug!("Creating surface.");
    let instance_handle = device.instance().handle().as_raw() as usize;
    let mut surface: u64 = 0;
    let result = window.create_window_surface(instance_handle, std::ptr::null(), &mut surface) as i32;
    let result = ash::vk::Result::from_raw(result);
    let surface = result.result_with_success(vk::SurfaceKHR::from_raw(surface))?;
    debug!("Successfully create surface!");

    Ok(Self { device: device.clone(), surface })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for Surface {
  fn drop(&mut self) {
    debug!("Destroying surface.");
    unsafe { self.device.destroy_surface(self.surface, None) };
  }
}

impl std::ops::Deref for Surface {
  type Target = vk::SurfaceKHR;

  fn deref(&self) -> &Self::Target {
    &self.surface
  }
}
