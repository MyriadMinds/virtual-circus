mod instance;
mod vertex_input_dynamic_state;

use crate::utils::tools::{required_match_available, vk_to_string, Result};
use instance::Instance;

use ash::extensions::ext::DescriptorBuffer;
use ash::extensions::khr::{Surface, Swapchain};
use ash::prelude::VkResult;
use ash::vk::{self, Handle};
use glfw::Glfw;
use log::{debug, error, trace};
use vertex_input_dynamic_state::VertexInputDynamicState;

use std::ffi::CString;
use std::ops::Deref;

pub(crate) struct Device {
  instance: Instance,
  physical_device: vk::PhysicalDevice,
  device: ash::Device,
  transfer_queue_family_index: u32,
  graphics_queue_family_index: u32,
  surface_loader: Surface,
  swapchain_loader: Swapchain,
  vertex_input_dynamic_state: VertexInputDynamicState,
  descriptor_buffer: DescriptorBuffer,
}

//------------------------Setup----------------------------------

fn get_required_extensions() -> Vec<CString> {
  vec![
    ash::extensions::khr::Swapchain::name().to_owned(),
    ash::extensions::ext::DescriptorBuffer::name().to_owned(),
    CString::new("VK_EXT_vertex_input_dynamic_state").unwrap(),
    CString::new("VK_EXT_robustness2").unwrap(),
    CString::new("VK_EXT_index_type_uint8").unwrap(),
  ]
}

//------------------------Device----------------------------------

impl Device {
  pub(crate) fn new(glfw: &Glfw) -> Result<Self> {
    let instance = Instance::new(glfw)?;

    debug!("Creating a logical device.");
    // let graphics_queue = device.get_device_queue(graphics_queue_family_index, 0);s
    //When searching for physical devices we check whether it supports both queue types so just unwrap
    let physical_device = pick_physical_device(&instance, glfw)?;

    let graphics_queue_family_index = find_graphics_queue_family(&instance, physical_device).unwrap();
    let transfer_queue_family_index = find_transfer_queue_family(&instance, physical_device).unwrap();
    let queue_family_indices = [graphics_queue_family_index, transfer_queue_family_index];
    let queue_priorities = [1.0];

    let graphics_queue_ci = vk::DeviceQueueCreateInfo {
      queue_family_index: queue_family_indices[0],
      p_queue_priorities: queue_priorities.as_ptr(),
      queue_count: queue_priorities.len() as u32,
      ..Default::default()
    };

    let transfer_queue_ci = vk::DeviceQueueCreateInfo {
      queue_family_index: queue_family_indices[1],
      p_queue_priorities: queue_priorities.as_ptr(),
      queue_count: queue_priorities.len() as u32,
      ..Default::default()
    };

    let queue_infos = [graphics_queue_ci, transfer_queue_ci];

    // Extension compatibility is checked when the physical device is picked.
    let extensions = get_required_extensions();
    trace!("Requested device extensions: {:?}", extensions);
    let extensions: Vec<*const i8> = extensions.iter().map(|item| item.as_ptr()).collect();

    let vulkan_10_features = vk::PhysicalDeviceFeatures {
      sampler_anisotropy: vk::TRUE,
      ..Default::default()
    };

    let mut vulkan_11_features = vk::PhysicalDeviceVulkan11Features { ..Default::default() };

    let mut vulkan_12_features = vk::PhysicalDeviceVulkan12Features {
      buffer_device_address: vk::TRUE,
      shader_uniform_buffer_array_non_uniform_indexing: vk::TRUE,
      shader_sampled_image_array_non_uniform_indexing: vk::TRUE,
      descriptor_binding_uniform_buffer_update_after_bind: vk::TRUE,
      descriptor_binding_sampled_image_update_after_bind: vk::TRUE,
      ..Default::default()
    };

    let mut vulkan_13_features = vk::PhysicalDeviceVulkan13Features {
      dynamic_rendering: vk::TRUE,
      ..Default::default()
    };

    let mut vertex_input_dynamic_state_feature = vk::PhysicalDeviceVertexInputDynamicStateFeaturesEXT {
      vertex_input_dynamic_state: vk::TRUE,
      ..Default::default()
    };

    let mut robustness_features = vk::PhysicalDeviceRobustness2FeaturesEXT {
      null_descriptor: vk::TRUE,
      ..Default::default()
    };

    let mut index_type_features = vk::PhysicalDeviceIndexTypeUint8FeaturesEXT {
      index_type_uint8: vk::TRUE,
      ..Default::default()
    };

    let mut descriptor_buffer_features = vk::PhysicalDeviceDescriptorBufferFeaturesEXT {
      descriptor_buffer: vk::TRUE,
      ..Default::default()
    };

    let l_device_ci = vk::DeviceCreateInfo::builder()
      .queue_create_infos(&queue_infos)
      .enabled_extension_names(&extensions)
      .enabled_features(&vulkan_10_features)
      .push_next(&mut vulkan_11_features)
      .push_next(&mut vulkan_12_features)
      .push_next(&mut vulkan_13_features)
      .push_next(&mut vertex_input_dynamic_state_feature)
      .push_next(&mut robustness_features)
      .push_next(&mut index_type_features)
      .push_next(&mut descriptor_buffer_features);

    let device = unsafe { instance.create_device(physical_device, &l_device_ci, None)? };
    let surface_loader = instance.get_surface_loader();
    let swapchain_loader = Swapchain::new(&instance, &device);
    let vertex_input_dynamic_state = VertexInputDynamicState::new(&instance, &device);
    let descriptor_buffer = DescriptorBuffer::new(&instance, &device);
    debug!("Successfully created a logical device!");

    Ok(Self {
      instance,
      physical_device,
      device,
      transfer_queue_family_index,
      graphics_queue_family_index,
      surface_loader,
      swapchain_loader,
      vertex_input_dynamic_state,
      descriptor_buffer,
    })
  }

