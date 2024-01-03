pub(crate) mod allocator;
pub(crate) mod descriptors;
mod device;
pub(crate) mod elements;
pub(crate) mod rendering_context;
mod window;

use self::descriptors::{GlobalDescriptorSetLayout, MaterialDescriptorSetLayout};
use crate::utils::constants::*;
use crate::utils::tools::Result;
pub(crate) use allocator::Allocator;
pub(crate) use device::Device;
pub(crate) use window::{Window, WindowResources};

use ash::vk;
use glfw::{Glfw, WindowEvent};

use std::sync::mpsc::Receiver;
use std::sync::Arc;

pub(crate) struct Vulkan {
  glfw: Glfw,
  device: Arc<Device>,
  global_descriptor_set_layout: Arc<GlobalDescriptorSetLayout>,
  material_descriptor_set_layout: Arc<MaterialDescriptorSetLayout>,
}

impl Vulkan {
  pub(crate) fn init() -> Result<Self> {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS)?;
    let device: Arc<Device> = Arc::new(Device::new(&glfw)?);
    let global_descriptor_set_layout = Arc::new(GlobalDescriptorSetLayout::new(&device)?);
    let material_descriptor_set_layout = Arc::new(MaterialDescriptorSetLayout::new(&device)?);

    Ok(Self {
      glfw,
      device,
      global_descriptor_set_layout,
      material_descriptor_set_layout,
    })
  }

  pub(crate) fn get_device(&self) -> Arc<Device> {
    self.device.clone()
  }

  pub(crate) fn get_global_descriptor_set_layout(&self) -> Arc<GlobalDescriptorSetLayout> {
    self.global_descriptor_set_layout.clone()
  }

  pub(crate) fn get_material_descriptor_set_layout(&self) -> Arc<MaterialDescriptorSetLayout> {
    self.material_descriptor_set_layout.clone()
  }

  pub(crate) fn get_descriptor_set_layouts(&self) -> [vk::DescriptorSetLayout; DESCRIPTOR_SET_COUNT] {
    [**self.global_descriptor_set_layout, **self.material_descriptor_set_layout]
  }

  pub(crate) fn create_allocator(&self) -> Result<Allocator> {
    Allocator::new(self)
  }

  pub(crate) fn device_wait_idle(&self) {
    self.device.wait_idle()
  }

  pub(crate) fn poll_events(&mut self) {
    self.glfw.poll_events()
  }

  pub(crate) fn create_window(&mut self, resources: WindowResources) -> Result<(Window, Receiver<(f64, WindowEvent)>)> {
    self.glfw.window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
    self.glfw.window_hint(glfw::WindowHint::Resizable(true));
    let (window, events) = self.glfw.create_window(WINDOW_WIDTH, WINDOW_HEIGHT, "Virtual Circus", glfw::WindowMode::Windowed).unwrap();
    let window = Window::new(self, window, resources)?;

    Ok((window, events))
  }
}
