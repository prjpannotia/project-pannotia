//! Tool for unpacking bitstreams

use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use bitvec::prelude::*;
use clap::{Parser, ValueEnum};

use pannotia::prelude::*;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    mode: Mode,
    bitstream: clio::Input,
    #[clap(default_value = "-")]
    output: clio::Output,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Mode {
    Raw,
    RawPng,
    Explain,
}

fn dump_raw_bits<W: Write>(b: &Bitstream, mut wr: W) -> io::Result<()> {
    let config_bits = b.family().config_bits();
    for (group, chains) in config_bits.iter().enumerate() {
        for (chain, &chain_bits) in chains.iter().enumerate() {
            writeln!(wr, "// group {group} chain {chain}")?;

            if (group, chain) != (0, 0) {
                for biti in 0..chain_bits {
                    write!(
                        wr,
                        "{}",
                        if b.get_aux_array_bit(group as u32, chain as u32, biti) {
                            "1"
                        } else {
                            "0"
                        }
                    )?;
                }
                writeln!(wr)?;
            } else {
                let (w, h) = b.family().main_logic_bits();
                for y in 0..h {
                    for x in 0..w {
                        let coord = debug::GlobalBitPos { x, y };
                        write!(
                            wr,
                            "{}",
                            if b.get_logic_array_bit(coord) {
                                "1"
                            } else {
                                "0"
                            }
                        )?;
                    }
                    writeln!(wr)?;
                }
            }

            writeln!(wr)?;
        }
    }

    Ok(())
}

fn dump_png<W: Write>(b: &Bitstream, wr: W) -> io::Result<()> {
    let (w, h) = b.family().main_logic_bits();
    let mut pngenc = png::Encoder::new(wr, w, h);
    pngenc.set_color(png::ColorType::Grayscale);
    pngenc.set_depth(png::BitDepth::One);
    let mut pngwr = pngenc.write_header()?;

    let mut row = BitVec::<u8, Msb0>::new();
    row.resize(w as usize, false);
    let row_bytes = (w as usize + 7) / 8;

    let mut all_img_data = Vec::new();
    all_img_data.reserve(row_bytes * h as usize);

    for y in 0..h {
        // XXX this is a bit slow, but we can't easily avoid it because we *do* need to mirror each row
        // (relative to the "raw" ordering slurped in from bitstream files)
        for x in 0..w {
            row.set(
                x as usize,
                b.get_logic_array_bit(debug::GlobalBitPos { y, x }),
            );
        }

        all_img_data.extend_from_slice(&row.as_raw_slice()[..row_bytes]);
    }

    pngwr.write_image_data(&all_img_data)?;
    Ok(())
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    let b = match Bitstream::read(cli.bitstream) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    };

    let out = BufWriter::new(cli.output);
    let e = match cli.mode {
        Mode::Raw => dump_raw_bits(&b, out),
        Mode::RawPng => dump_png(&b, out),
        Mode::Explain => todo!(),
    };
    match e {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
