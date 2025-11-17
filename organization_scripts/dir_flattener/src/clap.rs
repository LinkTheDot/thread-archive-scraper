use clap::Arg;
use clap::Command;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Args {
  args: clap::ArgMatches,
}

impl Args {
  const DIRECTORY: &'static str = "directory";
  const CUSTOM_DESTINATION: &'static str = "custom_dir";

  pub fn new() -> Self {
    let args = Self::setup_args();

    Self { args }
  }

  pub fn get_directory_path(&self) -> &'static PathBuf {
    static LOCK: OnceLock<PathBuf> = OnceLock::new();

    LOCK.get_or_init(|| {
      let Some(value) = self.args.get_one::<String>(Self::DIRECTORY) else {
        tracing::error!("Missing directory path.");
        panic!("Missing directory path.");
      };

      PathBuf::from(value)
    })
  }

  pub fn get_custom_destination(&self) -> &'static Option<PathBuf> {
    static LOCK: OnceLock<Option<PathBuf>> = OnceLock::new();

    LOCK.get_or_init(|| {
      self
        .args
        .get_one::<String>(Self::CUSTOM_DESTINATION)
        .map(PathBuf::from)
    })
  }

  fn setup_args() -> clap::ArgMatches {
    Command::new("Recursively flattens all files under a directory to be at a given destination.")
      .arg(
        Arg::new(Self::DIRECTORY)
          .short('d')
          .long("dir")
          .action(clap::ArgAction::Set)
          .help("Determines the directory to flatten."),
      )
      .arg(
        Arg::new(Self::CUSTOM_DESTINATION)
          .short('m')
          .long("dest")
          .action(clap::ArgAction::Set)
          .help("Sets a custom destination to move the files to."),
      )
      .get_matches()
  }
}

impl Default for Args {
  fn default() -> Self {
    Self::new()
  }
}
