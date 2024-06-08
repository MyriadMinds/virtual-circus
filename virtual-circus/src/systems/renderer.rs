use crate::framework::Model;
use crate::message_bus::{Message, MessageBox, MessageData};
use crate::utils::thread::Threaded;
use crate::utils::tools::{EngineError, Result};
use crate::vulkan::rendering_context::RenderingContext;
use crate::vulkan::{Vulkan, WindowResources};

use asset_lib::{Node, Scene};
use log::error;
use nalgebra_glm as glm;

use std::collections::HashMap;

pub(crate) struct Renderer {
  models: HashMap<u128, Model>,
  vulkan: Vulkan,
  message_box: MessageBox,
  scene: Option<Scene>,
}

impl Renderer {
  pub(crate) fn new(vulkan: Vulkan, message_box: MessageBox) -> Result<Self> {
    Ok(Self {
      vulkan,
      message_box,
      models: HashMap::new(),
      scene: None,
    })
  }

  fn save_model(&mut self, model: MessageData<Model>) {
    if let Some(model) = model.take() {
      self.models.insert(model.id, model);
    }
  }

  fn save_scene(&mut self, scene: MessageData<Scene>) {
    self.scene = scene.take();
  }

  fn process_message(&mut self, message: Message) {
    match message {
      Message::ModelReady(model) => self.save_model(model),
      Message::CurrentScene(scene) => self.save_scene(scene),
      _ => (),
    }
  }

  fn wait_for_window_resources(&mut self) -> WindowResources {
    loop {
      if let Some(message) = self.message_box.check_messages() {
        match message {
          Message::WindowResourcesReady(resources) => return resources.take().unwrap(),
          _ => self.process_message(message),
        }
      }
    }
  }

  fn draw_node(&self, matrix: glm::Mat4, node: &Node, rendering_context: &RenderingContext) {
    let matrix = matrix * node.transform;

    if let Some(model) = node.model {
      let model = self.scene.as_ref().unwrap().models()[model];
      let model = self.models.get(&model).unwrap();

      rendering_context.cmd_push_constants(&matrix);
      rendering_context.draw_model(model);
    }

    for node in &node.children {
      let node = &self.scene.as_ref().unwrap().nodes()[*node];
      self.draw_node(matrix.clone(), node, rendering_context);
    }
  }
}

impl Threaded for Renderer {
  fn run(&mut self) {
    self.message_box.post_message(Message::RequestWindowResources);
    // self.message_box.post_message(Message::RequestModel("models/Sword-01.glb".to_owned()));
    self.message_box.post_message(Message::RequestAsset("models/Vita.ast".to_owned()));

    let resources = self.wait_for_window_resources();

    let (mut window, events) = match self.vulkan.create_window(resources) {
      Ok(window) => window,
      Err(e) => {
        error!("Failed to create window: {}", e.to_string());
        return;
      }
    };

    while !window.should_close() && !self.message_box.should_close() {
      self.vulkan.poll_events();

      let Ok(rendering_context) = window.get_rendering_context() else {
        error!("Failed to get rednering context of a window!");
        continue;
      };

      if let Some(scene) = &self.scene {
        for node in scene.parent_nodes() {
          self.draw_node(glm::Mat4::identity(), &scene.nodes()[*node], &rendering_context);
        }
      }

      match window.draw_frame(rendering_context) {
        Ok(_) => (),
        Err(EngineError::OldSwapchain) => window.recreate_swapchain().unwrap(),
        Err(e) => {
          error!("Failed to draw frame: {}", e.to_string());
          break;
        }
      };

      window.progress_frame();

      if let Some(message) = self.message_box.check_messages() {
        self.process_message(message);
      }
    }

    self.vulkan.device_wait_idle();
    self.message_box.post_message(Message::Stop);
  }

  fn name(&self) -> String {
    "Renderer".to_owned()
  }
}
