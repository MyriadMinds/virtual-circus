use crate::framework::Model;
use crate::message_bus::{Message, MessageBox};
use crate::utils::thread::Threaded;
use crate::utils::tools::{EngineError, Result};
use crate::vulkan::{Vulkan, WindowResources};

use crate::error;

pub(crate) struct Renderer {
  models: Vec<Model>,
  vulkan: Vulkan,
  message_box: MessageBox,
}

impl Renderer {
  pub(crate) fn new(vulkan: Vulkan, message_box: MessageBox) -> Result<Self> {
    Ok(Self {
      vulkan,
      message_box,
      models: Vec::new(),
    })
  }

  fn process_message(&mut self, message: Message) {
    match message {
      Message::ModelReady(model) => {
        if let Some(model) = model.take() {
          self.models.push(model);
        }
      }
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
      match window.draw_frame(&self.models) {
        Ok(_) => (),
        Err(EngineError::OldSwapchain) => window.recreate_swapchain().unwrap(),
        Err(e) => {
          error!("Failed to draw frame: {}", e.to_string());
          break;
        }
      };

      if let Some(message) = self.message_box.check_messages() {
        self.process_message(message);
      }

      self.vulkan.poll_events();
    }

    self.vulkan.device_wait_idle();
    self.message_box.post_message(Message::Stop);
  }

  fn name(&self) -> String {
    "Renderer".to_owned()
  }
}
