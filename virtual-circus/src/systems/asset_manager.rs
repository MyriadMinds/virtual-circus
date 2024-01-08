use crate::framework::Model;
use crate::message_bus::{Message, MessageBox, MessageData};
use crate::utils::constants::*;
use crate::utils::thread::Threaded;
use crate::utils::tools::Result;
use crate::vulkan::allocator::{Image, ImagePurpose};
use crate::vulkan::descriptors::{GlobalDescriptorSetLayout, MaterialDescriptorSetLayout};
use crate::vulkan::WindowResources;
use crate::vulkan::{Allocator, Vulkan};

use ash::vk;
use asset_lib as ast;

use ast::AssetFile;
use log::error;
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;

pub(crate) struct AssetManager {
  message_box: MessageBox,
  allocator: Allocator,
  global_descriptor_set_layout: Arc<GlobalDescriptorSetLayout>,
  material_descriptor_set_layout: Arc<MaterialDescriptorSetLayout>,
}

#[derive(Default)]
struct AssetGroup {
  models: Vec<ast::Model>,
  scenes: Vec<ast::Scene>,
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

  fn load_assets(&mut self, path: String) {
    let mut asset_group = match parse_asset_file(&path) {
      Ok(asset_group) => asset_group,
      Err(e) => {
        error!("Failed to parse assets: {}", e);
        return;
      }
    };

    let models = match asset_group.convert_models(&mut self.allocator) {
      Ok(models) => models,
      Err(e) => {
        error!("Failed to convert model assets: {}", e);
        return;
      }
    };
    self.allocator.flush();

    let scenes = asset_group.scenes.drain(..);

    for model in models {
      let message = MessageData::new(model);
      self.message_box.post_message(Message::ModelReady(message));
    }

    for scene in scenes {
      let message = MessageData::new(scene);
      self.message_box.post_message(Message::SceneReady(message));
    }
  }

  fn prepare_window_resources(&mut self) {
    let Ok(global_descriptor_sets) = self.global_descriptor_set_layout.create_descriptor_sets(&mut self.allocator, 1) else {
      error!("Failed to create global descriptor set for window request");
      return;
    };

    let Ok(depth_images) = create_window_images(
      &mut self.allocator,
      MAX_FRAMES_IN_FLIGHT,
      DEPTH_FORMAT,
      vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT,
      ImagePurpose::DepthBuffer,
    ) else {
      error!("Failed to create depth images for window request");
      return;
    };

    let Ok(color_images) = create_window_images(
      &mut self.allocator,
      MAX_FRAMES_IN_FLIGHT,
      vk::Format::R8G8B8A8_SRGB,
      vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
      ImagePurpose::ColorAttachment,
    ) else {
      error!("Failed to create color images for window request");
      return;
    };
    let resources = WindowResources {
      depth_images,
      color_images,
      global_descriptor_sets,
    };
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
          Message::RequestAsset(path) => self.load_assets(path),
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

fn create_window_images(allocator: &mut Allocator, count: u32, format: vk::Format, usage: vk::ImageUsageFlags, purpose: ImagePurpose) -> Result<Vec<Image>> {
  let extent = vk::Extent3D { width: 3840, height: 2160, depth: 1 };

  let image_create_info = vk::ImageCreateInfo {
    format,
    tiling: vk::ImageTiling::OPTIMAL,
    usage,
    image_type: vk::ImageType::TYPE_2D,
    samples: vk::SampleCountFlags::TYPE_1,
    mip_levels: 1,
    array_layers: 1,
    extent,
    ..Default::default()
  };

  let mut images = Vec::with_capacity(count as usize);
  for _ in 0..count {
    images.push(allocator.create_image(&[], image_create_info, purpose)?);
  }

  Ok(images)
}

impl AssetGroup {
  fn add_asset(&mut self, asset: AssetFile) -> Result<()> {
    match asset.asset_type() {
      ast::AssetType::Model => self.models.push(ast::Model::load_model(asset)?),
      ast::AssetType::Scene => self.scenes.push(ast::Scene::load_scene(asset)?),
    }

    Ok(())
  }

  fn convert_models(&mut self, allocator: &mut Allocator) -> Result<Vec<Model>> {
    self.models.drain(..).map(|model| Model::new(model, allocator)).collect::<Result<Vec<Model>>>()
  }
}

fn parse_asset_file(path: &str) -> Result<AssetGroup> {
  let mut asset_group = AssetGroup::default();
  let path_buf = std::path::PathBuf::from(path);

  match path_buf.extension().unwrap().to_str().unwrap() {
    "ast" => {
      let assets = ast::AssetArchive::get_assets(path)?;

      for asset in assets {
        asset_group.add_asset(asset)?;
      }
    }
    _ => {
      asset_group.add_asset(AssetFile::load_from_file(&path)?)?;
    }
  }

  Ok(asset_group)
}
