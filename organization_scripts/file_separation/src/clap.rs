use clap::Arg;
use clap::Command;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Args {
  args: clap::ArgMatches,
}

impl Args {
  const DIRECTORY: &'static str = "directory";
  const BATCH_SIZE: &'static str = "split_count";

  pub fn new() -> Self {
    let args = Self::setup_args();

    Self { args }
  }

  pub fn get_directory_path(&self) -> &'static PathBuf {
    static LOCK: OnceLock<PathBuf> = OnceLock::new();

    LOCK.get_or_init(|| {
      let Some(value) = self.args.get_one::<String>(Self::DIRECTORY) else {
        let error = "Missing directory path.";
        tracing::error!(error);
        panic!("{}", error);
      };

      PathBuf::from(value)
    })
  }

  pub fn get_batch_size(&self) -> &'static usize {
    static LOCK: OnceLock<usize> = OnceLock::new();

    LOCK.get_or_init(|| {
      let Some(value) = self.args.get_one::<String>(Self::BATCH_SIZE) else {
        let error = "Missing batch size.";
        tracing::error!(error);
        panic!("{}", error);
      };

      value.trim().parse().unwrap()
    })
  }

  fn setup_args() -> clap::ArgMatches {
    Command::new("Takes a directory and batch size to split files into different directories.")
      .arg(
        Arg::new(Self::DIRECTORY)
          .short('d')
          .long("dir")
          .action(clap::ArgAction::Set)
          .help("Determines the directory to split up."),
      )
      .arg(
        Arg::new(Self::BATCH_SIZE)
          .short('b')
          .long("batch")
          .action(clap::ArgAction::Set)
          .help("Sets the batch size for how many files will be in each directory."),
      )
      .get_matches()
  }
}

impl Default for Args {
  fn default() -> Self {
    Self::new()
  }
}
