use std::cell::RefCell;
use std::error;
use std::fmt::{Display, Write};
use std::fs::File;
use std::io::{self, BufReader};
use std::process::ExitCode;

use base64::prelude::*;
use bitvec::prelude::*;

use pannotia::prelude::debug::*;
use pannotia::prelude::*;
use routedb::{FunctionInputSource, RMUXSource};

#[derive(Debug)]
pub enum Error {
    WrongArgs,
    InvalidMode,
    IoError(io::Error),
    BitstreamContainerError(BitstreamContainerError),
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
impl From<BitstreamContainerError> for Error {
    fn from(value: BitstreamContainerError) -> Self {
        Self::BitstreamContainerError(value)
    }
}

struct BitstreamDebugTracer {
    bit_w: usize,
    accesses: RefCell<Vec<Option<(TilePos, TileRelativeBitPos, String)>>>,
}
impl DebugTracer for BitstreamDebugTracer {
    fn log_coordinate_access(
        &self,
        global_bit_pos: GlobalBitPos,
        tile_pos: TilePos,
        tile_relative_pos: TileRelativeBitPos,
        field: &dyn std::fmt::Debug,
    ) {
        let mut accesses = self.accesses.borrow_mut();

        if let Some((orig_tile_pos, orig_rel_pos, orig_field)) =
            &accesses[global_bit_pos.y as usize * self.bit_w + global_bit_pos.x as usize]
        {
            assert_eq!(*orig_tile_pos, tile_pos);
            assert_eq!(*orig_rel_pos, tile_relative_pos);
            assert_eq!(orig_field.as_str(), format!("{:?}", field));
        }
        accesses[global_bit_pos.y as usize * self.bit_w + global_bit_pos.x as usize] =
            Some((tile_pos, tile_relative_pos, format!("{:?}", field)));
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

trait DumpTile {
    fn dump(&self);
}

fn main() -> Result<ExitCode, Error> {
    env_logger::init();
    let args = std::env::args_os().collect::<Vec<_>>();

    if args.len() < 3 {
        println!("Usage: {} dump_mode file.bin", args[0].to_string_lossy());
        return Err(Error::WrongArgs);
    }

    let f = BufReader::new(File::open(&args[2])?);
    let mut b = Bitstream::read_with_debug(f, BitstreamDebugTracer::new())?;
    let (bit_w, bit_h) = b.family().main_logic_bits();
    b.debug_tracer.bit_w = bit_w as usize;
    b.debug_tracer
        .accesses
        .borrow_mut()
        .resize(bit_w as usize * bit_h as usize, None);

    if args[1].eq_ignore_ascii_case("bits") {
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
                            tile.dump();
                        }
                        _ => {}
                    }
                }

