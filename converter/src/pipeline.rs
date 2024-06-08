use super::Converter;

use asset_lib as ast;
use ast::Asset;
use log::error;
use serde_yaml as yml;

use std::path::PathBuf;
pub(crate) struct PipelineConverter {}

impl Converter for PipelineConverter {
  fn parse_file(src_file: &str, output_dir: &str) {
    let mut path = PathBuf::new();
    path.push(src_file);
    let mut vertex_shader_path = path.clone();
    let mut fragment_shader_path = path.clone();

    let file = match std::fs::File::open(path) {
      Ok(file) => file,
      Err(e) => {
        error!("Failed to open pipeline file: {}", e);
        return;
      }
    };

    let document: ast::PipelineManifest = match yml::from_reader(file) {
      Ok(document) => document,
      Err(e) => {
        error!("Failed to deserialize file: {}", e);
        return;
      }
    };

    vertex_shader_path.pop();
    vertex_shader_path.push(document.vertex_shader);
    fragment_shader_path.pop();
    fragment_shader_path.push(document.fragment_shader);

    let vertex_file = match std::fs::read_to_string(vertex_shader_path) {
      Ok(file) => file,
      Err(e) => {
        error!("Failed to open vertex shader file: {}", e);
        return;
      }
    };

    let fragmet_file = match std::fs::read_to_string(fragment_shader_path) {
      Ok(file) => file,
      Err(e) => {
        error!("Failed to open fragment shader file: {}", e);
        return;
      }
    };

    let vertex_shader = compile_shader(&vertex_file, shaderc::ShaderKind::Vertex, &document.name);
    let fragment_shader = compile_shader(&fragmet_file, shaderc::ShaderKind::Fragment, &document.name);

    let pipeline = ast::Pipeline {
      name: document.name.clone(),
      blending: document.blending,
      vertex_shader: vertex_shader.as_binary_u8().to_owned(),
      fragment_shader: fragment_shader.as_binary_u8().to_owned(),
    };

    let name = document.name;
    let path = format!("{output_dir}/{name}.pipl");
    pipeline.convert_to_asset().unwrap().save_to_file(&path);
  }
}

fn compile_shader(code: &str, shader_type: shaderc::ShaderKind, filename: &str) -> shaderc::CompilationArtifact {
  let compiler = shaderc::Compiler::new().unwrap();
  let mut options = shaderc::CompileOptions::new().unwrap();
  options.set_target_env(shaderc::TargetEnv::Vulkan, shaderc::EnvVersion::Vulkan1_2 as u32);
  options.set_source_language(shaderc::SourceLanguage::GLSL);
  compiler.compile_into_spirv(code, shader_type, filename, "main", None).unwrap()
}