  pub(crate) fn wait_idle(&self) {
    unsafe {
      self.device_wait_idle().unwrap();
    }
  }

  pub(crate) fn instance(&self) -> &Instance {
    &self.instance
  }

  pub(crate) fn physical_device(&self) -> vk::PhysicalDevice {
    self.physical_device
  }

  pub(crate) fn graphics_queue(&self) -> vk::Queue {
    unsafe { self.device.get_device_queue(self.graphics_queue_family_index, 0) }
  }

  pub(crate) fn transfer_queue(&self) -> vk::Queue {
    unsafe { self.device.get_device_queue(self.transfer_queue_family_index, 0) }
  }

  pub(crate) fn transfer_queue_family_index(&self) -> u32 {
    self.transfer_queue_family_index
  }

  pub(crate) fn graphics_queue_family_index(&self) -> u32 {
    self.graphics_queue_family_index
  }

  // Delegates
  pub(crate) unsafe fn get_physical_device_properties(&self) -> vk::PhysicalDeviceProperties {
    self.instance.get_physical_device_properties(self.physical_device)
  }

  #[allow(dead_code)]
  pub(crate) unsafe fn get_physical_device_properties2(&self) -> vk::PhysicalDeviceProperties2 {
    let mut properties = vk::PhysicalDeviceProperties2::default();
    self.instance.get_physical_device_properties2(self.physical_device, &mut properties);
    properties
  }

  pub(crate) unsafe fn get_physical_device_descriptor_buffer_properties(&self) -> vk::PhysicalDeviceDescriptorBufferPropertiesEXT {
    let mut descriptor_buffer_properties = vk::PhysicalDeviceDescriptorBufferPropertiesEXT::default();
    let properties = vk::PhysicalDeviceProperties2::builder();
    let mut properties = properties.push_next(&mut descriptor_buffer_properties);
    self.instance.get_physical_device_properties2(self.physical_device, &mut properties);
    descriptor_buffer_properties
  }

