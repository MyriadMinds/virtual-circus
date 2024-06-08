mod error;
mod gltf;
mod pipeline;

pub(crate) use error::{ConverterError, Result};

use clap::{arg, command, Parser};
use log::{error, info};
use log4rs::append::console::ConsoleAppender;
use log4rs::config::{Appender, Config, Root};

use std::path::PathBuf;
use std::process::ExitCode;

pub(crate) trait Converter {
  fn parse_file(src_file: &str, output_dir: &str);
}

#[derive(Parser)]
#[command(version, about)]
struct Args {
  /// assey file to convert
  #[arg(id = "FILE")]
  src_path: String,
  /// output file to produce
  #[arg(short, long)]
  output_path: Option<String>,
}

fn main() -> ExitCode {
  initialize_logging();

  let (src_file, output_dir) = match parse_args() {
    Ok(files) => files,
    Err(e) => {
      error!("Failed to parse application arguments: {}", e);
      return ExitCode::FAILURE;
    }
  };

  convert_file(&src_file, &output_dir);

  ExitCode::SUCCESS
}

fn initialize_logging() {
  let mut config_file = std::env::current_exe().unwrap();
  config_file.pop();
  config_file.push("config/converter_log4rs.yaml");

  if !config_file.is_file() {
    println!("Couldn't find a log config file, initializing default console logger.");
    initialize_default_logger();
  } else if log4rs::init_file(config_file, Default::default()).is_err() {
    println!("Failed to initialize logger from config file, defaulting to console logger.");
    initialize_default_logger();
  }
}

fn initialize_default_logger() {
  let stdout = ConsoleAppender::builder().build();
  let config = Config::builder()
    .appender(Appender::builder().build("stdout", Box::new(stdout)))
    .build(Root::builder().appender("stdout").build(log::LevelFilter::Info))
    .unwrap();

  log4rs::init_config(config).unwrap();
}

fn parse_args() -> Result<(PathBuf, PathBuf)> {
  let args = Args::parse();
  let mut src_file = PathBuf::new();
  src_file.push(args.src_path);

  if !src_file.is_file() {
    return Err(ConverterError::ArgsError("Provided source path is not a file!"));
  }

  if src_file.extension().is_none() {
    return Err(ConverterError::ArgsError("Source file has no extension, could not figure out format!"));
  }

  let mut output_dir = PathBuf::new();
  match args.output_path {
    Some(path) => output_dir.push(path),
    None => {
      output_dir.push(std::env::current_dir().map_err(|_| ConverterError::ArgsError("Couldn't open output directory!"))?);
    }
  }

  Ok((src_file, output_dir))
}

fn convert_file(src_file: &PathBuf, output_dir: &PathBuf) {
  let extension = src_file.extension().unwrap().to_str().unwrap();

  let src_file = src_file.to_str().unwrap();
  let output_dir = output_dir.to_str().unwrap();

  match extension {
    "gltf" | "glb" | "vrm" => {
      info!("Parsing gltf file {}", src_file);
      gltf::GLTFConverter::parse_file(src_file, output_dir);
    }
    "pipmf" => {
      info!("Parsing pipline manifest {}", src_file);
      pipeline::PipelineConverter::parse_file(src_file, output_dir);
    }
    _ => error!("file {} has an unknown format, skipping...", src_file),
  }
}
