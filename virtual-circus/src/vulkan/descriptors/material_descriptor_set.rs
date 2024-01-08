use super::super::allocator::{Buffer, BufferType};
use super::super::elements::{ImageView, Sampler};
use super::super::{Allocator, Device};
use super::{DescriptorSet, DescriptorSetImpl, DescriptorSetLayoutImpl, DescriptorSets};
use crate::utils::constants::*;
use crate::utils::tools::Result;

use ash::vk;
use bitmask_enum::bitmask;
use nalgebra_glm::*;
use serde::Serialize;

use std::ops::Index;
use std::sync::Arc;

#[bitmask(u32)]
#[derive(Serialize, Default)]
pub(crate) enum MaterialFlags {
  AlphaModeOpaque = 0b00000001,
  AlphaModeMask = 0b00000010,
  AlphaModeBlend = 0b00000011,
  DoubleSided = 0b00000100,
  HasMetallicRougnessTexture = 0b00001000,
  HasNormalTexture = 0b00010000,
  HasOcclusionTexture = 0b00100000,
  HasEmmisiveTexture = 0b01000000,
}

#[derive(Serialize, Default)]
#[repr(C)]
pub(crate) struct MaterialInfo {
  pub(crate) base_color_factor: Vec4,
  pub(crate) emissive_factor: Vec3,
  pub(crate) metallic_roughness_factor: Vec2,
  pub(crate) normals_scale_factor: f32,
  pub(crate) occlusion_strength_factor: f32,
  pub(crate) alpha_cutoff: f32,
  pub(crate) material_flags: MaterialFlags,
}

pub(crate) struct TextureInfo<'a> {
  pub(crate) image_view: &'a ImageView,
  pub(crate) sampler: &'a Sampler,
}
pub(crate) struct MaterialDescriptorSetInfo<'a> {
  pub(crate) material_info: MaterialInfo,
  pub(crate) texture: TextureInfo<'a>,
  pub(crate) metallic_roughness_texture: TextureInfo<'a>,
  pub(crate) normal_texture: TextureInfo<'a>,
  pub(crate) occlusion_texture: TextureInfo<'a>,
  pub(crate) emissive_texture: TextureInfo<'a>,
}

pub(crate) struct MaterialDescriptorSetLayout {
  _device: Arc<Device>,
  descriptor_set_layout: DescriptorSetLayoutImpl,
}

impl MaterialDescriptorSetLayout {
  pub(crate) fn new(device: &Arc<Device>) -> Result<Self> {
    let bindings = [
      vk::DescriptorSetLayoutBinding {
        binding: 0,
        descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
      },
      vk::DescriptorSetLayoutBinding {
        binding: 1,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
      },
      vk::DescriptorSetLayoutBinding {
        binding: 2,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
      },
      vk::DescriptorSetLayoutBinding {
        binding: 3,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
      },
      vk::DescriptorSetLayoutBinding {
        binding: 4,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
      },
      vk::DescriptorSetLayoutBinding {
        binding: 5,
        descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
        descriptor_count: 1,
        stage_flags: vk::ShaderStageFlags::FRAGMENT,
        p_immutable_samplers: std::ptr::null(),
      },
    ];

    let descriptor_set_layout = DescriptorSetLayoutImpl::new(device, &bindings)?;
    Ok(Self {
      _device: device.clone(),
      descriptor_set_layout,
    })
  }

  pub(crate) fn create_descriptor_sets(&self, allocator: &mut Allocator, descriptor_infos: &[MaterialDescriptorSetInfo]) -> Result<MaterialDescriptorSets> {
    let (descriptor_buffer, descriptor_sets) = self.descriptor_set_layout.create_descriptor_sets(allocator, descriptor_infos.len())?;
    MaterialDescriptorSets::new(allocator, descriptor_buffer, descriptor_sets, descriptor_infos)
  }
}

impl std::ops::Deref for MaterialDescriptorSetLayout {
  type Target = vk::DescriptorSetLayout;

  fn deref(&self) -> &Self::Target {
    &self.descriptor_set_layout
  }
}

//---------------------------------Descriptor Sets-------------------------------------------------

pub(crate) struct MaterialDescriptorSets {
  descriptor_buffer: Buffer,
  descriptor_sets: Vec<MaterialDescriptorSet>,
}

impl MaterialDescriptorSets {
  fn new(allocator: &mut Allocator, mut descriptor_buffer: Buffer, mut descriptor_set_impls: Vec<DescriptorSetImpl>, descriptor_infos: &[MaterialDescriptorSetInfo]) -> Result<Self> {
    let mut descriptor_sets = Vec::with_capacity(descriptor_set_impls.len());
    for descriptor_info in descriptor_infos {
      let descriptor_set_impl = descriptor_set_impls.pop().unwrap();
      descriptor_sets.push(MaterialDescriptorSet::new(allocator, &mut descriptor_buffer, descriptor_set_impl, descriptor_info)?);
    }

    Ok(Self { descriptor_buffer, descriptor_sets })
  }
}

