use std::cell::RefCell;
use std::error;
use std::fmt::Display;
use std::fs::File;
use std::io::{self, BufReader};
use std::process::ExitCode;

use base64::prelude::*;
use bitvec::prelude::*;

use pannotia::coordinates::{GlobalBitPos, TilePos, TileRelativeBitPos};
use pannotia::tiles::generic_routing::{GenericRoutingRefTrait, RMUX};
use pannotia::tiles::{TileRefTrait, TileType};

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
    struct BitstreamDebugTracer {
        bit_w: usize,
        accesses: RefCell<Vec<Option<(TilePos, TileRelativeBitPos)>>>,
    }
    impl pannotia::container::DebugTracer for BitstreamDebugTracer {
        fn log_coordinate_access(
            &self,
            global_bit_pos: GlobalBitPos,
            tile_pos: TilePos,
            tile_relative_pos: TileRelativeBitPos,
        ) {
            let mut accesses = self.accesses.borrow_mut();

            if let Some((orig_tile_pos, orig_rel_pos)) =
                accesses[global_bit_pos.y as usize * self.bit_w + global_bit_pos.x as usize]
            {
                assert!(orig_tile_pos == tile_pos);
                assert!(orig_rel_pos == tile_relative_pos)
            }
            accesses[global_bit_pos.y as usize * self.bit_w + global_bit_pos.x as usize] =
                Some((tile_pos, tile_relative_pos));
        }
    }
    impl BitstreamDebugTracer {
        fn new() -> Self {
            Self {
                bit_w: 0,
                accesses: RefCell::new(Vec::new()),
            }
        }
    }
    let mut b = pannotia::container::Bitstream::read_with_debug(f, BitstreamDebugTracer::new())?;
    let (bit_w, bit_h) = b.family().main_logic_bits();
    b.debug_tracer.bit_w = bit_w as usize;
    b.debug_tracer
        .accesses
        .borrow_mut()
        .resize(bit_w as usize * bit_h as usize, None);

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
    } else if args[1].eq_ignore_ascii_case("dump") {
        let (tile_w, tile_h) = b.family().tile_dims();
        for tile_y in 0..tile_h {
            for tile_x in 0..tile_w {
                let tile_pos = TilePos {
                    x: tile_x,
                    y: tile_y,
                };
                if let Some(tile) = b.tile(tile_pos) {
                    // print generic routing
                    match tile.tile_type() {
                        TileType::Logic | TileType::RoutingOnly | TileType::BRAM => {
                            let tile = tile.as_generic_routing_tile();
                            for rmux_i in 0..96 {
                                let rmux = tile.rmux(rmux_i);
                                if rmux != RMUX::None {
                                    println!("tile[{}].rmux[{}] = {}", tile_pos, rmux_i, rmux);
                                }
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(tile) = b.tile(tile_pos) {
                    // print tile-specific stuff
                    match tile.tile_type() {
                        TileType::Logic => {
                            let tile = tile.as_logic_tile();

                            for clk_i in 0..2 {
                                let clk_mux = tile.clock_mux(clk_i);
                                if clk_mux != Default::default() {
                                    println!("tile[{}].clk[{}] = {}", tile_pos, clk_i, clk_mux);
                                }
                                let ce_mux = tile.clock_en_mux(clk_i);
                                if ce_mux != Default::default() {
                                    println!("tile[{}].ce[{}] = {}", tile_pos, clk_i, ce_mux);
                                }
                            }

                            for asy_i in 0..2 {
                                let async_mux = tile.async_mux(asy_i);
                                if async_mux != Default::default() {
                                    println!("tile[{}].async[{}] = {}", tile_pos, asy_i, async_mux);
                                }
                            }

                            let sload_mux = tile.sync_load_mux();
                            if sload_mux != Default::default() {
                                println!("tile[{}].sync_load = {}", tile_pos, sload_mux);
                            }
                            let sclr_mux = tile.sync_clr_mux();
                            if sclr_mux != Default::default() {
                                println!("tile[{}].sync_clr = {}", tile_pos, sclr_mux);
                            }

                            for glb2loc_i in 0..4 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            for ctrl_i in 0..4 {
                                let ctrl_mux = tile.control_signal_preselect(ctrl_i);
                                if ctrl_mux != Default::default() {
                                    println!("tile[{}].ctrl[{}] = {}", tile_pos, ctrl_i, ctrl_mux);
                                }
                            }

                            for lut_i in 0..16 {
                                let lut = tile.lut(lut_i);
                                if lut != 0 {
                                    println!("tile[{}].lut[{}] = 0x{:04x}", tile_pos, lut_i, lut);
                                }

                                for lut_inp_i in 0..4 {
                                    let lut_inp = tile.lut_input(lut_i, lut_inp_i);
                                    if lut_inp != Default::default() {
                                        println!(
                                            "tile[{}].lut_{}[{}] = {}",
                                            tile_pos,
                                            ["A", "B", "C", "D"][lut_inp_i as usize],
                                            lut_i,
                                            lut_inp
                                        );
                                    }
                                }

                                let lc_inp_c = tile.lc_input_c_mode(lut_i);
                                if lc_inp_c != Default::default() {
                                    println!(
                                        "tile[{}].inp_c[{}] = {:?}",
                                        tile_pos, lut_i, lc_inp_c
                                    );
                                }
                                let lc_carry_en = tile.lc_carry_en(lut_i);
                                if !lc_carry_en {
                                    println!("tile[{}].carry_en[{}] = 0", tile_pos, lut_i);
                                }

                                let lc_clk = tile.lc_clk_choice(lut_i);
                                if lc_clk != Default::default() {
                                    println!("tile[{}].lc_clk[{}] = {}", tile_pos, lut_i, lc_clk);
                                }
                                let lc_async = tile.lc_async_choice(lut_i);
                                if lc_async != Default::default() {
                                    println!(
                                        "tile[{}].lc_async[{}] = {}",
                                        tile_pos, lut_i, lc_async
                                    );
                                }
                                let lc_shift = tile.lc_shift_reg_mode(lut_i);
                                if lc_shift {
                                    println!("tile[{}].lc_shift[{}] = 1", tile_pos, lut_i);
                                }
                                let lc_bypass = tile.lc_input_c_bypass_mode(lut_i);
                                if lc_bypass {
                                    println!("tile[{}].lc_bypass[{}] = 1", tile_pos, lut_i);
                                }

                                for out_i in 0..3 {
                                    let outp = tile.lc_output(lut_i, out_i);
                                    if outp != Default::default() {
                                        println!(
                                            "tile[{}].omux{}[{}] = {}",
                                            tile_pos, out_i, lut_i, outp
                                        );
                                    }
                                }
                            }
                        }
                        TileType::RoutingOnly => {
                            let tile = tile.as_routing_only_tile();

                            for glb2loc_i in 0..4 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            for lut_i in 0..16 {
                                let outp = tile.right_neighbor_output(lut_i);
                                if outp != Default::default() {
                                    println!("tile[{}].omux[{}] = {}", tile_pos, lut_i, outp);
                                }
                            }
                        }
                        TileType::BRAM => {
                            let tile = tile.as_bram9k_tile();

                            for clk_i in 0..2 {
                                let clk_mux = tile.clock_mux(clk_i);
                                if clk_mux != Default::default() {
                                    println!("tile[{}].clk[{}] = {}", tile_pos, clk_i, clk_mux);
                                }
                                let ce_mux = tile.clock_en_mux(clk_i);
                                if ce_mux != Default::default() {
                                    println!("tile[{}].ce[{}] = {}", tile_pos, clk_i, ce_mux);
                                }
                            }

                            for asy_i in 0..2 {
                                let async_mux = tile.async_mux(asy_i);
                                if async_mux != Default::default() {
                                    println!("tile[{}].async[{}] = {}", tile_pos, asy_i, async_mux);
                                }
                            }

                            for glb2loc_i in 0..6 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            for ctrl_i in 0..4 {
                                let ctrl_mux = tile.control_signal_preselect(ctrl_i);
                                if ctrl_mux != Default::default() {
                                    println!("tile[{}].ctrl[{}] = {}", tile_pos, ctrl_i, ctrl_mux);
                                }
                            }

                            for addr_biti in 0..13 {
                                let addr_mux = tile.addr_a(addr_biti);
                                if addr_mux != Default::default() {
                                    println!(
                                        "tile[{}].addr_a[{}] = {}",
                                        tile_pos, addr_biti, addr_mux
                                    );
                                }
                            }
                            for addr_biti in 0..13 {
                                let addr_mux = tile.addr_b(addr_biti);
                                if addr_mux != Default::default() {
                                    println!(
                                        "tile[{}].addr_b[{}] = {}",
                                        tile_pos, addr_biti, addr_mux
                                    );
                                }
                            }
                            for data_biti in 0..18 {
                                let data_mux = tile.data_in_a(data_biti);
                                if data_mux != Default::default() {
                                    println!(
                                        "tile[{}].data_in_a[{}] = {}",
                                        tile_pos, data_biti, data_mux
                                    );
                                }
                            }
                            for data_biti in 0..18 {
                                let data_mux = tile.data_in_b(data_biti);
                                if data_mux != Default::default() {
                                    println!(
                                        "tile[{}].data_in_b[{}] = {}",
                                        tile_pos, data_biti, data_mux
                                    );
                                }
                            }
                            for imux_xtra_i in 0..2 {
                                let imux_xtra = tile.imux_xtra(imux_xtra_i);
                                if imux_xtra != Default::default() {
                                    println!(
                                        "tile[{}].imux_xtra[{}] = {}",
                                        tile_pos, imux_xtra_i, imux_xtra
                                    );
                                }
                            }

                            for tmux_i in 0..16 {
                                let tmux = tile.tmux(tmux_i);
                                if tmux != Default::default() {
                                    println!("tile[{}].tmux[{}] = {}", tile_pos, tmux_i, tmux);
                                }
                            }

                            let kmux = tile.read_en_a();
                            if kmux != Default::default() {
                                println!("tile[{}].read_en_a = {}", tile_pos, kmux);
                            }
                            let kmux = tile.read_en_b();
                            if kmux != Default::default() {
                                println!("tile[{}].read_en_b = {}", tile_pos, kmux);
                            }
                            let kmux = tile.write_en_a();
                            if kmux != Default::default() {
                                println!("tile[{}].write_en_a = {}", tile_pos, kmux);
                            }
                            let kmux = tile.write_en_b();
                            if kmux != Default::default() {
                                println!("tile[{}].write_en_b = {}", tile_pos, kmux);
                            }
                            let kmux = tile.addr_stall_a();
                            if kmux != Default::default() {
                                println!("tile[{}].addr_stall_a = {}", tile_pos, kmux);
                            }
                            let kmux = tile.addr_stall_b();
                            if kmux != Default::default() {
                                println!("tile[{}].addr_stall_b = {}", tile_pos, kmux);
                            }
                            for bit in 0..2 {
                                let kmux = tile.byte_en_a(bit);
                                if kmux != Default::default() {
                                    println!("tile[{}].byte_en_a[{}] = {}", tile_pos, bit, kmux);
                                }
                            }
                            for bit in 0..2 {
                                let kmux = tile.byte_en_b(bit);
                                if kmux != Default::default() {
                                    println!("tile[{}].byte_en_b[{}] = {}", tile_pos, bit, kmux);
                                }
                            }
                            for kmux_i in 10..16 {
                                let kmux = tile.kmux(kmux_i);
                                if kmux != Default::default() {
                                    println!("tile[{}].kmux[{}] = {}", tile_pos, kmux_i, kmux);
                                }
                            }

                            // settings for the RAM itself
                            let cfg_setting = tile.use_packed_mode_address_override();
                            if cfg_setting {
                                println!("tile[{}].use_packed_mode_address_override = 1", tile_pos);
                            }
                            let cfg_setting = tile.clock_choices_mode();
                            if cfg_setting != Default::default() {
                                println!("tile[{}].clock_choices_mode = {}", tile_pos, cfg_setting);
                            }

                            let cfg_setting = tile.width_a();
                            if cfg_setting != Default::default() {
                                println!("tile[{}].width_a = {}", tile_pos, cfg_setting);
                            }
                            let cfg_setting = tile.width_b();
                            if cfg_setting != Default::default() {
                                println!("tile[{}].width_b = {}", tile_pos, cfg_setting);
                            }

                            let cfg_setting = tile.use_output_register_a();
                            if cfg_setting {
                                println!("tile[{}].use_output_register_a = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_output_register_b();
                            if cfg_setting {
                                println!("tile[{}].use_output_register_b = 1", tile_pos);
                            }

                            let cfg_setting = tile.use_rst_in_a();
                            if cfg_setting {
                                println!("tile[{}].use_rst_in_a = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_rst_in_b();
                            if cfg_setting {
                                println!("tile[{}].use_rst_in_b = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_rst_out_a();
                            if cfg_setting {
                                println!("tile[{}].use_rst_out_a = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_rst_out_b();
                            if cfg_setting {
                                println!("tile[{}].use_rst_out_b = 1", tile_pos);
                            }

                            let cfg_setting = tile.use_clk_en_in_a();
                            if cfg_setting {
                                println!("tile[{}].use_clk_en_in_a = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_clk_en_in_b();
                            if cfg_setting {
                                println!("tile[{}].use_clk_en_in_b = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_clk_en_out_a();
                            if cfg_setting {
                                println!("tile[{}].use_clk_en_out_a = 1", tile_pos);
                            }
                            let cfg_setting = tile.use_clk_en_out_b();
                            if cfg_setting {
                                println!("tile[{}].use_clk_en_out_b = 1", tile_pos);
                            }

                            let cfg_setting = tile.write_thru_a();
                            if cfg_setting {
                                println!("tile[{}].write_thru_a = 1", tile_pos);
                            }
                            let cfg_setting = tile.write_thru_b();
                            if cfg_setting {
                                println!("tile[{}].write_thru_b = 1", tile_pos);
                            }

                            let cfg_setting = tile.rsen_delay();
                            if cfg_setting != 0 {
                                println!("tile[{}].rsen_delay = {}", tile_pos, cfg_setting);
                            }
                            let cfg_setting = tile.delay_time();
                            if cfg_setting != 0 {
                                println!("tile[{}].delay_time = {}", tile_pos, cfg_setting);
                            }

                            let mut init_val: BitArr!(for 9216, in u8, Lsb0) = BitArray::ZERO;
                            tile.init_data(&mut init_val.as_mut_bitslice());
                            if init_val.any() {
                                println!(
                                    "tile[{}].init_data = {}",
                                    tile_pos,
                                    BASE64_URL_SAFE.encode(init_val.as_raw_slice())
                                );
                            }
                        }
                        TileType::TopIP => {
                            let tile = tile.as_top_ip_tile();

                            for glb2loc_i in 0..12 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            for to_ip_i in 0..12 {
                                let to_ip = tile.to_ip(to_ip_i);
                                if to_ip != Default::default() {
                                    println!("tile[{}].to_ip[{}] = {}", tile_pos, to_ip_i, to_ip);
                                }
                            }

                            for from_ip_i in 0..12 {
                                let from_ip = tile.from_ip(from_ip_i);
                                if from_ip != Default::default() {
                                    println!(
                                        "tile[{}].from_ip[{}] = {}",
                                        tile_pos, from_ip_i, from_ip
                                    );
                                }
                            }
                        }
                        TileType::LeftRightIP => {
                            let tile = tile.as_leftright_ip_tile();

                            for glb2loc_i in 0..20 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            for to_ip_i in 0..12 {
                                let to_ip = tile.to_ip_13(to_ip_i);
                                if to_ip != Default::default() {
                                    println!("tile[{}].to_ip[{}] = {}", tile_pos, to_ip_i, to_ip);
                                }
                            }
                            for to_ip_i in 0..8 {
                                let to_ip = tile.to_ip_17(to_ip_i);
                                if to_ip != Default::default() {
                                    println!(
                                        "tile[{}].to_ip[{}] = {}",
                                        tile_pos,
                                        12 + to_ip_i,
                                        to_ip
                                    );
                                }
                            }

                            for from_ip_i in 0..12 {
                                let from_ip = tile.from_ip(from_ip_i);
                                if from_ip != Default::default() {
                                    println!(
                                        "tile[{}].from_ip[{}] = {}",
                                        tile_pos, from_ip_i, from_ip
                                    );
                                }
                            }
                        }
                        TileType::TopBottomIO => {
                            let tile = tile.as_topbottom_io_tile();

                            for local_line_i in 0..32 {
                                let local_line = tile.local_line(local_line_i);
                                if local_line != Default::default() {
                                    println!(
                                        "tile[{}].local_line[{}] = {}",
                                        tile_pos, local_line_i, local_line
                                    );
                                }
                            }

                            for glb2loc_i in 0..8 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            for loc2clk_i in 0..8 {
                                let loc2clk_mux = tile.local_to_clock(loc2clk_i);
                                if loc2clk_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc2clk[{}] = {}",
                                        tile_pos, loc2clk_i, loc2clk_mux
                                    );
                                }
                            }
                        }
                        TileType::LeftRightIO => {
                            let tile = tile.as_leftright_io_tile();

                            for local_line_i in 0..48 {
                                let local_line = tile.local_line(local_line_i);
                                if local_line != Default::default() {
                                    println!(
                                        "tile[{}].local_line[{}] = {}",
                                        tile_pos, local_line_i, local_line
                                    );
                                }
                            }

                            for glb2loc_i in 0..12 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }
                        }
                        // TileType::PLL => todo!(),
                        // TileType::GCLKSW => todo!(),
                        tile_type => {
                            println!("// WARN: Unimplemented tile type {:?}", tile_type);
                        }
                    }
                }
            }
        }

        println!("\n// access mask");
        let (bit_w, bit_h) = b.family().main_logic_bits();
        let accesses = b.debug_tracer.accesses.borrow();
        for y in 0..bit_h {
            print!("// ");
            for x in 0..bit_w {
                if accesses[y as usize * bit_w as usize + x as usize].is_some() {
                    print!("*");
                } else {
                    print!(" ");
                }
            }
            println!();
        }
        // b.tile(123, 456).unwrap().as_logic_tile().lut();
        // b.tile_mut(123, 456).unwrap().as_logic_tile().set_lut(123);
    } else {
        return Err(Error::InvalidMode);
    }

    Ok(ExitCode::SUCCESS)
}
