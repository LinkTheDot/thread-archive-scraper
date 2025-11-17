use clap::Arg;
use clap::Command;
use std::path::PathBuf;
use std::sync::OnceLock;

pub struct Args {
  args: clap::ArgMatches,
}

impl Args {
  const FILEPATH: &'static str = "filepath";

  pub fn new() -> Self {
    let args = Self::setup_args();

    Self { args }
  }

  pub fn get_file_path(&self) -> &'static PathBuf {
    static LOCK: OnceLock<PathBuf> = OnceLock::new();

    LOCK.get_or_init(|| {
      let Some(value) = self.args.get_one::<String>(Self::FILEPATH) else {
        let error = "Missing file path";
        tracing::error!(error);
        panic!("{error}");
      };

      PathBuf::from(value)
    })
  }

  fn setup_args() -> clap::ArgMatches {
    Command::new("Compares a list of `name: url` in a given file and removes duplicate urls.")
      .arg(
        Arg::new(Self::FILEPATH)
          .short('f')
          .long("file")
          .action(clap::ArgAction::Set)
          .help("Takes the file to resort as an argument."),
      )
      .get_matches()
  }
}

impl Default for Args {
  fn default() -> Self {
    Self::new()
  }
}
