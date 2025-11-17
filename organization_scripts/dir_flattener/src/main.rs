use crate::clap::Args;
use std::fs;
use std::path::{Path, PathBuf};
use tracing::level_filters::LevelFilter;
use walkdir::WalkDir;

pub mod clap;

fn main() {
  tracing_subscriber::fmt()
    .with_max_level(LevelFilter::INFO)
    .with_ansi(false)
    .init();

  let args = Args::new();
  let directory_path = args.get_directory_path().to_str().unwrap();
  let destination = args
    .get_custom_destination()
    .as_ref()
    .and_then(|p| p.to_str())
    .unwrap_or(directory_path);
  let walkdir = WalkDir::new(directory_path);

  if !Path::new(destination).exists() {
    tracing::info!("Destination {destination:?} doesn't exist, creating it.");

    fs::create_dir_all(destination).unwrap();
  }

  for entry_result in walkdir {
    let path = match entry_result.as_ref() {
      Ok(entry) => entry.path(),
      Err(error) => {
        tracing::error!("Failed to read a dir entry. Reason: `{error:?}`");
        continue;
      }
    };

    if path.is_dir() || path == Path::new(directory_path) {
      continue;
    }

    let Some(filename) = path
      .file_name()
      .and_then(|p| p.to_str().map(|s| s.to_string()))
    else {
      tracing::error!("File has an invalid name.");
      continue;
    };

    let new_location = PathBuf::from(format!("{}/{}", destination, filename));

    if let Err(error) = fs::rename(path, &new_location) {
      tracing::error!("An error occurred when attempting to rename {path:?} to {new_location:?}. Reason: `{error:?}`");
    }
  }
}
