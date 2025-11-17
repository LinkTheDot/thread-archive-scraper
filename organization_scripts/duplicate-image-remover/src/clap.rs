use clap::Arg;
use clap::Command;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Args {
  args: clap::ArgMatches,
}

impl Args {
  const DIRECTORY: &'static str = "directory";
  const DESTINATION: &'static str = "destination";

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

      let value = PathBuf::from(value);

      (!value.exists()).then(|| panic!("Given directory does not exist."));

      value
    })
  }

  pub fn get_destination_path(&self) -> &'static Option<PathBuf> {
    static LOCK: OnceLock<Option<PathBuf>> = OnceLock::new();

    LOCK.get_or_init(|| {
      self
        .args
        .get_one::<String>(Self::DESTINATION)
        .map(PathBuf::from)
    })
  }

  fn setup_args() -> clap::ArgMatches {
    Command::new("Just the surface level directory given for duplicate images, and moves them to the given destination.")
      .arg(
        Arg::new(Self::DIRECTORY)
          .short('d')
          .long("dir")
          .action(clap::ArgAction::Set)
          .help("Determines the directory to remove duplicates from."),
      )
      .arg(
        Arg::new(Self::DESTINATION)
          .short('m')
          .long("dest")
          .action(clap::ArgAction::Set)
          .help("Sets a custom destination to move the duplicate files to."),
      )
      .get_matches()
  }
}

impl Default for Args {
  fn default() -> Self {
    Self::new()
  }
}
