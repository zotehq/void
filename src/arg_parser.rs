use std::env;
use std::error::Error;

pub struct ArgParser {
  pub verbose: bool,
}

impl ArgParser {
  pub fn new() -> Result<Self, Box<dyn Error>> {
    let mut verbose = false;

    // NOTE: this skips the first argument (binary)
    let args = env::args().skip(1);

    for arg in args {
      match arg.as_str() {
        "--verbose" => {
          if verbose {
            return Err("--verbose was used more than once".into());
          }
          verbose = true;
        }
        _ => return Err(format!("unknown argument: {}", arg).into()),
      }
    }

    Ok(Self { verbose })
  }
}