                if let Some(tile) = b.tile(tile_pos) {
                    // print tile-specific stuff
                    match tile.tile_type() {
                        TileType::Logic => {
                            let tile = tile.as_logic_tile();
                        }
                        TileType::RoutingOnly => {
                            let tile = tile.as_routing_only_tile();
                        }
                        TileType::BRAM => {
                            let tile = tile.as_bram9k_tile();
                        }
                        TileType::TopIP => {
                            let tile = tile.as_top_ip_tile();
                        }
                        TileType::LeftRightIP => {
                            let tile = tile.as_leftright_ip_tile();
                        }
                        TileType::TopBottomIO | TileType::LeftRightIO => {
                            let tile_topbottom;
                            let tile_leftright;
                            let tile: &dyn IOTileCommon =
                                if tile.tile_type() == TileType::TopBottomIO {
                                    tile_topbottom = tile.as_topbottom_io_tile();
                                    let tile = &tile_topbottom;

                                    for local_line_i in 0..32 {
                                        let local_line = tile.local_line(local_line_i);
                                        if local_line != Default::default() {
                                            println!(
                                                "tile[{}].local_line[{}] = {}",
                                                tile_pos, local_line_i, local_line
                                            );
                                        }
                                    }

                                    tile
                                } else {
                                    tile_leftright = tile.as_leftright_io_tile();
                                    let tile = &tile_leftright;

                                    for local_line_i in 0..48 {
                                        let local_line = tile.local_line(local_line_i);
                                        if local_line != Default::default() {
                                            println!(
                                                "tile[{}].local_line[{}] = {}",
                                                tile_pos, local_line_i, local_line
                                            );
                                        }
                                    }

                                    tile
                                };

                            for io_i in 0..tile.num_ios() {
                                let glb2loc_mux = tile.out_clock_global_to_local(io_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].out_glb2loc[{}] = {}",
                                        tile_pos, io_i, glb2loc_mux
                                    );
                                }
                                let loc2clk_mux = tile.out_clock_local_to_clock(io_i);
                                if loc2clk_mux != Default::default() {
                                    println!(
                                        "tile[{}].out_loc2clk[{}] = {}",
                                        tile_pos, io_i, loc2clk_mux
                                    );
                                }
                                let clkmux = tile.out_clock_choice(io_i);
                                if clkmux != Default::default() {
                                    println!("tile[{}].out_clk[{}] = {}", tile_pos, io_i, clkmux);
                                }
                                let setting = tile.out_use_reg(io_i);
                                if setting {
                                    println!("tile[{}].out_use_reg[{}] = 1", tile_pos, io_i);
                                }
                                let setting = tile.out_async_mode(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].out_async_mode[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.out_sync_mode(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].out_sync_mode[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.out_powerup_state(io_i);
                                if setting {
                                    println!("tile[{}].out_powerup_state[{}] = 1", tile_pos, io_i);
                                }

                                let setting = tile.oe_use_reg(io_i);
                                if setting {
                                    println!("tile[{}].oe_use_reg[{}] = 1", tile_pos, io_i);
                                }
                                let setting = tile.oe_async_mode(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].oe_async_mode[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.oe_sync_mode(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].oe_sync_mode[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.oe_powerup_state(io_i);
                                if setting {
                                    println!("tile[{}].oe_powerup_state[{}] = 1", tile_pos, io_i);
                                }

                                let glb2loc_mux = tile.in_clock_global_to_local(io_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].in_glb2loc[{}] = {}",
                                        tile_pos, io_i, glb2loc_mux
                                    );
                                }
                                let loc2clk_mux = tile.in_clock_local_to_clock(io_i);
                                if loc2clk_mux != Default::default() {
                                    println!(
                                        "tile[{}].in_loc2clk[{}] = {}",
                                        tile_pos, io_i, loc2clk_mux
                                    );
                                }
                                let clkmux = tile.in_clock_choice(io_i);
                                if clkmux != Default::default() {
                                    println!("tile[{}].in_clk[{}] = {}", tile_pos, io_i, clkmux);
                                }
                                let setting = tile.in_async_mode(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].in_async_mode[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.in_sync_mode(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].in_sync_mode[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.in_powerup_state(io_i);
                                if setting {
                                    println!("tile[{}].in_powerup_state[{}] = 1", tile_pos, io_i);
                                }

                                let loc2io_mux = tile.local_to_io_out(io_i);
                                if loc2io_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc_to_io_out[{}] = {}",
                                        tile_pos, io_i, loc2io_mux
                                    );
                                }
                                let loc2io_mux = tile.local_to_io_oe(io_i);
                                if loc2io_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc_to_io_oe[{}] = {}",
                                        tile_pos, io_i, loc2io_mux
                                    );
                                }
                                let loc2io_mux = tile.local_to_out_clk_en(io_i);
                                if loc2io_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc_to_out_cen[{}] = {}",
                                        tile_pos, io_i, loc2io_mux
                                    );
                                }
                                let loc2io_mux = tile.local_to_in_clk_en(io_i);
                                if loc2io_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc_to_in_cen[{}] = {}",
                                        tile_pos, io_i, loc2io_mux
                                    );
                                }
                                let loc2io_mux = tile.local_to_async_ctrl(io_i);
                                if loc2io_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc_to_async[{}] = {}",
                                        tile_pos, io_i, loc2io_mux
                                    );
                                }
                                let loc2io_mux = tile.local_to_sync_ctrl(io_i);
                                if loc2io_mux != Default::default() {
                                    println!(
                                        "tile[{}].loc_to_sync[{}] = {}",
                                        tile_pos, io_i, loc2io_mux
                                    );
                                }

                                for out_i in 0..2 {
                                    let outp = tile.out_mux(io_i, out_i);
                                    if outp != Default::default() {
                                        println!(
                                            "tile[{}].omux{}[{}] = {}",
                                            tile_pos, out_i, io_i, outp
                                        );
                                    }
                                }

                                let setting = tile.in_data_delay(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].in_data_delay[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.in_reg_delay(io_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].in_reg_delay[{}] = {}",
                                        tile_pos, io_i, setting
                                    );
                                }
                                let setting = tile.out_delay(io_i);
                                if setting {
                                    println!("tile[{}].out_delay[{}] = 1", tile_pos, io_i);
                                }
                            }
                        }
                        TileType::PLL => {
                            let tile = tile.as_pll_tile();

                            for to_pll_i in 0..11 {
                                let to_pll = tile.to_pll(to_pll_i);
                                if to_pll != Default::default() {
                                    println!(
                                        "tile[{}].to_pll[{}] = {}",
                                        tile_pos, to_pll_i, to_pll
                                    );
                                }
                            }

                            for glb2loc_i in 0..11 {
                                let glb2loc_mux = tile.global_to_local(glb2loc_i);
                                if glb2loc_mux != Default::default() {
                                    println!(
                                        "tile[{}].glb2loc[{}] = {}",
                                        tile_pos, glb2loc_i, glb2loc_mux
                                    );
                                }
                            }

                            let setting = tile.gclk_mux();
                            if setting != Default::default() {
                                println!("tile[{}].gclk_mux = {}", tile_pos, setting);
                            }
                            let setting = tile.clock_mux_0();
                            if setting != Default::default() {
                                println!("tile[{}].clock_mux_0 = {}", tile_pos, setting);
                            }
                            let setting = tile.in_div_lo_time();
                            if setting != Default::default() {
                                println!("tile[{}].in_div_lo_time = {}", tile_pos, setting);
                            }
                            let setting = tile.in_div_hi_time();
                            if setting != Default::default() {
                                println!("tile[{}].in_div_hi_time = {}", tile_pos, setting);
                            }
                            let setting = tile.in_div_duty_cycle_adjust();
                            if setting {
                                println!("tile[{}].in_div_duty_cycle_adjust = 1", tile_pos,);
                            }
                            let setting = tile.in_div_bypass();
                            if setting {
                                println!("tile[{}].in_div_bypass = 1", tile_pos,);
                            }

                            let setting = tile.clock_feedback_mux();
                            if setting != Default::default() {
                                println!("tile[{}].clock_feedback_mux = {}", tile_pos, setting);
                            }
                            let setting = tile.use_internal_fb();
                            if setting {
                                println!("tile[{}].use_internal_fb = 1", tile_pos,);
                            }
                            let setting = tile.feedback_delay();
                            if setting != Default::default() {
                                println!("tile[{}].feedback_delay = {}", tile_pos, setting);
                            }
                            let setting = tile.fb_div_lo_time();
                            if setting != Default::default() {
                                println!("tile[{}].fb_div_lo_time = {}", tile_pos, setting);
                            }
                            let setting = tile.fb_div_hi_time();
                            if setting != Default::default() {
                                println!("tile[{}].fb_div_hi_time = {}", tile_pos, setting);
                            }
                            let setting = tile.fb_div_duty_cycle_adjust();
                            if setting {
                                println!("tile[{}].fb_div_duty_cycle_adjust = 1", tile_pos,);
                            }
                            let setting = tile.fb_div_bypass();
                            if setting {
                                println!("tile[{}].fb_div_bypass = 1", tile_pos,);
                            }
                            let setting = tile.fb_phase_coarse();
                            if setting != Default::default() {
                                println!("tile[{}].fb_phase_coarse = {}", tile_pos, setting);
                            }
                            let setting = tile.fb_phase_fine();
                            if setting != Default::default() {
                                println!("tile[{}].fb_phase_fine = {}", tile_pos, setting);
                            }

                            for out_i in 0..5 {
                                let setting = tile.out_enable(out_i);
                                if setting {
                                    println!("tile[{}].out_enable[{}] = 1", tile_pos, out_i,);
                                }
                                if out_i != 0 {
                                    let setting = tile.out_cascade(out_i);
                                    if setting {
                                        println!("tile[{}].out_cascade[{}] = 1", tile_pos, out_i,);
                                    }
                                }

                                let setting = tile.out_div_lo_time(out_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].out_div_lo_time[{}] = {}",
                                        tile_pos, out_i, setting
                                    );
                                }
                                let setting = tile.out_div_hi_time(out_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].out_div_hi_time[{}] = {}",
                                        tile_pos, out_i, setting
                                    );
                                }
                                let setting = tile.out_div_duty_cycle_adjust(out_i);
                                if setting {
                                    println!(
                                        "tile[{}].out_div_duty_cycle_adjust[{}] = 1",
                                        tile_pos, out_i
                                    );
                                }
                                let setting = tile.out_div_bypass(out_i);
                                if setting {
                                    println!("tile[{}].out_div_bypass[{}] = 1", tile_pos, out_i);
                                }

                                let setting = tile.out_phase_coarse(out_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].out_phase_coarse[{}] = {}",
                                        tile_pos, out_i, setting
                                    );
                                }
                                let setting = tile.out_phase_fine(out_i);
                                if setting != Default::default() {
                                    println!(
                                        "tile[{}].out_phase_fine[{}] = {}",
                                        tile_pos, out_i, setting
                                    );
                                }
                            }

                            let setting = tile.vco_div2();
                            if setting {
                                println!("tile[{}].vco_div2 = 1", tile_pos,);
                            }

                            let setting = tile.reg_ctrl();
                            if setting != Default::default() {
                                println!("tile[{}].reg_ctrl = {}", tile_pos, setting);
                            }
                            let setting = tile.enabled();
                            if setting {
                                println!("tile[{}].enabled = 1", tile_pos,);
                            }
                            let setting = tile.enable_dedicated_out_n();
                            if setting {
                                println!("tile[{}].enable_dedicated_out_n = 1", tile_pos,);
                            }
                            let setting = tile.enable_dedicated_out_p();
                            if setting {
                                println!("tile[{}].enable_dedicated_out_p = 1", tile_pos,);
                            }

                            let setting = tile.analog_icp();
                            if setting != Default::default() {
                                println!("tile[{}].analog_icp = {}", tile_pos, setting);
                            }
                            let setting = tile.analog_rlpf();
                            if setting != Default::default() {
                                println!("tile[{}].analog_rlpf = {}", tile_pos, setting);
                            }
                            let setting = tile.analog_rref();
                            if setting != Default::default() {
                                println!("tile[{}].analog_rref = {}", tile_pos, setting);
                            }
                            let setting = tile.analog_rvi();
                            if setting != Default::default() {
                                println!("tile[{}].analog_rvi = {}", tile_pos, setting);
                            }
                            let setting = tile.analog_ivco();
                            if setting != Default::default() {
                                println!("tile[{}].analog_ivco = {}", tile_pos, setting);
                            }

                            // TODO: the PLL's "actual" attributes
                        }
                        TileType::GCLKSW => {
                            let tile = tile.as_gclksw_tile();

                            for fab2clk_i in 0..6 {
                                let fab2clk = tile.fabric_to_clock(fab2clk_i);
                                if fab2clk != Default::default() {
                                    println!(
                                        "tile[{}].fab2clk[{}] = {}",
                                        tile_pos, fab2clk_i, fab2clk
                                    );
                                }
                            }

                            for ce_i in 0..6 {
                                let ce = tile.clock_enable(ce_i);
                                if ce != Default::default() {
                                    println!("tile[{}].ce[{}] = {}", tile_pos, ce_i, ce);
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

                            for out_i in 0..4 {
                                let outp = tile.clock_to_fabric(out_i);
                                if outp != Default::default() {
                                    println!("tile[{}].clk2fab[{}] = {}", tile_pos, out_i, outp);
                                }
                            }

                            for clk_i in 0..6 {
                                let cen_reg = tile.cen_is_registered(clk_i);
                                if cen_reg {
                                    println!("tile[{}].clk2fab[{}] = 1", tile_pos, clk_i);
                                }
                            }

                            for clk_i in 0..5 {
                                let dmux = tile.clock_dist_mux(clk_i);
                                if dmux != Default::default() {
                                    println!(
                                        "tile[{}].clock_dist_mux[{}] = {}",
                                        tile_pos, clk_i, dmux
                                    );
                                }
                            }
                        }
                        tile_type => {
                            println!("// WARN: Unimplemented tile type {:?}", tile_type);
                        }
                    }
                }
            }
        }

        println!();
        for pad_i in 0..79 {
            let setting = b.pad_input_en(pad_i);
            if setting {
                println!("pad[{}].input_en = 1", pad_i);
            }

            let setting = b.pad_open_drain(pad_i);
            if setting {
                println!("pad[{}].open_drain = 1", pad_i);
            }

            let setting = b.pad_reduced_slew(pad_i);
            if setting {
                println!("pad[{}].reduced_slew = 1", pad_i);
            }

            let setting = b.pad_pullup_to_fabric(pad_i);
            if setting {
                println!("pad[{}].pullup_to_fabric = 1", pad_i);
            }

            let setting = b.pad_drive_strength(pad_i);
            if setting != Default::default() {
                println!("pad[{}].drive_strength = {}", pad_i, setting);
            }

            let setting = b.pad_termination(pad_i);
            if setting != Default::default() {
                println!("pad[{}].term = {}", pad_i, setting);
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
    } else {
        return Err(Error::InvalidMode);
    }

    Ok(ExitCode::SUCCESS)
}
