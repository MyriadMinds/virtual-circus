use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
  println!("cargo:rerun-if-changed=shaders/");
  println!("cargo:rerun-if-changed=config/");
  println!("cargo:warning={:?}", env::current_exe().unwrap());
  let project_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
  let out_dir = match env::var("CARGO_BUILD_TARGET_DIR") {
    Ok(dir_string) => {
      let mut dir = PathBuf::new();
      dir.push(dir_string);
      dir
    }
    Err(_) => {
      let dir_string = env::var("OUT_DIR").unwrap();
      let mut dir = PathBuf::new();
      dir.push(dir_string);
      dir.pop();
      dir.pop();
      dir.pop();
      dir
    }
  };

  let mut shader_dir = out_dir.clone();
  shader_dir.push("shaders");
  match fs::create_dir_all(shader_dir.as_path()) {
    Ok(_) => (),
    Err(e) => println!("cargo:warning=failed to create directory for compiled shaders: {}", e),
  };

  let mut config_dir = out_dir.clone();
  config_dir.push("config");
  let mut copy_options = fs_extra::dir::CopyOptions::new();
  copy_options.overwrite = true;
  copy_options.copy_inside = true;
  let mut config_src = project_dir.clone();
  config_src.push("config");
  fs_extra::dir::copy(config_src, config_dir, &copy_options).unwrap();

  let mut shader_src = project_dir.clone();
  shader_src.push("shaders");
  println!("cargo:warning=reading shaders from: {:?}", shader_src);
  let files = fs::read_dir(shader_src).unwrap();
  let compiler = shaderc::Compiler::new().unwrap();

  for entry in files {
    let entry = match entry {
      Ok(entry) => entry.path(),
      Err(e) => {
        println!("cargo:warning=failed to open file: {}, skipping...", e);
        continue;
      }
    };

    let file_path = entry.to_str().unwrap();
    let file_type = match entry.extension() {
      Some(file_type) => file_type.to_str().unwrap(),
      None => {
        println!("cargo:warning=failed to read extension of file: {}, skipping...", file_path);
        continue;
      }
    };

    let shader_kind = match file_type {
      "vert" => shaderc::ShaderKind::Vertex,
      "frag" => shaderc::ShaderKind::Fragment,
      _ => {
        println!("cargo:warning=shader file {} has unknown extension: {}, skipping...", file_path, file_type);
        continue;
      }
    };

    let code = match fs::read_to_string(file_path) {
      Ok(code) => code,
      Err(e) => {
        println!("cargo:warning=shader file {} has failed to open: {}, skipping...", file_path, e);
        continue;
      }
    };

    let mut options = shaderc::CompileOptions::new().unwrap();
    options.set_target_env(shaderc::TargetEnv::Vulkan, shaderc::EnvVersion::Vulkan1_2 as u32);
    options.set_source_language(shaderc::SourceLanguage::GLSL);
    let shader = compiler.compile_into_spirv(&code, shader_kind, entry.file_name().unwrap().to_str().unwrap(), "main", None).unwrap();

    let mut output_file = shader_dir.clone();
    output_file.push(entry.file_name().unwrap());
    output_file.set_extension(format!("{}.spv", file_type));
    match fs::write(output_file.as_path(), shader.as_binary_u8()) {
      Ok(_) => (),
      Err(e) => println!(
        "cargo:warning=failed to write compiled shader {} to file: {}, skipping...",
        output_file.as_os_str().to_str().unwrap(),
        e
      ),
    };
  }
}
