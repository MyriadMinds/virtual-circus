use super::allocator::Image;
use super::descriptors::{GlobalDescriptorSetInfo, GlobalDescriptorSets};
use super::elements::{CommandPool, Fence, ImageView, Pipeline, PipelineLayout, Semaphore, Surface, Swapchain};
use super::rendering_context::RenderingContext;
use super::{Device, Vulkan};
use crate::utils::constants::*;
use crate::utils::tools::{EngineError, Result};

use ash::vk;
use log::{debug, trace};
use nalgebra_glm as glm;

use std::sync::Arc;

pub(crate) struct Window {
  device: Arc<Device>,
  glfw_window: glfw::Window,
  swapchain: Swapchain,
  swapchain_images: Vec<vk::Image>,
  // swapchain_image_views: Vec<ImageView>,
  surface: Surface,
  _depth_images: Vec<Image>,
  _color_images: Vec<Image>,
  depth_image_views: Vec<ImageView>,
  color_image_views: Vec<ImageView>,
  graphics_pipeline_layout: PipelineLayout,
  graphics_pipeline: Pipeline,
  command_pool: CommandPool,
  image_available_semaphores: Vec<Semaphore>,
  render_complete_semaphores: Vec<Semaphore>,
  frame_fences: Vec<Fence>,
  frame_index: usize,
  time: std::time::SystemTime,
  global_descriptor_sets: GlobalDescriptorSets,
}

impl Window {
  pub(crate) fn new(vulkan: &Vulkan, glfw_window: glfw::Window, mut resources: WindowResources) -> Result<Self> {
    debug!("Beginning creation of window elements.");

    let device = vulkan.get_device();
    let surface = Surface::new(&glfw_window, &device)?;

    let window_framebuffer = FramebufferSize::from(glfw_window.get_framebuffer_size());
    let swapchain = Swapchain::new(&device, &surface, window_framebuffer)?;

    let swapchain_images = unsafe { device.get_swapchain_images(*swapchain)? };
    // let swapchain_image_views = create_swapchain_image_views(&device, &swapchain_images, &swapchain.format)?;

    let depth_image_views = create_depth_image_views(&device, &resources.depth_images)?;
    let color_image_views = create_color_image_views(&device, &resources.color_images)?;

    let graphics_pipeline_layout = PipelineLayout::new(&device, &vulkan.get_descriptor_set_layouts())?;

    let graphics_pipeline = Pipeline::new(&device, &graphics_pipeline_layout)?;

    let command_pool = CommandPool::new(&device, device.graphics_queue_family_index(), MAX_FRAMES_IN_FLIGHT)?;

    let image_available_semaphores = create_semaphores(&device, MAX_FRAMES_IN_FLIGHT as usize)?;
    let render_complete_semaphores = create_semaphores(&device, MAX_FRAMES_IN_FLIGHT as usize)?;
    let frame_fences = create_fences(&device, MAX_FRAMES_IN_FLIGHT as usize)?;

    resources.global_descriptor_sets[0].update_descriptor(create_global_descriptor_set_info(&swapchain.extent))?;

    debug!("All window elements succesfully created!");

    Ok(Self {
      device: device.clone(),
      glfw_window,
      swapchain,
      swapchain_images,
      surface,
      _depth_images: resources.depth_images,
      _color_images: resources.color_images,
      depth_image_views,
      color_image_views,
      graphics_pipeline_layout,
      graphics_pipeline,
      command_pool,
      image_available_semaphores,
      render_complete_semaphores,
      frame_fences,
      global_descriptor_sets: resources.global_descriptor_sets,
      frame_index: 0,
      time: std::time::SystemTime::now(),
    })
  }

