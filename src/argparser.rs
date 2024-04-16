use std::error::Error;

pub struct ArgParser {
  pub verbose: bool,
}

impl ArgParser {
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let mut args = std::env::args();
    args.next(); // skip the first argument in the iterator which is ALWAYS the path to the binary

    let mut verbose = false;

    for arg in args {
      if arg == "--verbose" {
        if verbose {
          return Err("--verbose was used more than once".into());
        }

        verbose = true;
        continue;
      }

      return Err(format!("unknown argument {}", arg).into());
    }

    Ok(Self { verbose })
  }
}
