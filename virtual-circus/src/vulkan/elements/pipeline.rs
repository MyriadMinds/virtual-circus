use super::super::Device;
use crate::utils::constants::*;
use crate::utils::tools::Result;

use ash::vk;
use log::debug;

use std::ffi::CString;
use std::sync::Arc;

pub(crate) struct Pipeline {
  device: Arc<Device>,
  pipeline: vk::Pipeline,
}

impl Pipeline {
  pub(crate) fn new(device: &Arc<Device>, pipeline_layout: &vk::PipelineLayout, color_format: &vk::Format) -> Result<Self> {
    debug!("Creating graphics pipeline.");
    let vertex_shader = unsafe { read_shader("shaders/vertexShader.vert.spv", device)? };
    let fragment_shader = unsafe { read_shader("shaders/fragmentShader.frag.spv", device)? };

    let main_function_name = CString::new("main").unwrap();

    let vertex_shader_stage_info = vk::PipelineShaderStageCreateInfo {
      module: vertex_shader,
      stage: vk::ShaderStageFlags::VERTEX,
      p_name: main_function_name.as_ptr(),
      ..Default::default()
    };

    let fragment_shader_stage_info = vk::PipelineShaderStageCreateInfo {
      module: fragment_shader,
      stage: vk::ShaderStageFlags::FRAGMENT,
      p_name: main_function_name.as_ptr(),
      ..Default::default()
    };

    let shader_stages = [vertex_shader_stage_info, fragment_shader_stage_info];

    let input_assembly = vk::PipelineInputAssemblyStateCreateInfo {
      primitive_restart_enable: vk::FALSE,
      topology: vk::PrimitiveTopology::TRIANGLE_LIST,
      ..Default::default()
    };

    let dynamic_states = [
      vk::DynamicState::VIEWPORT,
      vk::DynamicState::SCISSOR,
      vk::DynamicState::PRIMITIVE_TOPOLOGY,
      vk::DynamicState::VERTEX_INPUT_EXT,
    ];
    let pipeline_dynamic_state = vk::PipelineDynamicStateCreateInfo {
      dynamic_state_count: dynamic_states.len() as u32,
      p_dynamic_states: dynamic_states.as_ptr(),
      ..Default::default()
    };

    let view_port_state = vk::PipelineViewportStateCreateInfo {
      viewport_count: 1,
      scissor_count: 1,
      ..Default::default()
    };

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo {
      depth_test_enable: vk::TRUE,
      depth_write_enable: vk::TRUE,
      depth_compare_op: vk::CompareOp::LESS,
      depth_bounds_test_enable: vk::FALSE,
      stencil_test_enable: vk::FALSE,
      ..Default::default()
    };

    let rasterizer = vk::PipelineRasterizationStateCreateInfo {
      depth_clamp_enable: vk::FALSE,
      depth_bias_enable: vk::FALSE,
      rasterizer_discard_enable: vk::FALSE,
      polygon_mode: vk::PolygonMode::FILL,
      line_width: 1.0,
      cull_mode: vk::CullModeFlags::NONE,
      front_face: vk::FrontFace::CLOCKWISE,
      ..Default::default()
    };

    let multisampling = vk::PipelineMultisampleStateCreateInfo {
      sample_shading_enable: vk::FALSE,
      rasterization_samples: vk::SampleCountFlags::TYPE_1,
      ..Default::default()
    };

    let color_blend_attachment = vk::PipelineColorBlendAttachmentState {
      blend_enable: vk::FALSE,
      color_write_mask: vk::ColorComponentFlags::RGBA,
      src_color_blend_factor: vk::BlendFactor::SRC_ALPHA,
      dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_SRC_ALPHA,
      color_blend_op: vk::BlendOp::ADD,
      src_alpha_blend_factor: vk::BlendFactor::ONE,
      dst_alpha_blend_factor: vk::BlendFactor::ZERO,
      alpha_blend_op: vk::BlendOp::ADD,
    };

    let color_blending = vk::PipelineColorBlendStateCreateInfo {
      logic_op_enable: vk::FALSE,
      logic_op: vk::LogicOp::COPY,
      p_attachments: &color_blend_attachment,
      attachment_count: 1,
      ..Default::default()
    };

    let mut rendering_info = vk::PipelineRenderingCreateInfo {
      color_attachment_count: 1,
      p_color_attachment_formats: color_format,
      depth_attachment_format: DEPTH_FORMAT,
      stencil_attachment_format: vk::Format::UNDEFINED,
      ..Default::default()
    };

    let pipeline_create_info = vk::GraphicsPipelineCreateInfo::builder()
      .flags(vk::PipelineCreateFlags::DESCRIPTOR_BUFFER_EXT)
      .depth_stencil_state(&depth_stencil_state)
      .dynamic_state(&pipeline_dynamic_state)
      .input_assembly_state(&input_assembly)
      .viewport_state(&view_port_state)
      .rasterization_state(&rasterizer)
      .multisample_state(&multisampling)
      .color_blend_state(&color_blending)
      .stages(&shader_stages)
      .layout(*pipeline_layout)
      .subpass(0)
      .push_next(&mut rendering_info);

    let pipeline = unsafe {
      match device.create_graphics_pipelines(vk::PipelineCache::null(), &[pipeline_create_info.build()], None) {
        Ok(pipelines) => Ok(pipelines[0]),
        Err((pipelines, err)) => err.result_with_success(pipelines[0]),
      }?
    };

    unsafe {
      device.destroy_shader_module(vertex_shader, None);
      device.destroy_shader_module(fragment_shader, None);
    }

    debug!("Successfully created graphics pipeline!");
    Ok(Self { device: device.clone(), pipeline })
  }

  #[allow(dead_code)]
  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }
}

unsafe fn read_shader(path: &str, device: &Device) -> Result<vk::ShaderModule> {
  debug!("Loading shader: {}", path);
  let mut exe = std::env::current_exe().map_err(|_| vk::Result::ERROR_INITIALIZATION_FAILED)?;
  exe.pop();
  let mut file = std::fs::File::open(exe.join(path)).map_err(|_| vk::Result::ERROR_INITIALIZATION_FAILED)?;
  let code = ash::util::read_spv(&mut file).map_err(|_| vk::Result::ERROR_INITIALIZATION_FAILED)?;

  let create_info = vk::ShaderModuleCreateInfo {
    code_size: code.len() * 4,
    p_code: code.as_ptr(),
    ..Default::default()
  };

  let shader = device.create_shader_module(&create_info, None)?;
  debug!("Successfully loaded shader!");
  Ok(shader)
}

impl Drop for Pipeline {
  fn drop(&mut self) {
    debug!("Destroying graphics pipeline.");
    unsafe { self.device.destroy_pipeline(self.pipeline, None) };
  }
}

impl std::ops::Deref for Pipeline {
  type Target = vk::Pipeline;

  fn deref(&self) -> &Self::Target {
    &self.pipeline
  }
}
