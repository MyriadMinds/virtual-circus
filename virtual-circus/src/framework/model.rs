mod gltf;

use crate::vulkan::rendering_context::RenderingContext;
pub(crate) use gltf::GltfModel;

pub(crate) trait Model {
  unsafe fn draw(&self, rendering_context: &mut RenderingContext);
}

// pub(crate) trait ModelRequest {
//   fn wait_finalize()
// }
