use crate::framework::GltfModel;
use crate::vulkan::WindowResources;

use log::debug;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) enum Message {
  Stop,
  RequestWindowResources,
  RequestModel(String),
  WindowResourcesReady(MessageData<WindowResources>),
  ModelReady(MessageData<GltfModel>),
}

impl Message {
  pub(super) fn log_message(&self) {
    match self {
      Message::Stop => debug!("Message: Stop"),
      Message::RequestWindowResources => debug!("Message: RequestWindowResources"),
      Message::RequestModel(path) => debug!("Message: RequestModel {}", path),
      Message::WindowResourcesReady(_) => debug!("Message: WindowResourcesReady"),
      Message::ModelReady(_) => debug!("Message: ModelReady"),
    }
  }
}

pub(crate) struct MessageData<T> {
  content: Arc<Mutex<Option<T>>>,
}

impl<T> MessageData<T> {
  pub(crate) fn new(content: T) -> Self {
    let content = Arc::new(Mutex::new(Some(content)));
    Self { content }
  }

  pub(crate) fn take(self) -> Option<T> {
    match self.content.lock() {
      Ok(mut content) => content.take(),
      Err(_) => None,
    }
  }
}

impl<T> Clone for MessageData<T> {
  fn clone(&self) -> Self {
    Self { content: self.content.clone() }
  }
}
