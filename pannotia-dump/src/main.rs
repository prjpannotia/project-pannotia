use std::error;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader};
use std::process::ExitCode;

#[derive(Debug)]
pub enum Error {
    WrongArgs,
    IoError(io::Error),
    BitstreamContainerError(pannotia::container::BitstreamContainerError),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongArgs => write!(f, "wrong number of arguments"),
            Self::IoError(e) => e.fmt(f),
            Self::BitstreamContainerError(e) => e.fmt(f),
        }
    }
}
impl error::Error for Error {
    fn cause(&self) -> Option<&dyn error::Error> {
        match self {
            Self::IoError(e) => Some(e),
            Self::BitstreamContainerError(e) => Some(e),
            _ => None,
        }
    }
}
impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self::IoError(value)
    }
}
impl From<pannotia::container::BitstreamContainerError> for Error {
    fn from(value: pannotia::container::BitstreamContainerError) -> Self {
        Self::BitstreamContainerError(value)
    }
}

fn main() -> Result<ExitCode, Error> {
    env_logger::init();
    let args = std::env::args_os().collect::<Vec<_>>();

    if args.len() < 2 {
        println!("Usage: {} file.bin", args[0].to_string_lossy());
        return Err(Error::WrongArgs);
    }

    let f = BufReader::new(File::open(&args[1])?);
    let b = pannotia::container::Bitstream::read(f)?;
    // dbg!(b);

    Ok(ExitCode::SUCCESS)
}
