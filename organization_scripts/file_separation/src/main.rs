use crate::clap::Args;
use itertools::Itertools;
use rand::{distributions::Alphanumeric, Rng};
use std::fs::{self};
use std::path::{Path, PathBuf};
use tracing::level_filters::LevelFilter;

pub mod clap;

fn main() {
  tracing_subscriber::fmt()
    .with_max_level(LevelFilter::INFO)
    .with_ansi(false)
    .init();

  let args = Args::new();
  let directory_path = args.get_directory_path().to_str().unwrap();

  let chunks = fs::read_dir(directory_path)
    .unwrap()
    .filter_map(|entry| {
      let path = entry.ok()?.path();

      path.is_file().then_some(path)
    })
    .chunks(*args.get_batch_size());

  for (batch_number, batch) in chunks.into_iter().enumerate() {
    tracing::info!("Working on batch {batch_number:?}");

    let batch_dir = format!("{}/{}", directory_path, generate_random_string(10));
    let batch_dir = Path::new(&batch_dir);

    if !batch_dir.exists() {
      tracing::info!("Crating missing batch directory.");

      if let Err(error) = fs::create_dir_all(batch_dir) {
        tracing::error!("Failed to create batch directory {batch_number:?}. Reason: `{error:?}`");

        continue;
      }
    }

    for path in batch {
      if !path.is_file() {
        continue;
      }

      let Some(file_name) = path.file_name().map(Path::new) else {
        tracing::error!("Failed to obtain filename for path {path:?}");
        continue;
      };
      let mut new_path = PathBuf::from(batch_dir);
      new_path.push(file_name);

      if let Err(error) = fs::rename(&path, &new_path) {
        tracing::error!("Failed to move {path:?} to {new_path:?}. Reason: {error:?}");
      }
    }
  }
}

fn generate_random_string(length: usize) -> String {
  rand::thread_rng()
    .sample_iter(&Alphanumeric)
    .take(length)
    .map(char::from)
    .collect()
}
