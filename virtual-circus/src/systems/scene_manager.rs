use crate::message_bus::{Message, MessageBox};
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
}

impl Threaded for SceneManager {
  fn run(&mut self) {
    while !self.message_box.should_close() {
      if let Some(message) = self.message_box.check_messages() {
        match message {
          Message::SceneReady(data) => self.scenes.push(data.take().unwrap()),
          _ => (),
        }
      }
    }
  }

  fn name(&self) -> String {
    "Scene Manager".to_owned()
  }
}
