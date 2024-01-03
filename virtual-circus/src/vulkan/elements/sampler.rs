use super::super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::debug;

use std::sync::Arc;

pub(crate) struct Sampler {
  device: Arc<Device>,
  sampler: vk::Sampler,
}

impl Sampler {
  pub(crate) fn new(
    device: &Arc<Device>,
    mag_filter: vk::Filter,
    min_filter: vk::Filter,
    mipmap_mode: vk::SamplerMipmapMode,
    address_mode_u: vk::SamplerAddressMode,
    address_mode_v: vk::SamplerAddressMode,
  ) -> Result<Self> {
    debug!("Creating sampler.");
    let create_info = vk::SamplerCreateInfo {
      mag_filter,
      min_filter,
      mipmap_mode,
      address_mode_u,
      address_mode_v,
      address_mode_w: vk::SamplerAddressMode::CLAMP_TO_EDGE,
      anisotropy_enable: vk::TRUE,
      max_anisotropy: unsafe { device.get_physical_device_properties().limits.max_sampler_anisotropy },
      compare_enable: vk::FALSE,
      compare_op: vk::CompareOp::ALWAYS,
      mip_lod_bias: 0.0,
      min_lod: 0.0,
      max_lod: 0.0,
      border_color: vk::BorderColor::FLOAT_OPAQUE_BLACK,
      unnormalized_coordinates: vk::FALSE,
      ..Default::default()
    };

    let sampler = unsafe { device.create_sampler(&create_info, None)? };
    debug!("Successfully created sampler!");

    Ok(Self { device: device.clone(), sampler })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for Sampler {
  fn drop(&mut self) {
    unsafe { self.device.destroy_sampler(self.sampler, None) };
  }
}

impl std::ops::Deref for Sampler {
  type Target = vk::Sampler;

  fn deref(&self) -> &Self::Target {
    &self.sampler
  }
}