  pub(crate) unsafe fn get_physical_device_surface_capabilities(&self, surface: vk::SurfaceKHR) -> VkResult<vk::SurfaceCapabilitiesKHR> {
    self.surface_loader.get_physical_device_surface_capabilities(self.physical_device, surface)
  }

  pub(crate) unsafe fn get_physical_device_surface_formats(&self, surface: vk::SurfaceKHR) -> VkResult<Vec<vk::SurfaceFormatKHR>> {
    self.surface_loader.get_physical_device_surface_formats(self.physical_device, surface)
  }

  pub(crate) unsafe fn get_physical_device_surface_present_modes(&self, surface: vk::SurfaceKHR) -> VkResult<Vec<vk::PresentModeKHR>> {
    self.surface_loader.get_physical_device_surface_present_modes(self.physical_device, surface)
  }

  pub(crate) unsafe fn destroy_surface(&self, surface: vk::SurfaceKHR, allocation_callbacks: Option<&vk::AllocationCallbacks>) {
    self.surface_loader.destroy_surface(surface, allocation_callbacks)
  }

  pub(crate) unsafe fn create_swapchain(&self, create_info: &vk::SwapchainCreateInfoKHR, allocation_callbacks: Option<&vk::AllocationCallbacks>) -> VkResult<vk::SwapchainKHR> {
    self.swapchain_loader.create_swapchain(create_info, allocation_callbacks)
  }

  pub(crate) unsafe fn get_swapchain_images(&self, swapchain: vk::SwapchainKHR) -> VkResult<Vec<vk::Image>> {
    self.swapchain_loader.get_swapchain_images(swapchain)
  }

  pub(crate) unsafe fn destroy_swapchain(&self, swapchain: vk::SwapchainKHR, allocation_callbacks: Option<&vk::AllocationCallbacks>) {
    self.swapchain_loader.destroy_swapchain(swapchain, allocation_callbacks)
  }

  pub(crate) unsafe fn acquire_next_image(&self, swapchain: vk::SwapchainKHR, timeout: u64, semaphore: vk::Semaphore, fence: vk::Fence) -> VkResult<(u32, bool)> {
    self.swapchain_loader.acquire_next_image(swapchain, timeout, semaphore, fence)
  }

  pub(crate) unsafe fn queue_present(&self, queue: vk::Queue, present_info: &vk::PresentInfoKHR) -> VkResult<bool> {
    self.swapchain_loader.queue_present(queue, present_info)
  }

  pub(crate) unsafe fn cmd_set_vertex_input(
    &self,
    command_buffer: vk::CommandBuffer,
    vertex_binding_descriptions: &[vk::VertexInputBindingDescription2EXT],
    vertex_attribute_descriptions: &[vk::VertexInputAttributeDescription2EXT],
  ) {
    self
      .vertex_input_dynamic_state
      .cmd_set_vertex_input(command_buffer, vertex_binding_descriptions, vertex_attribute_descriptions)
  }

  pub(crate) unsafe fn get_descriptor_set_layout_size(&self, layout: vk::DescriptorSetLayout) -> vk::DeviceSize {
    self.descriptor_buffer.get_descriptor_set_layout_size(layout)
  }

  pub(crate) unsafe fn get_descriptor_set_layout_binding_offset(&self, layout: vk::DescriptorSetLayout, binding: u32) -> vk::DeviceSize {
    self.descriptor_buffer.get_descriptor_set_layout_binding_offset(layout, binding)
  }

  pub(crate) unsafe fn cmd_bind_descriptor_buffers(&self, command_buffer: vk::CommandBuffer, binding_info: &[vk::DescriptorBufferBindingInfoEXT]) {
    self.descriptor_buffer.cmd_bind_descriptor_buffers(command_buffer, binding_info)
  }

