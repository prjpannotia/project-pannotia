use std::error;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::process::ExitCode;

#[derive(Debug)]
pub enum Error {
    WrongArgs,
    InvalidMode,
    IoError(io::Error),
    BitstreamContainerError(pannotia::container::BitstreamContainerError),
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::WrongArgs => write!(f, "wrong number of arguments"),
            Self::InvalidMode => write!(f, "invalid dump mode"),
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

    if args.len() < 3 {
        println!("Usage: {} dump_mode file.bin", args[0].to_string_lossy());
        return Err(Error::WrongArgs);
    }

    let f = BufReader::new(File::open(&args[2])?);
    let b = pannotia::container::Bitstream::read(f)?;
    // dbg!(b);

    if args[1].eq_ignore_ascii_case("bits") {
        // TODO
    } else {
        return Err(Error::InvalidMode);
    }

    let f = BufWriter::new(File::create("dump.bin")?);
    b.save(f)?;

    Ok(ExitCode::SUCCESS)
}
