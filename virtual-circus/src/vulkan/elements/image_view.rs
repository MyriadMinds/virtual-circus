use super::super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::trace;

use std::sync::Arc;

pub(crate) struct ImageView {
  device: Arc<Device>,
  image_view: vk::ImageView,
}

impl ImageView {
  pub(crate) fn new(device: &Arc<Device>, image: &vk::Image, format: &vk::Format, aspect_mask: vk::ImageAspectFlags) -> Result<Self> {
    let components = vk::ComponentMapping {
      r: vk::ComponentSwizzle::IDENTITY,
      g: vk::ComponentSwizzle::IDENTITY,
      b: vk::ComponentSwizzle::IDENTITY,
      a: vk::ComponentSwizzle::IDENTITY,
    };

    let subresource_range = vk::ImageSubresourceRange {
      aspect_mask,
      base_mip_level: 0,
      level_count: 1,
      base_array_layer: 0,
      layer_count: 1,
    };

    let create_info = vk::ImageViewCreateInfo {
      format: *format,
      components,
      subresource_range,
      image: *image,
      view_type: vk::ImageViewType::TYPE_2D,
      ..Default::default()
    };

    let image_view = unsafe { device.create_image_view(&create_info, None)? };
    trace!("Created image view: {:?}", image_view);

    Ok(Self { device: device.clone(), image_view })
  }

  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for ImageView {
  fn drop(&mut self) {
    trace!("Destroying image view: {:?}", self.image_view);
    unsafe { self.device.destroy_image_view(self.image_view, None) };
  }
}

impl std::ops::Deref for ImageView {
  type Target = vk::ImageView;

  fn deref(&self) -> &Self::Target {
    &self.image_view
  }
}