  pub(crate) unsafe fn cmd_set_descriptor_buffer_offsets(
    &self,
    command_buffer: vk::CommandBuffer,
    pipeline_bind_point: vk::PipelineBindPoint,
    layout: vk::PipelineLayout,
    first_set: u32,
    buffer_indices: &[u32],
    offsets: &[vk::DeviceSize],
  ) {
    self
      .descriptor_buffer
      .cmd_set_descriptor_buffer_offsets(command_buffer, pipeline_bind_point, layout, first_set, buffer_indices, offsets)
  }

  pub(crate) unsafe fn get_descriptor(&self, descriptor_info: &vk::DescriptorGetInfoEXT, descriptor: &mut [u8]) {
    self.descriptor_buffer.get_descriptor(descriptor_info, descriptor)
  }
}

impl Drop for Device {
  fn drop(&mut self) {
    unsafe {
      debug!("Destroying device.");
      self.device.destroy_device(None);
    }
  }
}

impl Deref for Device {
  type Target = ash::Device;

  fn deref(&self) -> &Self::Target {
    &self.device
  }
}

//------------------------Helpers-------------------------------

fn pick_physical_device(instance: &Instance, glfw: &Glfw) -> Result<vk::PhysicalDevice> {
  debug!("Picking physical device.");
  let physical_devices = unsafe { instance.enumerate_physical_devices()? };

  let device = physical_devices.into_iter().find(|device| device_is_suitable(instance, glfw, *device)).ok_or_else(|| {
    error!("Couldn't find suitable physical device!");
    vk::Result::ERROR_INITIALIZATION_FAILED
  })?;
  debug!("Found suitable physical device!");

  Ok(device)
}

fn device_is_suitable(instance: &Instance, glfw: &Glfw, device: vk::PhysicalDevice) -> bool {
  // TODO: check whether the buffer_device_address feature is present
  let device_properties = unsafe { instance.get_physical_device_properties(device) };
  let device_extensions = unsafe { instance.enumerate_device_extension_properties(device).expect("Could not get device extension properties!") };
  trace!("Testing device: {:?}", vk_to_string(&device_properties.device_name));

  let device_extensions: Vec<CString> = device_extensions.iter().map(|extension| vk_to_string(&extension.extension_name).to_owned()).collect();

  let required_extensions = get_required_extensions();

  trace!("Checking if device has all the required extensions...");
  trace!("Device extensions: {:?}", device_extensions);
  if !required_match_available(&required_extensions, &device_extensions) {
    return false;
  }

  trace!("Checking if device is a discrete GPU...");
  if device_properties.device_type != vk::PhysicalDeviceType::DISCRETE_GPU {
    return false;
  };

  trace!("Checking if device supports a dedicated transfer queue...");
  if find_transfer_queue_family(instance, device).is_none() {
    return false;
  }

  trace!("Checking if device supports a graphics queue...");
  let graphics_queue = find_graphics_queue_family(instance, device);
  if graphics_queue.is_none() {
    return false;
  }

  trace!("Checking if graphics queue also supports presentation...");
  if !glfw.get_physical_device_presentation_support_raw(instance.handle().as_raw() as usize, device.as_raw() as usize, graphics_queue.unwrap()) {
    return false;
  }

  true
}

pub(crate) fn find_graphics_queue_family(instance: &Instance, device: vk::PhysicalDevice) -> Option<u32> {
  let families = unsafe { instance.get_physical_device_queue_family_properties(device) };

  for (index, queue_family) in families.into_iter().enumerate() {
    if queue_family.queue_count > 0 && queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
      return Some(index as u32);
    }
  }

  None
}

pub(crate) fn find_transfer_queue_family(instance: &Instance, device: vk::PhysicalDevice) -> Option<u32> {
  let families = unsafe { instance.get_physical_device_queue_family_properties(device) };

  for (index, queue_family) in families.into_iter().enumerate() {
    if queue_family.queue_count > 0 && queue_family.queue_flags.contains(vk::QueueFlags::TRANSFER) && !queue_family.queue_flags.contains(vk::QueueFlags::GRAPHICS) {
      return Some(index as u32);
    }
  }

  None
}