  pub(crate) fn get_rendering_context(&self) -> Result<RenderingContext> {
    let device = &self.device;
    let fence = &self.frame_fences[self.frame_index];
    unsafe { device.wait_for_fences(&[**fence], true, u64::MAX) }?;

    let command_buffer = self.command_pool[self.frame_index];

    unsafe {
      device.reset_fences(&[**fence])?;
      device.reset_command_buffer(command_buffer, vk::CommandBufferResetFlags::empty())?;
    };

    let begin_info = vk::CommandBufferBeginInfo::default();

    let render_area = vk::Rect2D {
      offset: vk::Offset2D { x: 0, y: 0 },
      extent: self.swapchain.extent,
    };

    let clear_color_value = vk::ClearColorValue { float32: [0.2, 0.0, 0.9, 1.0] };
    let color_clear = vk::ClearValue { color: clear_color_value };

    let clear_depth_stencil_value = vk::ClearDepthStencilValue { depth: 1.0, stencil: 0 };
    let depth_clear = vk::ClearValue {
      depth_stencil: clear_depth_stencil_value,
    };

    let color_attachment = [vk::RenderingAttachmentInfo {
      image_view: *self.color_image_views[self.frame_index],
      image_layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
      load_op: vk::AttachmentLoadOp::CLEAR,
      store_op: vk::AttachmentStoreOp::STORE,
      clear_value: color_clear,
      ..Default::default()
    }];

    let depth_attachment = [vk::RenderingAttachmentInfo {
      image_view: *self.depth_image_views[self.frame_index],
      image_layout: vk::ImageLayout::DEPTH_ATTACHMENT_OPTIMAL,
      load_op: vk::AttachmentLoadOp::CLEAR,
      store_op: vk::AttachmentStoreOp::DONT_CARE,
      clear_value: depth_clear,
      ..Default::default()
    }];

    let rendering_info = vk::RenderingInfo {
      render_area,
      layer_count: 1,
      color_attachment_count: 1,
      p_color_attachments: color_attachment.as_ptr(),
      p_depth_attachment: depth_attachment.as_ptr(),
      ..Default::default()
    };

    let viewport = vk::Viewport {
      x: 0.0,
      y: 0.0,
      height: self.swapchain.extent.height as f32,
      width: self.swapchain.extent.width as f32,
      max_depth: 1.0,
      min_depth: 0.0,
    };

    let scissor = vk::Rect2D {
      offset: vk::Offset2D { x: 0, y: 0 },
      extent: self.swapchain.extent,
    };

    let time = std::time::SystemTime::now().duration_since(self.time).unwrap().as_millis() as f32;
    let mut rendering_context = RenderingContext::new(device, &self.command_pool[self.frame_index], &self.graphics_pipeline_layout, time);

    unsafe {
      device.begin_command_buffer(command_buffer, &begin_info)?;
      device.cmd_begin_rendering(command_buffer, &rendering_info);
      device.cmd_bind_pipeline(command_buffer, vk::PipelineBindPoint::GRAPHICS, *self.graphics_pipeline);
      device.cmd_set_viewport(command_buffer, 0, &[viewport]);
      device.cmd_set_scissor(command_buffer, 0, &[scissor]);
    }

    rendering_context.bind_descriptor_buffer(&self.global_descriptor_sets);
    rendering_context.set_descriptor_set(&self.global_descriptor_sets[0]);

    Ok(rendering_context)
  }

  fn transition_color_image(&self, command_buffer: &vk::CommandBuffer, image: &vk::Image, stage: RenderingStage) {
    let old_layout;
    let new_layout;
    let src_stage_mask;
    let dst_stage_mask;
    let src_access_mask;
    let dst_access_mask;

    match stage {
      RenderingStage::BeforeCopy => {
        old_layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
        new_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        src_stage_mask = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        dst_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
        src_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
        dst_access_mask = vk::AccessFlags::NONE;
      }
      RenderingStage::AfterCopy => {
        old_layout = vk::ImageLayout::TRANSFER_SRC_OPTIMAL;
        new_layout = vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL;
        src_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
        dst_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
        src_access_mask = vk::AccessFlags::NONE;
        dst_access_mask = vk::AccessFlags::NONE;
      }
    }

    let image_barrier = vk::ImageMemoryBarrier {
      src_access_mask,
      dst_access_mask,
      old_layout,
      new_layout,
      src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      image: *image,
      subresource_range: vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
      },
      ..Default::default()
    };

