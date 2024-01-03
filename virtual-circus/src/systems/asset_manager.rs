use crate::framework::model::GltfModel;
use crate::message_bus::{Message, MessageBox, MessageData};
use crate::utils::constants::*;
use crate::utils::thread::Threaded;
use crate::utils::tools::Result;
use crate::vulkan::allocator::{Image, ImagePurpose};
use crate::vulkan::descriptors::{GlobalDescriptorSetLayout, MaterialDescriptorSetLayout};
use crate::vulkan::WindowResources;
use crate::vulkan::{Allocator, Vulkan};

use ash::vk;

use log::error;
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;

pub(crate) struct AssetManager {
  message_box: MessageBox,
  allocator: Allocator,
  global_descriptor_set_layout: Arc<GlobalDescriptorSetLayout>,
  material_descriptor_set_layout: Arc<MaterialDescriptorSetLayout>,
}

impl AssetManager {
  pub(crate) fn new(vulkan: &Vulkan, message_box: MessageBox) -> Result<Self> {
    let allocator = vulkan.create_allocator()?;
    let global_descriptor_set_layout = vulkan.get_global_descriptor_set_layout();
    let material_descriptor_set_layout = vulkan.get_material_descriptor_set_layout();

    Ok(Self {
      message_box,
      allocator,
      global_descriptor_set_layout,
      material_descriptor_set_layout,
    })
  }

  fn prepare_model(&mut self, path: String) {
    let model = GltfModel::new(&path, &mut self.allocator, &self.material_descriptor_set_layout).unwrap();
    let model = MessageData::new(model);

    self.allocator.flush();
    self.message_box.post_message(Message::ModelReady(model));
  }

  fn prepare_window_resources(&mut self) {
    let Ok(global_descriptor_sets) = self.global_descriptor_set_layout.create_descriptor_sets(&mut self.allocator, 1) else {
      error!("Failed to create global descriptor set for window request");
      return;
    };

    let Ok(depth_images) = create_depth_images(&mut self.allocator, 10) else {
      error!("Failed to create depth images for window request");
      return;
    };
    let resources = WindowResources { depth_images, global_descriptor_sets };
    let resources = MessageData::new(resources);

    self.allocator.flush();
    self.message_box.post_message(Message::WindowResourcesReady(resources));
  }
}

impl Threaded for AssetManager {
  fn run(&mut self) {
    while !self.message_box.should_close() {
      // process potential dealocations first to free up memory on the GPU.
      match self.allocator.process_deallocations() {
        Ok(_) | Err(TryRecvError::Empty) => (),
        Err(TryRecvError::Disconnected) => {
          error!("GPU Allocator unexpectedly lost ability to process deallocations, closing down");
          self.message_box.post_message(Message::Stop);
          break;
        }
      };

      // process requests for assets.
      if let Some(message) = self.message_box.check_messages() {
        match message {
          Message::RequestModel(path) => self.prepare_model(path),
          Message::RequestWindowResources => self.prepare_window_resources(),
          _ => (),
        }
      }
    }

    self.allocator.cleanup();
  }

  fn name(&self) -> String {
    "Asset Manager".to_owned()
  }
}

fn create_depth_images(allocator: &mut Allocator, count: u32) -> Result<Vec<Image>> {
  let extent = vk::Extent3D { width: 3840, height: 2160, depth: 1 };

  let image_create_info = vk::ImageCreateInfo {
    format: DEPTH_FORMAT,
    tiling: vk::ImageTiling::OPTIMAL,
    usage: vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
    image_type: vk::ImageType::TYPE_2D,
    samples: vk::SampleCountFlags::TYPE_1,
    mip_levels: 1,
    array_layers: 1,
    extent,
    ..Default::default()
  };

  let mut images = Vec::with_capacity(count as usize);
  for _ in 0..count {
    images.push(allocator.create_image(&[], image_create_info, ImagePurpose::DepthBuffer)?);
  }

  Ok(images)
}
