use std::error;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader};
use std::process::ExitCode;

use pannotia::coordinates::TilePos;
use pannotia::tiles::TileRefTrait;

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
    let mut b = pannotia::container::Bitstream::read(f)?;

    if args[1].eq_ignore_ascii_case("bits") {
        let config_bits = b.family().config_bits();
        for (group, chains) in config_bits.iter().enumerate() {
            for (chain, &chain_bits) in chains.iter().enumerate() {
                println!("// group {group} chain {chain}");

                if (group, chain) != (0, 0) {
                    for biti in 0..chain_bits {
                        print!(
                            "{}",
                            if b.get_aux_array_bit(group as u32, chain as u32, biti) {
                                "1"
                            } else {
                                "0"
                            }
                        );
                    }
                    println!();
                } else {
                    let (w, h) = b.family().main_logic_bits();
                    for y in 0..h {
                        for x in 0..w {
                            let coord = pannotia::coordinates::GlobalBitPos { x, y };
                            print!(
                                "{}",
                                if b.get_logic_array_bit(coord) {
                                    "1"
                                } else {
                                    "0"
                                }
                            );
                        }
                        println!();
                    }
                }

                println!("");
            }
        }
    } else if args[1].eq_ignore_ascii_case("debug_tile_grid") {
        let (tile_w, tile_h) = b.family().tile_dims();
        for tile_y in 0..tile_h {
            for tile_x in 0..tile_w {
                let tile_pos = TilePos {
                    x: tile_x,
                    y: tile_y,
                };
                if let Some(tile) = b.tile(tile_pos) {
                    println!("tile {}: {:?}", tile_pos, tile.tile_type());
                }
            }
        }
        // b.tile(123, 456).unwrap().as_logic_tile().lut();
        // b.tile_mut(123, 456).unwrap().as_logic_tile().set_lut(123);
    } else {
        return Err(Error::InvalidMode);
    }

    Ok(ExitCode::SUCCESS)
}
