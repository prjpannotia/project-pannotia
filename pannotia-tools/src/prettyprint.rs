//! Pretty-print muxes
//!
//! This module *relies on* autoderef specialization!

use std::fmt::Display;

use pannotia::prelude::*;
use pannotia::routedb::*;

/// This is needed to get around issues with blanket impls
pub struct PrettyPrintWrap<T>(pub T);

pub trait PrettyPrintGeneric {
    fn pretty_print<W: std::fmt::Write>(
        &self,
        w: W,
        family: Family,
        tile_pos: TilePos,
        tile_type: TileType,
        i: u8,
    ) -> std::fmt::Result;
}
impl<T: Display> PrettyPrintGeneric for PrettyPrintWrap<T> {
    fn pretty_print<W: std::fmt::Write>(
        &self,
        mut w: W,
        _family: Family,
        _tile_pos: TilePos,
        _tile_type: TileType,
        _i: u8,
    ) -> std::fmt::Result {
        write!(w, "{}", self.0)
    }
}

/// Implements a more-specific pretty-print, using autoderef specialization
macro_rules! make_specific_prettyprint {
    ($ty:ty, $val:ident, $w:ident, $family:ident, $tile_pos:ident, $tile_type:ident, $i:ident $body:expr) => {
        mident::mident! {
            pub trait #concat(PrettyPrint #flatten_basename($ty)) {
                fn pretty_print<W: std::fmt::Write>(
                    &self,
                    w: W,
                    family: Family,
                    tile_pos: TilePos,
                    tile_type: TileType,
                    i: u8,
                ) -> std::fmt::Result;
            }
            impl #concat(PrettyPrint #flatten_basename($ty)) for &PrettyPrintWrap<$ty> {
                fn pretty_print<W: std::fmt::Write>(
                    &self,
                    mut $w: W,
                    $family: Family,
                    $tile_pos: TilePos,
                    $tile_type: TileType,
                    $i: u8,
                ) -> std::fmt::Result {
                    let $val = self.0;
                    $body
                    Ok(())
                }
            }
        }
    };
}

make_specific_prettyprint!(bool, val, w, _family, _tile_pos, _tile_type, _i {
    // print bools as just "1"
    write!(w, "{}", val as u8)?;
});
make_specific_prettyprint!(::bitmux::InvertedBool, val, w, _family, _tile_pos, _tile_type, _i {
    // print bools as just "1"
    write!(w, "{}", val.0 as u8)?;
});

make_specific_prettyprint!(LUT, val, w, _family, _tile_pos, _tile_type, _i {
    // print LUT values in hex
    write!(w, "0x{:04x}", val.0)?;
});

make_specific_prettyprint!(mux::RMUX, val, w, family, tile_pos, tile_type, i {
    write!(w, "{}", val)?;

    if let mux::RMUX::I(rmux_inp_i) = val {
        write!(w, "\t// ")?;

        // mux purpose
        let this_rmux = RMUX_PURPOSE[i as usize];
        match this_rmux {
            RMUXPurpose::SelfWire(wire_idx) => write!(w, "rmux_self[{}]", wire_idx)?,
            RMUXPurpose::LeftNeighbor(wire_idx) => write!(w, "T1_W[{}]", wire_idx)?,
            RMUXPurpose::Span4 {
                going_dir,
                wire_idx,
            } => write!(w, "T4_{}[{}]", going_dir, wire_idx)?,
        }

        write!(w, " = ")?;

        // mux source (decoded)
        let rmux_src = rmux_input(i, rmux_inp_i, tile_type == TileType::BRAM);
        match rmux_src {
            RMUXSource::GlobalToLocal(i) => write!(w, "glb2loc[{i}]")?,
            RMUXSource::RMUX(i) => write!(w, "rmux[{i}]")?,
            RMUXSource::CellOutput(i) => write!(w, "this_output[{i}]")?,
            RMUXSource::RoutingWire(src_wire) => {
                let abs_wire = src_wire.to_absolute(family, tile_pos);
                write!(
                    w,
                    "tile[{}] {}_{}[{}]",
                    abs_wire.tile, abs_wire.ty, abs_wire.going_dir, abs_wire.wire_idx
                )?
            }
            _ => unreachable!(),
        }
    }
});

fn print_function_input_source<W: std::fmt::Write>(
    src: FunctionInputSource,
    tile_pos: TilePos,
    mut w: W,
) -> std::fmt::Result {
    match src {
        FunctionInputSource::RMUX(i) => write!(w, "rmux[{i}]")?,
        FunctionInputSource::LEOutput(i) => write!(w, "this_output[{i}]")?,
        FunctionInputSource::RightNeighborWire(i) => {
            let tile_right = tile_pos + Direction::E;
            write!(w, "tile[{}] T1_W[{}]", tile_right, i)?
        }
        FunctionInputSource::LeftNeighborWire(i) => {
            let tile_right = tile_pos + Direction::W;
            write!(w, "tile[{}] T1_E[{}]", tile_right, i)?
        }
        FunctionInputSource::Unused => write!(w, "vcc")?,
        _ => unreachable!(),
    }
    Ok(())
}

