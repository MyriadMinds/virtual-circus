use super::super::rendering_context::PushConstant;
use super::super::Device;
use crate::utils::tools::Result;

use ash::vk;
use log::debug;

use std::sync::Arc;

pub(crate) struct PipelineLayout {
  device: Arc<Device>,
  layout: vk::PipelineLayout,
}

impl PipelineLayout {
  pub(crate) fn new(device: &Arc<Device>, descriptor_sets: &[vk::DescriptorSetLayout]) -> Result<Self> {
    debug!("Creating pipeline layout.");
    let range = vk::PushConstantRange {
      offset: 0,
      size: std::mem::size_of::<PushConstant>() as u32,
      stage_flags: vk::ShaderStageFlags::VERTEX,
    };
    let push_constants = [range];

    let pipeline_layout = vk::PipelineLayoutCreateInfo {
      set_layout_count: descriptor_sets.len() as u32,
      p_set_layouts: descriptor_sets.as_ptr(),
      push_constant_range_count: push_constants.len() as u32,
      p_push_constant_ranges: push_constants.as_ptr(),
      ..Default::default()
    };

    let layout = unsafe { device.create_pipeline_layout(&pipeline_layout, None)? };
    debug!("Successfully created pipeline layout!");
    Ok(Self { device: device.clone(), layout })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

impl Drop for PipelineLayout {
  fn drop(&mut self) {
    debug!("Destroying pipeline layout.");
    unsafe { self.device.destroy_pipeline_layout(self.layout, None) };
  }
}

impl std::ops::Deref for PipelineLayout {
  type Target = vk::PipelineLayout;

  fn deref(&self) -> &Self::Target {
    &self.layout
  }
}
