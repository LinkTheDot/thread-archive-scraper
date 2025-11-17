use clap::*;
use std::fs;
use std::io::Read;
use std::io::Write;
use tracing::level_filters::LevelFilter;

pub mod clap;

fn main() {
  tracing_subscriber::fmt()
    .with_max_level(LevelFilter::INFO)
    .with_ansi(false)
    .init();

  let args = Args::new();
  let file_path = args.get_file_path();

  let mut file = fs::File::open(file_path).unwrap();
  let mut file_contents = String::new();
  file.read_to_string(&mut file_contents).unwrap();
  drop(file);

  let mut full_contents: Vec<(&str, &str)> = file_contents
    .lines()
    .filter_map(|line| {
      let split: Vec<&str> = line.splitn(2, ": ").collect();

      Some((*split.first()?, *split.get(1)?))
    })
    .collect();

  full_contents.sort_by(|(_, url_1), (_, url_2)| url_1.partial_cmp(url_2).unwrap());
  full_contents.dedup_by(|(_, url_1), (_, url_2)| url_1 == url_2);

  let mut file = fs::OpenOptions::new()
    .write(true)
    .truncate(true)
    .open(file_path)
    .unwrap();

  for (thread_id, url) in full_contents {
    if let Err(error) = writeln!(file, "{}: {}", thread_id, url) {
      tracing::error!("Failed to write a url for {thread_id:?}. Reason: {error:?}");
    }
  }
}
