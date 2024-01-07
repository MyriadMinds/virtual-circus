use crate::utils::tools::Result;
use crate::vulkan::allocator::{Buffer, BufferType};
use crate::vulkan::rendering_context::RenderingContext;
use crate::vulkan::Allocator;

use ash::vk;
use asset_lib as ast;

pub(crate) struct Model {
  name: String,
  id: u128,
  meshes: Vec<ast::Mesh>,
  buffer: Buffer,
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

  pub(crate) fn draw(&self, rendering_context: &mut RenderingContext) {}
}

// pub(crate) trait ModelRequest {
//   fn wait_finalize()
// }
