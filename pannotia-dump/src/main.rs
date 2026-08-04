use std::cell::RefCell;
use std::error;
use std::fmt::{Display, Write};
use std::fs::File;
use std::io::{self, BufReader};
use std::process::ExitCode;

use base64::prelude::*;
use bitvec::prelude::*;

use pannotia::coordinates::{GlobalBitPos, TilePos, TileRelativeBitPos};
use pannotia::padring::PadRingExt;
use pannotia::routedb::{Direction, FunctionInputSource, RMUXSource, RoutingWire};
use pannotia::tiles::generic_routing::{GenericRoutingRefTrait, RMUX};
use pannotia::tiles::io::IOTileCommon;
use pannotia::tiles::local_lines::IMUX;
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

struct PrettyPrintWrap<T>(T);

trait PrettyPrintRMUX {
    fn pretty_print(
        &self,
        family: pannotia::chips::Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> String;
}
impl PrettyPrintRMUX for &PrettyPrintWrap<RMUX> {
    fn pretty_print(
        &self,
        family: pannotia::chips::Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> String {
        let mut ret = String::new();

        if let RMUX::I(rmux_inp_i) = self.0 {
            write!(ret, "{}\t// ", self.0).unwrap();

            let this_rmux = pannotia::routedb::RMUX_PURPOSE[i as usize];
            match this_rmux {
                pannotia::routedb::RMUXPurpose::SelfWire => {
                    write!(ret, "rmux_self[{}]", i / 6 * 2 + i % 6 - 4)
                }
                pannotia::routedb::RMUXPurpose::LeftNeighbor => {
                    write!(ret, "T1_W[{}]", i / 6)
                }
                pannotia::routedb::RMUXPurpose::Span4 {
                    going_dir,
                    wire_idx,
                } => write!(ret, "T4_{}[{}]", going_dir, wire_idx),
            }
            .unwrap();

            write!(ret, " = ").unwrap();

            let rmux_src =
                pannotia::routedb::rmux_input(i, rmux_inp_i, tile_type == TileType::BRAM);
            match rmux_src {
                RMUXSource::GlobalToLocal(i) => {
                    write!(ret, "glb2loc[{i}]")
                }
                RMUXSource::RMUX(i) => {
                    write!(ret, "rmux[{i}]")
                }
                RMUXSource::CellOutput(i) => {
                    write!(ret, "this_output[{i}]")
                }
                RMUXSource::RoutingWire(src_wire) => {
                    let abs_wire = src_wire.to_absolute(family, tile_pos);
                    write!(
                        ret,
                        "tile[{}] {}_{}[{}]",
                        abs_wire.tile, abs_wire.ty, abs_wire.going_dir, abs_wire.wire_idx
                    )
                }
                _ => unreachable!(),
            }
            .unwrap();
        }

        ret
    }
}
struct BitstreamDebugTracer {
    bit_w: usize,
    accesses: RefCell<Vec<Option<(TilePos, TileRelativeBitPos, String)>>>,
}
impl pannotia::container::DebugTracer for BitstreamDebugTracer {
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

macro_rules! hypothetical_autogenerated_fn_to_string {
    (rmux) => {
        "rmux"
    };
}

macro_rules! dump_one_thingy {
    ($self:ident, $magic_macro:ident, $fn_name:ident, $i:expr) => {
        let setting = $self.rmux($i);
        if setting != Default::default() {
            let formatted = (&&PrettyPrintWrap(setting)).pretty_print(
                $self.family(),
                $self.pos(),
                $self.tile_type(),
                $i,
            );
            println!(
                "tile[{}].{}[{}] = {}",
                $self.pos(),
                $magic_macro!($fn_name),
                $i,
                formatted
            );
        }
    };
}

trait DumpTile {
    fn dump(&self);
}
impl<
    D: pannotia::container::DebugTracer,
    Ref: std::borrow::Borrow<pannotia::container::Bitstream<D>>,
> DumpTile for pannotia::tiles::generic_routing::GenericRoutingRef<D, Ref>
{
    fn dump(&self) {
        for i in 0..96 {
            dump_one_thingy!(self, hypothetical_autogenerated_fn_to_string, rmux, i);
        }
    }
}

trait PrettyPrintGeneric {
    fn pretty_print(
        &self,
        family: pannotia::chips::Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> String;
}
impl<T: Display> PrettyPrintGeneric for PrettyPrintWrap<T> {
    fn pretty_print(
        &self,
        _family: pannotia::chips::Family,
        _tile_pos: TilePos,
        _tile_type: TileType,
        _i: u8,
    ) -> String {
        format!("{}", self.0)
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

                            for clk_i in 0..2 {
                                let clk_mux = tile.clock_mux(clk_i);
                                if clk_mux != Default::default() {
                                    let formatted = (&&PrettyPrintWrap(clk_mux)).pretty_print(
                                        tile.family(),
                                        tile.pos(),
                                        tile.tile_type(),
                                        clk_i,
                                    );
                                    println!("tile[{}].clk[{}] = {}", tile_pos, clk_i, formatted);
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
                                    if let IMUX::I(imux_idx) = lut_inp {
                                        print!(
                                            "tile[{}].lut_{}[{}] = {}",
                                            tile_pos,
                                            ["A", "B", "C", "D"][lut_inp_i as usize],
                                            lut_i,
                                            lut_inp
                                        );

                                        let imux_src = pannotia::routedb::logic_imux_input(
                                            lut_i, lut_inp_i, imux_idx,
                                        );
                                        match imux_src {
                                            FunctionInputSource::RMUX(i) => {
                                                println!("\t// rmux[{i}]")
                                            }
                                            FunctionInputSource::LEOutput(i) => {
                                                println!("\t// this_output[{i}]")
                                            }
                                            FunctionInputSource::RightNeighborWire(i) => {
                                                let tile_right = tile.pos() + Direction::E;
                                                println!("\t// tile[{}] T4_W[{}]", tile_right, i);
                                            }
                                            _ => unreachable!(),
                                        }
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
                                if (!lc_carry_en).0 {
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
    } else if args[1].eq_ignore_ascii_case("debug_rmux_routing") {
        for rmux_i in 0..96 {
            println!("RMUX_21_1 m_RMUX{rmux_i:02} (");
            for inp_i in (0..21).rev() {
                let inp = pannotia::routedb::rmux_input(rmux_i, inp_i, true);
                print!("    .I{inp_i}(");
                match inp {
                    RMUXSource::GlobalToLocal(i) => {
                        print!("IsoMUXPseudo{:02}_O", i);
                    }
                    RMUXSource::RMUX(i) => {
                        print!("RMUX{:02}_O", i);
                    }
                    RMUXSource::CellOutput(i) => {
                        // print!("OMUX{:02}_O", i * 3 + 2);   // For a logic tile
                        print!("BufMUX{:02}_O", i); // For a BRAM tile
                    }
                    RMUXSource::RoutingWire(RoutingWire {
                        ty,
                        going_dir,
                        bundle,
                        wire_idx,
                    }) => {
                        let xy =
                            match going_dir {
                                pannotia::routedb::Direction::N
                                | pannotia::routedb::Direction::S => "Y",
                                pannotia::routedb::Direction::E
                                | pannotia::routedb::Direction::W => "X",
                            };
                        print!(
                            "{}{}_{}_I{}[{}]",
                            ty,
                            if ty != pannotia::routedb::WireType::T1 {
                                xy
                            } else {
                                ""
                            },
                            going_dir,
                            bundle,
                            wire_idx,
                        );
                    }
                    _ => unreachable!(),
                }
                println!("),")
            }
            println!("    .O0(RMUX{rmux_i:02}_O));\n");
        }
    } else if args[1].eq_ignore_ascii_case("debug_logic_imux_routing") {
        for le_i in 0..16 {
            for le_inp_i in 0..4 {
                let imux_i = le_i * 4 + le_inp_i;
                println!("IMUX_27_1 m_IMUX{:02} (", imux_i);
                for mux_inp_i in (0..27).rev() {
                    let inp = pannotia::routedb::logic_imux_input(le_i, le_inp_i, mux_inp_i);
                    print!("    .I{mux_inp_i}(");
                    match inp {
                        FunctionInputSource::RMUX(i) => {
                            print!("RMUX{:02}_O", i);
                        }
                        FunctionInputSource::RightNeighborWire(i) => {
                            print!("T1_W_I0[{i}]");
                        }
                        FunctionInputSource::LEOutput(i) => {
                            print!("OMUX{:02}_O", i * 3 + 1);
                        }
                        _ => unreachable!(),
                    }
                    println!("),")
                }
                println!("    .O0(IMUX{imux_i:02}_O));\n");
            }
        }
    } else if args[1].eq_ignore_ascii_case("debug_logic_ctrlmux_routing") {
        for ctrlmux_i in 0..4 {
            println!("CtrlMUX_32_1 m_CtrlMUX{:02} (", ctrlmux_i);
            for mux_inp_i in (0..32).rev() {
                let inp = pannotia::routedb::logic_ctrl_preselect_input(ctrlmux_i, mux_inp_i);
                print!("    .I{mux_inp_i}(");
                match inp {
                    FunctionInputSource::RMUX(i) => {
                        print!("RMUX{:02}_O", i);
                    }
                    FunctionInputSource::RightNeighborWire(i) => {
                        print!("T1_W_I0[{i}]");
                    }
                    FunctionInputSource::LEOutput(i) => {
                        print!("OMUX{:02}_O", i * 3 + 1);
                    }
                    _ => unreachable!(),
                }
                println!("),")
            }
            println!("    .O0(CtrlMUX{ctrlmux_i:02}_O));\n");
        }
    } else if args[1].eq_ignore_ascii_case("debug_bram_imux_routing") {
        for imux_i in 0..64 {
            println!("IMUX_27_1 m_IMUX{:02} (", imux_i);
            for mux_inp_i in (0..27).rev() {
                let inp = pannotia::routedb::bram_imux_input(imux_i, mux_inp_i);
                print!("    .I{mux_inp_i}(");
                match inp {
                    FunctionInputSource::RMUX(i) => {
                        print!("RMUX{:02}_O", i);
                    }
                    FunctionInputSource::RightNeighborWire(i) => {
                        print!("T1_W_I0[{i}]");
                    }
                    FunctionInputSource::LeftNeighborWire(i) => {
                        print!("T1_E_I0[{i}]");
                    }
                    FunctionInputSource::Unused => {
                        print!("vcc");
                    }
                    _ => unreachable!(),
                }
                println!("),")
            }
            println!("    .O0(IMUX{imux_i:02}_O));\n");
        }
    } else if args[1].eq_ignore_ascii_case("debug_bram_ctrlmux_routing") {
        for ctrlmux_i in 0..4 {
            println!("CtrlMUX_32_1 m_CtrlMUX{:02} (", ctrlmux_i);
            for mux_inp_i in (0..32).rev() {
                let inp = pannotia::routedb::bram_ctrl_preselect_input(ctrlmux_i, mux_inp_i);
                print!("    .I{mux_inp_i}(");
                match inp {
                    FunctionInputSource::RMUX(i) => {
                        print!("RMUX{:02}_O", i);
                    }
                    FunctionInputSource::RightNeighborWire(i) => {
                        print!("T1_W_I0[{i}]");
                    }
                    FunctionInputSource::LeftNeighborWire(i) => {
                        print!("T1_E_I0[{i}]");
                    }
                    _ => unreachable!(),
                }
                println!("),")
            }
            println!("    .O0(CtrlMUX{ctrlmux_i:02}_O));\n");
        }
    } else if args[1].eq_ignore_ascii_case("debug_bram_kmux_routing") {
        for kmux_i in 0..16 {
            println!("KMUX_15_1 m_KMUX{:02} (", kmux_i);
            for mux_inp_i in (0..15).rev() {
                let inp = pannotia::routedb::kmux_input(kmux_i, mux_inp_i);
                println!("    .I{mux_inp_i}(TMUX{inp:02}_O),");
            }
            println!("    .O0(KMUX{kmux_i:02}_O));\n");
        }
    } else {
        return Err(Error::InvalidMode);
    }

    Ok(ExitCode::SUCCESS)
}
