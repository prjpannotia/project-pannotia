use std::collections::HashMap;
use std::io::{BufRead, BufReader};
use std::process::ExitCode;

use clap::Parser;

use pannotia::prelude::*;
use pannotia_tools::PackerParse;

#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    family: String,
    textfile: clio::Input,
    output: clio::Output,
}

fn try_parse_field(f: &str) -> Result<(&str, u8), ()> {
    if let Some(bracket_idx) = f.find("[") {
        let (field, mut rest) = f.split_at(bracket_idx);

        if let Some(rest_) = rest.strip_suffix("]") {
            rest = rest_;
        }

        Ok((
            field,
            u8::from_str_radix(rest.trim_ascii(), 10).map_err(|_| {})?,
        ))
    } else {
        Ok((f, 0))
    }
}

fn try_parse_line(
    b: &mut Bitstream,
    l: &str,
    tile_to_padring_map: &HashMap<(TilePos, u8), u8>,
) -> Result<(), ()> {
    let l_ = l.trim_ascii();
    if l_.len() == 0 || l_.starts_with("//") {
        return Ok(());
    }

    if let Some(userid) = l_.strip_prefix("USERID ") {
        let userid = parse_int::parse::<u32>(userid.trim_ascii()).map_err(|_| {})?;
        b.user_id = userid;
    } else {
        let (thing, val) = l_.split_once("=").ok_or(())?;
        let thing = thing.trim_ascii();
        let mut val = val.trim_ascii();

        if let Some(comment_idx) = val.find("//") {
            val = val[..comment_idx].trim_ascii();
        }

        if thing.starts_with("tile[") {
            //
        } else if let Some(rest) = thing.strip_prefix("pad[") {
            let (coord, field) = rest.split_once("].").ok_or(())?;
            let coord = coord.split(",").collect::<Vec<_>>();
            if coord.len() != 3 {
                return Err(());
            }
            let x = u32::from_str_radix(coord[0].trim_ascii(), 10).map_err(|_| {})?;
            let y = u32::from_str_radix(coord[1].trim_ascii(), 10).map_err(|_| {})?;
            let n = u8::from_str_radix(coord[2].trim_ascii(), 10).map_err(|_| {})?;

            let pad_i = *tile_to_padring_map.get(&(TilePos { x, y }, n)).ok_or(())?;
            let (field, _field_idx) = try_parse_field(field.trim_ascii())?;

            match field {
                "input_en" => {
                    b.set_pad_input_en(pad_i, PackerParse::try_parse(val)?);
                }
                "open_drain" => {
                    b.set_pad_open_drain(pad_i, PackerParse::try_parse(val)?);
                }
                "reduced_slew" => {
                    b.set_pad_reduced_slew(pad_i, PackerParse::try_parse(val)?);
                }
                "pullup_to_fabric" => {
                    b.set_pad_pullup_to_fabric(pad_i, PackerParse::try_parse(val)?);
                }
                _ => return Err(()),
            }
        } else {
            return Err(());
        }
    }

    Ok(())
}

fn main() -> ExitCode {
    env_logger::init();

    let cli = Cli::parse();

    let family = if let Ok(family) = Family::try_from(cli.family.as_str()) {
        family
    } else {
        eprintln!("invalid family");
        return ExitCode::FAILURE;
    };

    let tile_to_padring_map = padring::PADRING_TO_TILE
        .iter()
        .enumerate()
        .map(|(pad_i, tile_pos)| (*tile_pos, pad_i as u8))
        .collect::<HashMap<_, _>>();

    let mut b = Bitstream::new(family);

    let r = BufReader::new(cli.textfile);
    for l in r.lines() {
        match l {
            Ok(l) => {
                if try_parse_line(&mut b, &l, &tile_to_padring_map).is_err() {
                    eprintln!("syntax error: {}", l);
                    return ExitCode::FAILURE;
                }
            }
            Err(e) => {
                eprintln!("{}", e);
                return ExitCode::FAILURE;
            }
        }
    }

    let e = b.save(cli.output);
    match e {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{}", e);
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}
