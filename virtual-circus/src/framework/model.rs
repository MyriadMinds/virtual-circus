use crate::utils::tools::Result;
use crate::vulkan::allocator::{Buffer, BufferType};
use crate::vulkan::Allocator;

use ash::vk;
use asset_lib as ast;

pub(crate) struct Model {
  pub(crate) name: String,
  pub(crate) id: u128,
  pub(crate) meshes: Vec<ast::Mesh>,
  pub(crate) buffer: Buffer,
}

impl Model {
  pub(crate) fn new(model: ast::Model, allocator: &mut Allocator) -> Result<Self> {
    let usage_flags = vk::BufferUsageFlags::VERTEX_BUFFER | vk::BufferUsageFlags::INDEX_BUFFER;
    let buffer = allocator.create_buffer_from_data(&model.blob, usage_flags, BufferType::GpuOnly)?;

    Ok(Self {
      name: model.name,
      id: model.id,
      meshes: model.meshes,
      buffer,
    })
  }
}

// pub(crate) trait ModelRequest {
//   fn wait_finalize()
// }
