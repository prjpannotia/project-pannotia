use std::error;
use std::fmt::Display;
use std::fs::File;
use std::io;
use std::process::ExitCode;

#[derive(Debug)]
pub enum Error {
    WrongArgs,
    IoError(io::Error),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::WrongArgs => write!(f, "wrong number of arguments"),
            Error::IoError(e) => e.fmt(f),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            Error::IoError(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}

fn main() -> Result<ExitCode, Error> {
    env_logger::init();
    let args = std::env::args_os().collect::<Vec<_>>();

    if args.len() < 2 {
        println!("Usage: {} file.bin", args[0].to_string_lossy());
        return Err(Error::WrongArgs);
    }

    let mut f = File::open(&args[1])?;
    dbg!(&f);

    Ok(ExitCode::SUCCESS)
}