make_specific_prettyprint!(mux::IMUX, val, w, _family, tile_pos, tile_type, i {
    write!(w, "{}", val)?;

    if let mux::IMUX::I(imux_inp_i) = val {
        write!(w, "\t// ")?;

        let imux_src = if tile_type == TileType::BRAM {
            bram_imux_input(i, imux_inp_i)
        } else {
            logic_imux_input(i / 4, i % 4, imux_inp_i)
        };
        print_function_input_source(imux_src, tile_pos, w)?
    }
});

make_specific_prettyprint!(mux::CtrlMux, val, w, _family, tile_pos, tile_type, i {
    write!(w, "{}", val)?;

    if let mux::CtrlMux::I(imux_inp_i) = val {
        write!(w, "\t// ")?;

        let ctrlmux_src = if tile_type == TileType::BRAM {
            bram_ctrl_preselect_input(i, imux_inp_i)
        } else {
            logic_ctrl_preselect_input(i, imux_inp_i)
        };
        print_function_input_source(ctrlmux_src, tile_pos, w)?
    }
});

make_specific_prettyprint!(mux::TMUX, val, w, family, tile_pos, _tile_type, i {
    write!(w, "{}", val)?;

    if let mux::TMUX::I(tmux_inp_i) = val {
        let tmux_src = TMUX_MAP[i as usize][tmux_inp_i as usize];

        if tmux_src != TMUXSource::Unused {
            write!(w, "\t// ")?;
        }

        match tmux_src {
            TMUXSource::GlobalToLocal(i) => write!(w, "glb2loc[{i}]")?,
            TMUXSource::RMUX(i) => write!(w, "rmux[{i}]")?,
            TMUXSource::RoutingWire(src_wire) => {
                let abs_wire = src_wire.to_absolute(family, tile_pos);
                write!(
                    w,
                    "tile[{}] {}_{}[{}]",
                    abs_wire.tile, abs_wire.ty, abs_wire.going_dir, abs_wire.wire_idx
                )?
            }
            _ => unreachable!(),
        }
    }
});

make_specific_prettyprint!(mux::KMUX, val, w, _family, _tile_pos, _tile_type, i {
    write!(w, "{}", val)?;

    if let mux::KMUX::I { i: kmux_inp_i, .. } = val {
        let kmux_src = kmux_input(i, kmux_inp_i);
        write!(w, "\t// tmux[{kmux_src}]")?;
    }
});

make_specific_prettyprint!(mux::LeftRightIOLocalMux, val, w, family, tile_pos, _tile_type, i {
    write!(w, "{}", val)?;

    if let mux::LeftRightIOLocalMux::I(mux_inp_i) = val {
        write!(w, "\t// ")?;

        let mux_src = left_right_io_local_line_input(i, mux_inp_i);
        match mux_src {
            IOLocalLineSource::GlobalToLocal(i) => write!(w, "glb2loc[{i}]")?,
            IOLocalLineSource::RoutingWire { ty, bundle, wire_idx } => {
                let going_dir = if tile_pos.x == 0 { Direction::W } else { Direction::E };
                let wire = RoutingWire { ty, bundle, wire_idx, going_dir };
                let abs_wire = wire.to_absolute(family, tile_pos);
                write!(
                    w,
                    "tile[{}] {}_{}[{}]",
                    abs_wire.tile, abs_wire.ty, abs_wire.going_dir, abs_wire.wire_idx
                )?
            }
            _ => unreachable!()
        }
    }
});

make_specific_prettyprint!(mux::TopBottomIOLocalMux, val, w, family, tile_pos, _tile_type, i {
    write!(w, "{}", val)?;

    if let mux::TopBottomIOLocalMux::I(mux_inp_i) = val {
        write!(w, "\t// ")?;

        let mux_src = top_bottom_io_local_line_input(i, mux_inp_i);
        match mux_src {
            IOLocalLineSource::GlobalToLocal(i) => write!(w, "glb2loc[{i}]")?,
            IOLocalLineSource::RoutingWire { ty, bundle, wire_idx } => {
                let going_dir = if tile_pos.y == 0 { Direction::S } else { Direction::N };
                let wire = RoutingWire { ty, bundle, wire_idx, going_dir };
                let abs_wire = wire.to_absolute(family, tile_pos);
                write!(
                    w,
                    "tile[{}] {}_{}[{}]",
                    abs_wire.tile, abs_wire.ty, abs_wire.going_dir, abs_wire.wire_idx
                )?
            }
            _ => unreachable!()
        }
    }
});

make_specific_prettyprint!(mux::IOLocalToClockMux, val, w, family, tile_pos, _tile_type, i {
    write!(w, "{}", val)?;

    if let mux::IOLocalToClockMux::I(mux_inp_i) = val {
        let mux_src = if tile_pos.x == 0 || tile_pos.x == family.tile_dims().0 - 1 {
            left_right_io_clock_input(i, mux_inp_i)
        } else {
            top_bottom_io_clock_input(i, mux_inp_i)
        };
        write!(w, "\t// local_line[{mux_src}]")?;
    }
});
make_specific_prettyprint!(mux::LocalToIOMux, val, w, family, tile_pos, _tile_type, i {
    write!(w, "{}", val)?;

    if let mux::LocalToIOMux::I { i: mux_inp_i, .. } = val {
        let mux_src = if tile_pos.x == 0 || tile_pos.x == family.tile_dims().0 - 1 {
            left_right_io_signal_input(i, mux_inp_i)
        } else {
            top_bottom_io_signal_input(i, mux_inp_i)
        };
        write!(w, "\t// local_line[{mux_src}]")?;
    }
});