impl DescriptorSets for MaterialDescriptorSets {
  fn get_descriptor_buffer_info(&self) -> (vk::DescriptorBufferBindingInfoEXT, usize) {
    let binding_info = vk::DescriptorBufferBindingInfoEXT {
      address: self.descriptor_buffer.device_address(),
      usage: vk::BufferUsageFlags::RESOURCE_DESCRIPTOR_BUFFER_EXT | vk::BufferUsageFlags::SAMPLER_DESCRIPTOR_BUFFER_EXT,
      ..Default::default()
    };

    (binding_info, MATERIAL_DESCRIPTOR_BINDING)
  }
}

impl Index<usize> for MaterialDescriptorSets {
  type Output = MaterialDescriptorSet;

  fn index(&self, index: usize) -> &Self::Output {
    &self.descriptor_sets[index]
  }
}

//---------------------------------Descriptor Set--------------------------------------------------
pub(crate) struct MaterialDescriptorSet {
  descriptor_set: DescriptorSetImpl,
}

impl MaterialDescriptorSet {
  fn new(allocator: &mut Allocator, descriptor_buffer: &mut Buffer, descriptor_set: DescriptorSetImpl, descriptor_info: &MaterialDescriptorSetInfo) -> Result<Self> {
    // Prepare the buffer with extra data
    let data = bincode::serialize(&descriptor_info.material_info).unwrap();
    let usage = vk::BufferUsageFlags::UNIFORM_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS;
    let buffer = allocator.create_buffer_from_data(&data, usage, BufferType::GpuOnly)?;

    // Get the write data for the buffer
    let buffer_data = vk::DescriptorAddressInfoEXT {
      address: buffer.device_address(),
      range: data.len() as u64,
      format: vk::Format::UNDEFINED,
      ..Default::default()
    };

    let buffer_get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::UNIFORM_BUFFER,
      data: vk::DescriptorDataEXT {
        p_uniform_buffer: [buffer_data].as_ptr(),
      },
      ..Default::default()
    };

    // Get the write data for the texture
    let texture_info = vk::DescriptorImageInfo {
      image_view: **descriptor_info.texture.image_view,
      sampler: **descriptor_info.texture.sampler,
      image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };

    let texture_get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
      data: vk::DescriptorDataEXT {
        p_combined_image_sampler: [texture_info].as_ptr(),
      },
      ..Default::default()
    };

    // Get the write data for the material
    let material_image_info = vk::DescriptorImageInfo {
      image_view: **descriptor_info.metallic_roughness_texture.image_view,
      sampler: **descriptor_info.metallic_roughness_texture.sampler,
      image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };

    let material_get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
      data: vk::DescriptorDataEXT {
        p_combined_image_sampler: [material_image_info].as_ptr(),
      },
      ..Default::default()
    };

    // Get the write data for the normal map
    let normal_info = vk::DescriptorImageInfo {
      image_view: **descriptor_info.normal_texture.image_view,
      sampler: **descriptor_info.normal_texture.sampler,
      image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };

    let normal_get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
      data: vk::DescriptorDataEXT {
        p_combined_image_sampler: [normal_info].as_ptr(),
      },
      ..Default::default()
    };

    // Get the write data for the occlusion texture
    let occlusion_info = vk::DescriptorImageInfo {
      image_view: **descriptor_info.occlusion_texture.image_view,
      sampler: **descriptor_info.occlusion_texture.sampler,
      image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };

    let occlusion_get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
      data: vk::DescriptorDataEXT {
        p_combined_image_sampler: [occlusion_info].as_ptr(),
      },
      ..Default::default()
    };

    // Get the write data for the occlusion texture
    let emissive_info = vk::DescriptorImageInfo {
      image_view: **descriptor_info.emissive_texture.image_view,
      sampler: **descriptor_info.emissive_texture.sampler,
      image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
    };

    let emissive_get_info = vk::DescriptorGetInfoEXT {
      ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
      data: vk::DescriptorDataEXT {
        p_combined_image_sampler: [emissive_info].as_ptr(),
      },
      ..Default::default()
    };

    let descriptor_infos = vec![buffer_get_info, texture_get_info, material_get_info, normal_get_info, occlusion_get_info, emissive_get_info];
    descriptor_set.write_descriptor(&descriptor_infos, descriptor_buffer);

    Ok(Self { descriptor_set })
  }
}

impl DescriptorSet for MaterialDescriptorSet {
  fn get_descriptor_set_info(&self) -> (u64, usize) {
    (self.descriptor_set.buffer_offset, MATERIAL_DESCRIPTOR_BINDING)
  }
}
