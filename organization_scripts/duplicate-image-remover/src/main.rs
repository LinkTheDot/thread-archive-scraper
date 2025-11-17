use crate::clap::Args;
use image_hasher::{HasherConfig, ImageHash};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tracing::level_filters::LevelFilter;

pub mod clap;

const DUPLICATE_LOG_FILE_PATH: &str = "duplicates.txt";

fn main() {
  tracing_subscriber::fmt()
    .with_max_level(LevelFilter::INFO)
    .with_ansi(false)
    .init();

  let args = Args::new();
  let directory_path = args.get_directory_path().to_str().unwrap();
  let destination = args
    .get_destination_path()
    .as_ref()
    .and_then(|p| p.to_str())
    .unwrap_or(directory_path);
  let image_hashes_and_paths = get_image_hashes_and_paths(directory_path);

  if !Path::new(destination).exists() {
    tracing::info!("Destination {destination:?} doesn't exist, creating it.");

    fs::create_dir_all(destination).unwrap();
  }

  let mut duplicate_log_file_path = PathBuf::from(destination);
  duplicate_log_file_path.push(DUPLICATE_LOG_FILE_PATH);

  let mut duplicate_log_file = fs::OpenOptions::new()
    .write(true)
    .truncate(false)
    .create(true)
    .open(duplicate_log_file_path)
    .unwrap();

  for image_path_list in image_hashes_and_paths.values() {
    if image_path_list.len() <= 1 {
      continue;
    }

    tracing::info!("Found a duplicate list.");

    image_path_list.iter().skip(1).for_each(|path| {
      let Some(file_name) = path.file_name() else {
        tracing::error!("Failed to get the filename for path `{path:?}`");
        return;
      };
      let mut new_path = PathBuf::from(destination);
      new_path.push(file_name);

      if let Err(error) = fs::rename(path, &new_path) {
        tracing::error!("Failed to change path `{path:?}` to `{new_path:?}`. Reason: `{error:?}`");
      }
    });

    if let Err(error) = writeln!(duplicate_log_file, "{:?}", image_path_list) {
      tracing::error!("Failed to write a duplicate list to the log file. Reason: `{error:?}`");
    }
  }
}

fn path_is_image<P: AsRef<Path>>(path: P) -> bool {
  let path = path.as_ref();
  let mime = mime_guess::MimeGuess::from_path(path).first();

  match mime {
    Some(mime) => mime.type_() == "image",
    None => false,
  }
}

fn get_image_hashes_and_paths<P: AsRef<Path>>(
  directory_path: P,
) -> HashMap<ImageHash, Vec<PathBuf>> {
  let mut image_hashes_and_paths_map = HashMap::new();
  let hasher = HasherConfig::new().to_hasher();

  let image_file_paths: Vec<PathBuf> = fs::read_dir(directory_path)
    .unwrap()
    .filter_map(|entry| {
      let path = entry.ok()?.path();
      let path_meets_criteria = path.is_file() && path_is_image(&path);

      path_meets_criteria.then_some(path)
    })
    .collect();

  let image_hashes_and_paths_list: Vec<(ImageHash, PathBuf)> = image_file_paths
    .into_par_iter()
    .filter_map(|file_path| {
      tracing::info!("Processing image file {file_path:?}...");

      let image = match image::open(&file_path) {
        Ok(image) => image,
        Err(error) => {
          tracing::error!("Failed to read {file_path:?} as an image. Reason: `{error:?}`");
          return None;
        }
      };

      let image_hash = hasher.hash_image(&image);

      Some((image_hash, file_path))
    })
    .collect();

  image_hashes_and_paths_list
    .into_iter()
    .for_each(|(image_hash, file_path)| {
      let entry_list = image_hashes_and_paths_map
        .entry(image_hash)
        .or_insert(vec![]);
      entry_list.push(file_path);
    });

  image_hashes_and_paths_map
}