    unsafe {
      self
        .device
        .cmd_pipeline_barrier(*command_buffer, src_stage_mask, dst_stage_mask, vk::DependencyFlags::empty(), &[], &[], &[image_barrier]);
    }
  }

  fn transition_swapchain_image(&self, command_buffer: &vk::CommandBuffer, image: &vk::Image, stage: RenderingStage) {
    let old_layout;
    let new_layout;
    let src_stage_mask;
    let dst_stage_mask;
    let src_access_mask;
    let dst_access_mask;

    match stage {
      RenderingStage::BeforeCopy => {
        old_layout = vk::ImageLayout::UNDEFINED;
        new_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        src_stage_mask = vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT;
        dst_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
        src_access_mask = vk::AccessFlags::COLOR_ATTACHMENT_WRITE;
        dst_access_mask = vk::AccessFlags::NONE;
      }
      RenderingStage::AfterCopy => {
        old_layout = vk::ImageLayout::TRANSFER_DST_OPTIMAL;
        new_layout = vk::ImageLayout::PRESENT_SRC_KHR;
        src_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
        dst_stage_mask = vk::PipelineStageFlags::BOTTOM_OF_PIPE;
        src_access_mask = vk::AccessFlags::NONE;
        dst_access_mask = vk::AccessFlags::NONE;
      }
    }

    let image_barrier = vk::ImageMemoryBarrier {
      src_access_mask,
      dst_access_mask,
      old_layout,
      new_layout,
      src_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      dst_queue_family_index: vk::QUEUE_FAMILY_IGNORED,
      image: *image,
      subresource_range: vk::ImageSubresourceRange {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        base_mip_level: 0,
        level_count: 1,
        base_array_layer: 0,
        layer_count: 1,
      },
      ..Default::default()
    };

    unsafe {
      self
        .device
        .cmd_pipeline_barrier(*command_buffer, src_stage_mask, dst_stage_mask, vk::DependencyFlags::empty(), &[], &[], &[image_barrier]);
    }
  }

  pub(crate) fn draw_frame(&self, mut rendering_context: RenderingContext) -> Result<()> {
    unsafe {
      trace!("Drawing frame: {}", self.frame_index);
      let device = &self.device;
      let graphics_queue = &self.device.graphics_queue();
      let image_available = &self.image_available_semaphores[self.frame_index];
      let render_complete = &self.render_complete_semaphores[self.frame_index];
      let fence = &self.frame_fences[self.frame_index];

      let (image_index, recreate_swapchain) = device.acquire_next_image(*self.swapchain, u64::MAX, **image_available, vk::Fence::null())?;

      let swapchain_image = &self.swapchain_images[image_index as usize];
      let color_image = &self._color_images[self.frame_index];
      rendering_context.complete_rendering_command();

      self.transition_swapchain_image(rendering_context.command_buffer(), swapchain_image, RenderingStage::BeforeCopy);
      self.transition_color_image(rendering_context.command_buffer(), &color_image, RenderingStage::BeforeCopy);

      let swapchain_extent = self.swapchain.extent;
      let layers = vk::ImageSubresourceLayers {
        aspect_mask: vk::ImageAspectFlags::COLOR,
        mip_level: 0,
        base_array_layer: 0,
        layer_count: 1,
      };

      let offsets = [
        vk::Offset3D { x: 0, y: 0, z: 0 },
        vk::Offset3D {
          x: swapchain_extent.width as i32,
          y: swapchain_extent.height as i32,
          z: 1,
        },
      ];

      let regions = vk::ImageBlit {
        src_subresource: layers,
        src_offsets: offsets,
        dst_subresource: layers,
        dst_offsets: offsets,
      };

      // copy the content of color attachment to swapchain image
      device.cmd_blit_image(
        *rendering_context.command_buffer(),
        **color_image,
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
        *swapchain_image,
        vk::ImageLayout::TRANSFER_DST_OPTIMAL,
        &[regions],
        vk::Filter::NEAREST,
      );

      self.transition_swapchain_image(rendering_context.command_buffer(), swapchain_image, RenderingStage::AfterCopy);
      self.transition_color_image(rendering_context.command_buffer(), &color_image, RenderingStage::AfterCopy);

      rendering_context.end_command_buffer()?;

      let submit_info = vk::SubmitInfo {
        command_buffer_count: 1,
        p_command_buffers: rendering_context.command_buffer(),
        signal_semaphore_count: 1,
        p_signal_semaphores: &**render_complete,
        wait_semaphore_count: 1,
        p_wait_semaphores: &**image_available,
        p_wait_dst_stage_mask: &vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
        ..Default::default()
      };

      device.queue_submit(*graphics_queue, &[submit_info], **fence)?;

      let present_info = vk::PresentInfoKHR {
        wait_semaphore_count: 1,
        p_wait_semaphores: &**render_complete,
        swapchain_count: 1,
        p_swapchains: &*self.swapchain,
        p_image_indices: &image_index,
        p_results: std::ptr::null_mut(),
        ..Default::default()
      };

      device.queue_present(*graphics_queue, &present_info)?;

      if recreate_swapchain {
        return Err(EngineError::OldSwapchain);
      }

      Ok(())
    }
  }

  pub(crate) fn progress_frame(&mut self) {
    self.frame_index = (self.frame_index + 1) % MAX_FRAMES_IN_FLIGHT as usize;
  }

  pub(crate) fn recreate_swapchain(&mut self) -> Result<()> {
    debug!("Recreating swapchain!");
    self.device.wait_idle();

    // create new swapchain related elements
    let window_framebuffer = FramebufferSize::from(self.glfw_window.get_framebuffer_size());
    let swapchain = Swapchain::new(&self.device, &self.surface, window_framebuffer)?;

    let swapchain_images = unsafe { self.device.get_swapchain_images(*swapchain)? };
    // let swapchain_image_views = create_swapchain_image_views(&self.device, &swapchain_images, &swapchain.format)?;

    // put the new elements into the renderer
    self.global_descriptor_sets[0].update_descriptor(create_global_descriptor_set_info(&swapchain.extent))?;
    self.swapchain = swapchain;
    self.swapchain_images = swapchain_images;
    // self.swapchain_image_views = swapchain_image_views;

    debug!("Successfuly recreated swapchain!");
    Ok(())
  }

  pub(crate) fn should_close(&self) -> bool {
    self.glfw_window.should_close()
  }
}

