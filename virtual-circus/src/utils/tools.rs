use log::trace;
use thiserror::Error;

use std::ffi::CStr;
use std::os::raw::c_char;

//---------------------------Error------------------------
pub(crate) type Result<T> = std::result::Result<T, EngineError>;

#[derive(Error, Debug)]
pub(crate) enum EngineError {
  #[error("swapchain currently in use no longer matches drawing surface")]
  OldSwapchain,
  #[error("creation of resource has failed: {0}")]
  CreationError(&'static str),
  #[error("faile to initialize glfw: {0}")]
  GltfError(#[from] glfw::InitError),
  #[error("failed to load the vulkan driver: {0}")]
  LoadingError(#[from] ash::LoadingError),
  #[error("failed to execute a vulkan operation: {0}")]
  VulkanError(#[from] ash::vk::Result),
  #[error("failed to create allocator: {0}")]
  AllocatorError(#[from] gpu_allocator::AllocationError),
  #[error("failed to process asset file: {0}")]
  AssetError(#[from] asset_lib::AssetError),
}
//---------------------------Macros------------------------

//---------------------------Storage helpers------------------------

//---------------------------Misc helper functions------------------------

pub(crate) fn vk_to_string(string: &[c_char]) -> &CStr {
  unsafe { CStr::from_ptr(string.as_ptr()) }
}

pub(crate) fn required_match_available<T: AsRef<CStr> + Eq + std::fmt::Debug>(required: &[T], available: &[T]) -> bool {
  required.iter().all(|item| {
    trace!("Looking for {:?}", item);
    available.contains(item)
  })
}
