use crate::utils::constants::*;
use crate::utils::tools::{required_match_available, vk_to_string, EngineError, Result};

use ash::extensions::ext::DebugUtils;
use ash::extensions::khr::Surface;
use ash::{vk, Entry};
use glfw::Glfw;
use log::{debug, error, info, trace, warn};

use std::ffi::{CStr, CString};
use std::ops::Deref;
use std::os::raw::c_void;
use std::ptr;

pub(crate) struct Instance {
  entry: Entry,
  instance: ash::Instance,
  #[cfg(debug_assertions)]
  debug_utils_loader: DebugUtils,
  #[cfg(debug_assertions)]
  debug_messenger: vk::DebugUtilsMessengerEXT,
}

//---------------------------Setup---------------------------
fn get_required_extensions() -> Vec<CString> {
  vec![
    #[cfg(debug_assertions)]
    ash::extensions::ext::DebugUtils::name().to_owned(),
  ]
}

fn get_required_layers() -> Vec<CString> {
  vec![
    #[cfg(debug_assertions)]
    CString::new("VK_LAYER_KHRONOS_validation").unwrap(),
  ]
}

//-------------------------debug messenger stuff------------------------------

unsafe extern "system" fn vulkan_debug_utils_callback(
  message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
  message_type: vk::DebugUtilsMessageTypeFlagsEXT,
  p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
  _p_user_data: *mut c_void,
) -> vk::Bool32 {
  let types = match message_type {
    vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "[General]",
    vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "[Performance]",
    vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "[Validation]",
    _ => "[Unknown]",
  };
  let message = CStr::from_ptr((*p_callback_data).p_message);

  match message_severity {
    vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => debug!("[Vulkan]{}{:?}", types, message),
    vk::DebugUtilsMessageSeverityFlagsEXT::INFO => info!("[Vulkan]{}{:?}", types, message),
    vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => warn!("[Vulkan]{}{:?}", types, message),
    vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => error!("[Vulkan]{}{:?}", types, message),
    _ => warn!("[Vulkan] Received log message with severity: {:?}", message_severity),
  };

  vk::FALSE
}

unsafe fn create_debug(loader: &DebugUtils) -> Result<vk::DebugUtilsMessengerEXT> {
  debug!("Creating debug messenger.");
  let messenger_ci = vk::DebugUtilsMessengerCreateInfoEXT {
    s_type: vk::StructureType::DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT,
    p_next: ptr::null(),
    flags: vk::DebugUtilsMessengerCreateFlagsEXT::empty(),
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
      | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
      | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
      | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
    // | vk::DebugUtilsMessageTypeFlagsEXT::GENERAL,
    pfn_user_callback: Some(vulkan_debug_utils_callback),
    p_user_data: ptr::null_mut(),
  };

  let messenger = loader.create_debug_utils_messenger(&messenger_ci, None)?;
  debug!("Successfully created debug messenger!");
  Ok(messenger)
}

//---------------------------Instance------------------------

impl Instance {
  pub(super) fn new(glfw: &Glfw) -> Result<Self> {
    let entry = unsafe { Entry::load()? };

    debug!("Creating instance.");
    let app_info = vk::ApplicationInfo {
      p_application_name: APP_NAME.as_ptr(),
      application_version: APP_VERSION,
      p_engine_name: ENGINE_NAME.as_ptr(),
      engine_version: ENGINE_VERSION,
      api_version: API_VERSION,
      ..Default::default()
    };

    let extensions = get_extensions(&entry, glfw)?;
    trace!("Requested instance extensions: {:?}", extensions);
    let extensions: Vec<*const i8> = extensions.iter().map(|item| item.as_ptr()).collect();
    let layers = get_layers(&entry)?;
    trace!("Requested instance layers: {:?}", layers);
    let layers: Vec<*const i8> = layers.iter().map(|item| item.as_ptr()).collect();

    let create_info = vk::InstanceCreateInfo {
      p_application_info: &app_info,
      pp_enabled_extension_names: extensions.as_ptr(),
      enabled_extension_count: extensions.len() as u32,
      pp_enabled_layer_names: layers.as_ptr(),
      enabled_layer_count: layers.len() as u32,
      ..Default::default()
    };

    let instance = unsafe { entry.create_instance(&create_info, None)? };

    #[cfg(debug_assertions)]
    let debug_utils_loader = DebugUtils::new(&entry, &instance);
    #[cfg(debug_assertions)]
    let debug_messenger = unsafe { create_debug(&debug_utils_loader)? };
    debug!("Successfully created instance!");

    Ok(Self {
      entry,
      instance,
      #[cfg(debug_assertions)]
      debug_utils_loader,
      #[cfg(debug_assertions)]
      debug_messenger,
    })
  }

  pub(super) fn get_surface_loader(&self) -> Surface {
    Surface::new(&self.entry, &self.instance)
  }
}

impl Drop for Instance {
  fn drop(&mut self) {
    unsafe {
      #[cfg(debug_assertions)]
      debug!("Destroying debug messenger.");
      #[cfg(debug_assertions)]
      self.debug_utils_loader.destroy_debug_utils_messenger(self.debug_messenger, None);
      debug!("Destroying instance.");
      self.instance.destroy_instance(None);
    }
  }
}

impl Deref for Instance {
  type Target = ash::Instance;
  fn deref(&self) -> &Self::Target {
    &self.instance
  }
}

//---------------------------Helpers------------------------

fn get_extensions(entry: &Entry, glfw: &Glfw) -> Result<Vec<CString>> {
  let mut required_extensions = get_required_extensions();
  let mut glfw_extensions = get_glfw_extensions(glfw)?;
  required_extensions.append(&mut glfw_extensions);
  let available_extensions = get_available_extensions(entry)?;

  required_match_available(&required_extensions, &available_extensions)
    .then_some(required_extensions)
    .ok_or(EngineError::CreationError("missing support for necessary extensions!"))
}

fn get_glfw_extensions(glfw: &Glfw) -> Result<Vec<CString>> {
  let extensions = glfw.get_required_instance_extensions().ok_or(EngineError::CreationError("failed to get requred glfw extensions"))?;
  extensions
    .into_iter()
    .map(|ext| CString::new(ext).map_err(|_| EngineError::CreationError("failed to process extension names")))
    .collect::<Result<Vec<CString>>>()
}

fn get_available_extensions(entry: &Entry) -> Result<Vec<CString>> {
  let extensions = entry.enumerate_instance_extension_properties(None)?;
  let extensions: Vec<CString> = extensions.into_iter().map(|extension| vk_to_string(&extension.extension_name).to_owned()).collect();
  Ok(extensions)
}

fn get_layers(entry: &Entry) -> Result<Vec<CString>> {
  let required_layers = get_required_layers();
  let available_layers = entry.enumerate_instance_layer_properties()?;
  let available_layers: Vec<CString> = available_layers.iter().map(|layer| vk_to_string(&layer.layer_name).to_owned()).collect();

  required_match_available(&required_layers, &available_layers)
    .then_some(required_layers)
    .ok_or(EngineError::CreationError("missing support for necessary layers!"))
}
