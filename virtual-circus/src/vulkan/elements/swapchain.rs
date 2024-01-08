use super::super::window::FramebufferSize;
use super::super::Device;
use crate::utils::constants::*;
use crate::utils::tools::Result;

use ash::vk;
use ash::vk::SurfaceKHR;
use log::{debug, trace};

use std::sync::Arc;

pub(crate) struct Swapchain {
  device: Arc<Device>,
  swapchain: vk::SwapchainKHR,
  pub(crate) extent: vk::Extent2D,
  pub(crate) format: vk::Format,
}

impl Swapchain {
  pub(crate) fn new(device: &Arc<Device>, surface: &SurfaceKHR, window_framebuffer: FramebufferSize) -> Result<Self> {
    debug!("Creating swapchain.");
    let capabilities = unsafe { device.get_physical_device_surface_capabilities(*surface)? };
    let formats = unsafe { device.get_physical_device_surface_formats(*surface)? };
    let present_modes = unsafe { device.get_physical_device_surface_present_modes(*surface)? };

    let min_image_count = get_optimal_image_count(&capabilities);
    trace!("Swpachain image count: {}", min_image_count);
    let pre_transform = capabilities.current_transform;
    trace!("Swpachain transform: {:?}", pre_transform);
    let image_extent = get_optimal_extent(&capabilities, window_framebuffer);
    trace!("Swapchain extent: {:?}", image_extent);
    let present_mode = get_optimal_present_mode(&present_modes);
    trace!("Swapchain presentation mode: {:?}", present_mode);
    let format = get_optimal_format(&formats);
    trace!("Swapchain format: {:?}", format);

    let create_info = vk::SwapchainCreateInfoKHR {
      min_image_count,
      pre_transform,
      image_extent,
      present_mode,
      surface: *surface,
      image_format: format.format,
      image_color_space: format.color_space,
      image_array_layers: 1,
      image_sharing_mode: ash::vk::SharingMode::EXCLUSIVE,
      image_usage: ash::vk::ImageUsageFlags::TRANSFER_DST,
      composite_alpha: ash::vk::CompositeAlphaFlagsKHR::OPAQUE,
      clipped: ash::vk::TRUE,
      ..Default::default()
    };

    let swapchain = unsafe { device.create_swapchain(&create_info, None)? };
    debug!("Successfully created swapchain!");
    Ok(Self {
      device: device.clone(),
      swapchain,
      extent: image_extent,
      format: format.format,
    })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for Swapchain {
  fn drop(&mut self) {
    debug!("Destroying swapchain.");
    unsafe { self.device.destroy_swapchain(self.swapchain, None) };
  }
}

impl std::ops::Deref for Swapchain {
  type Target = vk::SwapchainKHR;

  fn deref(&self) -> &Self::Target {
    &self.swapchain
  }
}

//------------------------Helpers-------------------------------

fn get_optimal_image_count(capabilities: &vk::SurfaceCapabilitiesKHR) -> u32 {
  let mut count = capabilities.min_image_count;
  if count < DESIRED_SWAPCHAIN_IMAGES {
    count = DESIRED_SWAPCHAIN_IMAGES;
  }
  if capabilities.max_image_count > 0 && count > capabilities.max_image_count {
    count = capabilities.max_image_count;
  }
  count
}

fn get_optimal_format(formats: &[vk::SurfaceFormatKHR]) -> vk::SurfaceFormatKHR {
  let optimal_format = vk::SurfaceFormatKHR {
    format: vk::Format::B8G8R8A8_SRGB,
    color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
  };

  if formats.contains(&optimal_format) {
    optimal_format
  } else {
    *formats.first().unwrap()
  }
}

fn get_optimal_extent(capabilities: &vk::SurfaceCapabilitiesKHR, window_framebuffer: FramebufferSize) -> vk::Extent2D {
  if capabilities.current_extent.width != u32::max_value() {
    return capabilities.current_extent;
  }

  let (width, height) = (window_framebuffer.0, window_framebuffer.1);
  let mut width = width as u32;
  let mut height = height as u32;
  let min_extent = capabilities.min_image_extent;
  let max_extent = capabilities.max_image_extent;

  width = width.clamp(min_extent.width, max_extent.width);
  height = height.clamp(min_extent.height, max_extent.height);

  vk::Extent2D { width, height }
}

fn get_optimal_present_mode(present_modes: &[vk::PresentModeKHR]) -> vk::PresentModeKHR {
  if present_modes.contains(&ash::vk::PresentModeKHR::MAILBOX) {
    ash::vk::PresentModeKHR::MAILBOX
  } else {
    ash::vk::PresentModeKHR::FIFO
  }
}
