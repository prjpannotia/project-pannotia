//! Traits, macros, etc for going through everything in a tile

use pannotia::prelude::*;

// *MUST* import all the traits!
use crate::prettyprint::*;

/// Replace a "bad" `self` token with a "good" `self` token
macro_rules! _replace_self_tok {
    ($desired_self:ident self) => {
        $desired_self
    };
    ($desired_self:ident $tok:tt) => {
        $tok
    };
}

/// Replace _all_ "bad" `self` tokens with a "good" `self` token
///
/// This is needed to work around some weird hygiene rules
macro_rules! _replace_self {
    ($desired_self:ident $($toks:tt)*) => {
        $(_replace_self_tok!($desired_self $toks))*
    };
}

macro_rules! _dump_one_tile_field {
    ($self:ident $w:ident $fn_name:ident $fn_str:literal = $count:expr) => {
        let count = $count;
        for i in 0..count {
            let setting = $self.$fn_name(i);
            if setting != Default::default() {
                write!($w, "tile[{}].{}[{}] = ", $self.pos(), $fn_str, i)?;
                (&&PrettyPrintWrap(setting)).pretty_print(
                    &mut $w,
                    $self.family(),
                    $self.pos(),
                    $self.tile_type(),
                    i,
                )?;
                writeln!($w)?;
            }
        }
    };
    ($self:ident $w:ident $fn_name:ident $fn_str:literal ; ) => {
        let setting = $self.$fn_name();
        if setting != Default::default() {
            write!($w, "tile[{}].{} = ", $self.pos(), $fn_str)?;
            (&&PrettyPrintWrap(setting)).pretty_print(
                &mut $w,
                $self.family(),
                $self.pos(),
                $self.tile_type(),
                0,
            )?;
            writeln!($w)?;
        }
    };

    // Copypasta for bool, since there's issues with Default
    ($self:ident $w:ident @bool $fn_name:ident $fn_str:literal = $count:expr) => {
        let count = $count;
        for i in 0..count {
            let setting = $self.$fn_name(i);
            if setting {
                write!($w, "tile[{}].{}[{}] = ", $self.pos(), $fn_str, i)?;
                (&&PrettyPrintWrap(setting)).pretty_print(
                    &mut $w,
                    $self.family(),
                    $self.pos(),
                    $self.tile_type(),
                    i,
                )?;
                writeln!($w)?;
            }
        }
    };
    ($self:ident $w:ident @bool $fn_name:ident $fn_str:literal ; ) => {
        let setting = $self.$fn_name();
        if setting {
            write!($w, "tile[{}].{} = ", $self.pos(), $fn_str)?;
            (&&PrettyPrintWrap(setting)).pretty_print(
                &mut $w,
                $self.family(),
                $self.pos(),
                $self.tile_type(),
                0,
            )?;
            writeln!($w)?;
        }
    };
}

macro_rules! make_tile_fields {
    ($self:ident: $($tile_ref:ident)::+ { $($(@$maybe_bool:ident)? $func:ident $human_name:literal $eq_or_semi:tt $($count:literal)? $({ $($count_complex:tt)* })? $(;)?)* }) => {
        impl<
            D: debug::DebugTracer,
            Ref: std::borrow::Borrow<Bitstream<D>>,
        > crate::DumpTile for $($tile_ref)::+<D, Ref>
        {
            fn dump<W: std::fmt::Write>(&$self, mut w: W) -> std::fmt::Result {
                $({
                    _dump_one_tile_field!{ $self w $(@$maybe_bool)? $func $human_name $eq_or_semi $($count)? $(_replace_self!{ $self { $($count_complex)* } })? }
                })*

                Ok(())
            }
        }
    };
}

make_tile_fields! {
    self: tile::GenericRoutingRef {
        rmux "rmux" = 96;
    }
}

make_tile_fields! {
    self: tile::LogicTileRef {
        rmux "rmux" = 96;
        clock_mux "clk" = 2;
        clock_en_mux "ce" = 2;
        async_mux "async" = 2;
        sync_load_mux "sync_load";
        sync_clr_mux "sync_clr";
        global_to_local "glb2loc" = 4;
        control_signal_preselect "ctrl" = 4;

        lut "lut" = 16;
        lut_inp_a "lut_A" = 16;
        lut_inp_b "lut_B" = 16;
        lut_inp_c "lut_C" = 16;
        lut_inp_d "lut_D" = 16;
        lc_input_c_mode "inp_c" = 16;
        lc_carry_en "carry_en" = 16;
        lc_clk_choice "lc_clk" = 16;
        lc_async_choice "lc_async" = 16;
        @bool lc_shift_reg_mode "lc_shift" = 16;
        @bool lc_input_c_bypass_mode "lc_bypass" = 16;
        lc_output_neigh "omux_neigh" = 16;
        lc_output_imux "omux_imux" = 16;
        lc_output_rmux "omux_rmux" = 16;
    }
}

make_tile_fields! {
    self: tile::RoutingOnlyTileRef {
        rmux "rmux" = 96;
        global_to_local "glb2loc" = 4;
        right_neighbor_output "omux" = 16;
    }
}

make_tile_fields! {
    self: tile::BRAMTileRef {
        rmux "rmux" = 96;
        clock_mux "clk" = 2;
        clock_en_mux "ce" = 2;
        async_mux "async" = 2;
        global_to_local "glb2loc" = 6;
        control_signal_preselect "ctrl" = 4;

        addr_a "addr_a" = 13;
        addr_b "addr_b" = 13;
        data_in_a "data_in_a" = 18;
        data_in_b "data_in_b" = 18;
        imux_xtra "imux_xtra" = 2;
        tmux "tmux" = 16;
        read_en_a "read_en_a";
        read_en_b "read_en_b";
        write_en_a "write_en_a";
        write_en_b "write_en_b";
        addr_stall_a "addr_stall_a";
        addr_stall_b "addr_stall_b";
        byte_en_a "byte_en_a" = 2;
        byte_en_b "byte_en_b" = 2;
        kmux_unused "kmux_unused" = 6;

        @bool use_packed_mode_address_override "use_packed_mode_address_override";
        clock_choices_mode "clock_choices_mode";
        width_a "width_a";
        width_b "width_b";
        @bool use_output_register_a "use_output_register_a";
        @bool use_output_register_b "use_output_register_b";
        @bool use_rst_in_a "use_rst_in_a";
        @bool use_rst_in_b "use_rst_in_b";
        @bool use_rst_out_a "use_rst_out_a";
        @bool use_rst_out_b "use_rst_out_b";
        @bool use_clk_en_in_a "use_clk_en_in_a";
        @bool use_clk_en_in_b "use_clk_en_in_b";
        @bool use_clk_en_out_a "use_clk_en_out_a";
        @bool use_clk_en_out_b "use_clk_en_out_b";
        @bool write_thru_a "write_thru_a";
        @bool write_thru_b "write_thru_b";
        rsen_delay "rsen_delay";
        delay_time "delay_time";
    }
}

make_tile_fields! {
    self: tile::TopIPTileRef {
        global_to_local "glb2loc" = 12;
        to_ip "to_ip" = 12;
        from_ip "from_ip" = 12;
    }
}
make_tile_fields! {
    self: tile::LeftRightIPTileRef {
        global_to_local "glb2loc" = 20;
        to_ip_13 "to_ip_13" = 12;
        to_ip_17 "to_ip_18" = 8;
        from_ip "from_ip" = 12;
    }
}
