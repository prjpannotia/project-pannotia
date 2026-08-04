//! Tool for unpacking bitstreams

use std::collections::HashMap;
use std::io::{self, BufWriter, Write};
use std::process::ExitCode;

use base64::prelude::*;
use bitvec::prelude::*;
use clap::{Parser, ValueEnum};

use pannotia::prelude::*;

use pannotia_tools::DumpTile;

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

fn dump_explain_padring<W: Write>(
    b: &Bitstream,
    mut wr: W,
    tile_pos: TilePos,
    io_i: u8,
    pad_i: u8,
) -> io::Result<()> {
    let setting = b.pad_input_en(pad_i);
    if setting {
        writeln!(wr, "pad[{}, {}].input_en = 1", tile_pos, io_i)?;
    }

    let setting = b.pad_open_drain(pad_i);
    if setting {
        writeln!(wr, "pad[{}, {}].open_drain = 1", tile_pos, io_i)?;
    }

    let setting = b.pad_reduced_slew(pad_i);
    if setting {
        writeln!(wr, "pad[{}, {}].reduced_slew = 1", tile_pos, io_i)?;
    }

    let setting = b.pad_pullup_to_fabric(pad_i);
    if setting {
        writeln!(wr, "pad[{}, {}].pullup_to_fabric = 1", tile_pos, io_i)?;
    }

    let setting = b.pad_drive_strength(pad_i);
    if setting != Default::default() {
        writeln!(
            wr,
            "pad[{}, {}].drive_strength = {}",
            tile_pos, io_i, setting
        )?;
    }

    let setting = b.pad_termination(pad_i);
    if setting != Default::default() {
        writeln!(wr, "pad[{}, {}].term = {}", tile_pos, io_i, setting)?;
    }

    Ok(())
}

fn dump_explain<W: Write>(b: &Bitstream, mut wr: W) -> io::Result<()> {
    let tile_to_padring_map = padring::PADRING_TO_TILE
        .iter()
        .enumerate()
        .map(|(pad_i, tile_pos)| (*tile_pos, pad_i as u8))
        .collect::<HashMap<_, _>>();

    writeln!(wr, "// Family: {}", b.family())?;
    writeln!(wr, "USERID 0x{:08x}", b.user_id)?;

    let (tile_w, tile_h) = b.family().tile_dims();
    for tile_y in 0..tile_h {
        for tile_x in 0..tile_w {
            let tile_pos = TilePos {
                x: tile_x,
                y: tile_y,
            };
            if let Some(tile) = b.tile(tile_pos) {
                let tile_type = tile.tile_type();

                match tile_type {
                    TileType::Logic => {
                        let tile = tile.as_logic_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;
                    }
                    TileType::RoutingOnly => {
                        let tile = tile.as_routing_only_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;
                    }
                    TileType::BRAM => {
                        let tile = tile.as_bram9k_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;

                        // Special logic for init data
                        let mut init_val: BitArr!(for 9216, in u8, Lsb0) = BitArray::ZERO;
                        tile.init_data(&mut init_val.as_mut_bitslice());
                        if init_val.any() {
                            writeln!(
                                wr,
                                "tile[{}].init_data = {}",
                                tile_pos,
                                BASE64_URL_SAFE.encode(init_val.as_raw_slice())
                            )?;
                        }
                    }
                    TileType::TopIP => {
                        let tile = tile.as_top_ip_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;
                    }
                    TileType::LeftRightIP => {
                        let tile = tile.as_leftright_ip_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;
                    }
                    TileType::TopBottomIO => {
                        let tile = tile.as_topbottom_io_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;

                        for io_i in 0..tile.num_ios() {
                            if let Some(pad_i) = tile_to_padring_map.get(&(tile.pos(), io_i)) {
                                dump_explain_padring(b, &mut wr, tile.pos(), io_i, *pad_i)?;
                            }
                        }
                    }
                    TileType::LeftRightIO => {
                        let tile = tile.as_leftright_io_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;

                        for io_i in 0..tile.num_ios() {
                            if let Some(pad_i) = tile_to_padring_map.get(&(tile.pos(), io_i)) {
                                dump_explain_padring(b, &mut wr, tile.pos(), io_i, *pad_i)?;
                            }
                        }
                    }
                    TileType::PLL => {
                        let tile = tile.as_pll_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;
                    }
                    TileType::GCLKSW => {
                        let tile = tile.as_gclksw_tile();
                        let mut tile_str = String::new();
                        tile.dump(&mut tile_str).unwrap();
                        write!(wr, "{}", tile_str)?;
                    }
                    TileType::None => {}
                    _ => writeln!(wr, "// WARN: Unimplemented tile type {:?}", tile_type)?,
                }
            }
        }
    }

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
        Mode::Explain => dump_explain(&b, out),
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
