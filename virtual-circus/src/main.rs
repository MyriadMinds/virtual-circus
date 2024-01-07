mod framework;
mod message_bus;
mod systems;
mod utils;
mod vulkan;

use message_bus::MessageBus;
use systems::{AssetManager, Renderer, SceneManager, Systems};
use utils::tools::Result;
use vulkan::Vulkan;

use log::{error, info};
use std::process::ExitCode;

fn main() -> ExitCode {
  let mut config_file = std::env::current_exe().unwrap();
  config_file.pop();
  config_file.push("config/log4rs.yaml");
  match log4rs::init_file(config_file, Default::default()) {
    Ok(_) => (),
    Err(e) => {
      println!("Failed to initialize logging: {}", e);
      return ExitCode::FAILURE;
    }
  };

  match run_systems() {
    Ok(_) => (),
    Err(e) => error!("Initialization failed: {}", e.to_string()),
  }

  info!("Successfully closed!");
  ExitCode::SUCCESS
}

fn run_systems() -> Result<()> {
  let mut systems = Systems::new();

  let mut message_bus = MessageBus::new();

  let vulkan = Vulkan::init()?;

  let asset_manager = AssetManager::new(&vulkan, message_bus.get_message_box())?;
  systems.add_system(asset_manager);

  let renderer = Renderer::new(vulkan, message_bus.get_message_box())?;
  systems.add_system(renderer);

  let scene_manager = SceneManager::new(message_bus.get_message_box());
  systems.add_system(scene_manager);

  systems.add_system(message_bus);
  while !systems.all_systems_finished() {}
  Ok(())
}
