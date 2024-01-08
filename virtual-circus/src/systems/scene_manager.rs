use crate::message_bus::{Message, MessageBox, MessageData};
use crate::utils::thread::Threaded;

use asset_lib as ast;

pub(crate) struct SceneManager {
  message_box: MessageBox,
  scenes: Vec<ast::Scene>,
}

impl SceneManager {
  pub(crate) fn new(message_box: MessageBox) -> Self {
    Self { message_box, scenes: Vec::new() }
  }

  fn save_scene(&mut self, scene: MessageData<ast::Scene>) {
    if let Some(scene) = scene.take() {
      let data = MessageData::new(scene.clone());
      self.message_box.post_message(Message::CurrentScene(data));
      self.scenes.push(scene);
    }
  }
}

impl Threaded for SceneManager {
  fn run(&mut self) {
    while !self.message_box.should_close() {
      if let Some(message) = self.message_box.check_messages() {
        match message {
          Message::SceneReady(data) => self.save_scene(data),
          _ => (),
        }
      }
    }
  }

  fn name(&self) -> String {
    "Scene Manager".to_owned()
  }
}
