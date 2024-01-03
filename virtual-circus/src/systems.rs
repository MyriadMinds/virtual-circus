mod asset_manager;
mod renderer;

pub(crate) use asset_manager::AssetManager;
pub(crate) use renderer::Renderer;

use crate::utils::thread::{Thread, Threaded};

pub(crate) struct Systems {
  systems: Vec<Thread>,
}

impl Systems {
  pub(crate) fn new() -> Self {
    Self { systems: Vec::new() }
  }

  pub(crate) fn add_system(&mut self, system: impl Threaded + Send + 'static) {
    let system = Thread::new(system);
    self.systems.push(system);
  }

  pub(crate) fn all_systems_finished(&self) -> bool {
    self.systems.iter().all(|system| system.is_finished())
  }
}