//-----------------------------------Helpers----------------------------------------------

enum RenderingStage {
  BeforeCopy,
  AfterCopy,
}

pub(crate) struct FramebufferSize(pub(crate) i32, pub(crate) i32);
impl From<(i32, i32)> for FramebufferSize {
  fn from(input: (i32, i32)) -> Self {
    let (width, height) = input;
    Self(width, height)
  }
}

pub(crate) struct WindowResources {
  pub(crate) depth_images: Vec<Image>,
  pub(crate) color_images: Vec<Image>,
  pub(crate) global_descriptor_sets: GlobalDescriptorSets,
}

// fn create_swapchain_image_views(device: &Arc<Device>, images: &Vec<vk::Image>, format: &vk::Format) -> Result<Vec<ImageView>> {
//   debug!("Creating swapchain image views.");
//   let mut image_views: Vec<ImageView> = Vec::with_capacity(images.len());

//   for image in images {
//     let image_view = ImageView::new(device, image, format, vk::ImageAspectFlags::COLOR)?;
//     image_views.push(image_view);
//   }

//   debug!("Successfully created swapchain image views!");
//   Ok(image_views)
// }

fn create_depth_image_views(device: &Arc<Device>, images: &Vec<Image>) -> Result<Vec<ImageView>> {
  debug!("Creating depth image views.");
  let mut image_views: Vec<ImageView> = Vec::with_capacity(images.len());

  for image in images {
    let image_view = ImageView::new(device, image, &DEPTH_FORMAT, vk::ImageAspectFlags::DEPTH)?;
    image_views.push(image_view);
  }

  debug!("Successfully created depth image views!");
  Ok(image_views)
}

fn create_color_image_views(device: &Arc<Device>, images: &Vec<Image>) -> Result<Vec<ImageView>> {
  debug!("Creating depth image views.");
  let mut image_views: Vec<ImageView> = Vec::with_capacity(images.len());

  for image in images {
    let image_view = ImageView::new(device, image, &vk::Format::R8G8B8A8_SRGB, vk::ImageAspectFlags::COLOR)?;
    image_views.push(image_view);
  }

  debug!("Successfully created depth image views!");
  Ok(image_views)
}

fn create_semaphores(device: &Arc<Device>, count: usize) -> Result<Vec<Semaphore>> {
  debug!("Creating {} semaphores.", count);
  let mut semaphores: Vec<Semaphore> = Vec::with_capacity(count);

  for _ in 0..count {
    let semaphore = Semaphore::new(device)?;
    semaphores.push(semaphore);
  }

  Ok(semaphores)
}

fn create_fences(device: &Arc<Device>, count: usize) -> Result<Vec<Fence>> {
  debug!("Creating {} fences.", count);
  let mut fences: Vec<Fence> = Vec::with_capacity(count);

  for _ in 0..count {
    let fence = Fence::new(device, vk::FenceCreateFlags::SIGNALED)?;
    fences.push(fence);
  }

  Ok(fences)
}

fn create_global_descriptor_set_info(swapchain_extent: &vk::Extent2D) -> GlobalDescriptorSetInfo {
  let camera_pos = glm::Vec3::new(1.0, 1.0, 1.5);
  let center_pos = glm::Vec3::new(-2.0, -2.0, 0.0);
  let up_direction = glm::Vec3::new(0.0, 0.0, -1.0);
  let view = glm::look_at(&camera_pos, &center_pos, &up_direction);

  let fov_y_radians = 80.0 * std::f32::consts::PI / 180.0;
  let aspect_ratio = swapchain_extent.width as f32 / swapchain_extent.height as f32;
  let z_near = 0.1;
  let z_far = 10.0;
  let projection = glm::perspective(aspect_ratio, fov_y_radians, z_near, z_far);

  GlobalDescriptorSetInfo {
    view,
    projection,
    model: glm::rotate_z(&glm::Mat4::identity(), 2.0),
  }
}
