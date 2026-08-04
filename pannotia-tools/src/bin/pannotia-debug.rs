//! Undocumented ad-hoc testing tools
//!
//! These are usually used for comparing our data against data extracted from the vendor tools

use std::process::ExitCode;

use pannotia::prelude::*;
use routedb::{FunctionInputSource, RMUXSource};

fn main() -> ExitCode {
    let args = std::env::args_os().collect::<Vec<_>>();

    if args.len() < 2 {
        println!("Usage: {} what_to_debug", args[0].to_string_lossy());
        return ExitCode::FAILURE;
    }
    if args[1].eq_ignore_ascii_case("debug_tile_grid") {
        if args.len() < 3 {
            println!(
                "Usage: {} debug_tile_grid family",
                args[0].to_string_lossy()
            );
            return ExitCode::FAILURE;
        }

        let family = args[2].to_string_lossy();
        let family = if let Ok(family) = Family::try_from(family.as_ref()) {
            family
        } else {
            eprintln!("invalid family");
            return ExitCode::FAILURE;
        };

        let (tile_w, tile_h) = family.tile_dims();
        for tile_y in (0..tile_h).rev() {
            for tile_x in 0..tile_w {
                let tile_pos = TilePos {
                    x: tile_x,
                    y: tile_y,
                };
                let tile_type = family.get_tile_type(tile_pos);
                print!("{:?}\t", tile_type);
            }
            println!()
        }
    } else if args[1].eq_ignore_ascii_case("debug_rmux_routing") {
        for rmux_i in 0..96 {
            println!("RMUX_21_1 m_RMUX{rmux_i:02} (");
            for inp_i in (0..21).rev() {
                let inp = routedb::rmux_input(rmux_i, inp_i, true);
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
                        let xy = match going_dir {
                            Direction::N | Direction::S => "Y",
                            Direction::E | Direction::W => "X",
                        };
                        print!(
                            "{}{}_{}_I{}[{}]",
                            ty,
                            if ty != WireType::T1 { xy } else { "" },
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
                    let inp = routedb::logic_imux_input(le_i, le_inp_i, mux_inp_i);
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
                let inp = routedb::logic_ctrl_preselect_input(ctrlmux_i, mux_inp_i);
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
                let inp = routedb::bram_imux_input(imux_i, mux_inp_i);
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
                let inp = routedb::bram_ctrl_preselect_input(ctrlmux_i, mux_inp_i);
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
                let inp = routedb::kmux_input(kmux_i, mux_inp_i);
                println!("    .I{mux_inp_i}(TMUX{inp:02}_O),");
            }
            println!("    .O0(KMUX{kmux_i:02}_O));\n");
        }
    } else {
        println!("invalid mode {}", args[1].to_string_lossy());
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
