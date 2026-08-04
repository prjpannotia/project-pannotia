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
    ($self:ident $w:ident $fn_name:ident $fn_str:literal = $start:literal .. $end:literal) => {
        for i in $start..$end {
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
    ($self:ident $w:ident @bool $fn_name:ident $fn_str:literal = $start:literal .. $end:literal) => {
        for i in $start..$end {
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
    ($self:ident: $($tile_ref:ident)::+ { $($(@$maybe_bool:ident)? $func:ident $human_name:literal $eq_or_semi:tt $($count:literal $(.. $count_range:literal)? )? $({ $($count_complex:tt)* })? $(;)?)* }) => {
        impl<
            D: debug::DebugTracer,
            Ref: std::borrow::Borrow<Bitstream<D>>,
        > crate::DumpTile for $($tile_ref)::+<D, Ref>
        {
            fn dump<W: std::fmt::Write>(&$self, mut w: W) -> std::fmt::Result {
                $({
                    _dump_one_tile_field!{ $self w $(@$maybe_bool)? $func $human_name $eq_or_semi $($count $(.. $count_range)? )? $(_replace_self!{ $self { $($count_complex)* } })? }
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
        kmux "kmux_unused" = 10..16;

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

make_tile_fields! {
    self: tile::TopBottomIOTileRef {
        local_line "local_line" = 32;

        out_clock_global_to_local "out_glb2loc" = 4;
        out_clock_local_to_clock "out_loc2clk" = 4;
        out_clock_choice "out_clk" = 4;
        @bool out_use_reg "out_use_reg" = 4;
        out_async_mode "out_async_mode" = 4;
        out_sync_mode "out_sync_mode" = 4;
        @bool out_powerup_state "out_powerup_state" = 4;
        @bool oe_use_reg "oe_use_reg" = 4;
        oe_async_mode "oe_async_mode" = 4;
        oe_sync_mode "oe_sync_mode" = 4;
        @bool oe_powerup_state "oe_powerup_state" = 4;
        in_clock_global_to_local "in_glb2loc" = 4;
        in_clock_local_to_clock "in_loc2clk" = 4;
        in_clock_choice "in_clk" = 4;
        in_async_mode "in_async_mode" = 4;
        in_sync_mode "in_sync_mode" = 4;
        @bool in_powerup_state "in_powerup_state" = 4;
        local_to_io_out "loc_to_io_out" = 4;
        local_to_io_oe "loc_to_io_oe" = 4;
        local_to_out_clk_en "loc_to_out_cen" = 4;
        local_to_in_clk_en "loc_to_in_cen" = 4;
        local_to_async_ctrl "loc_to_async" = 4;
        local_to_sync_ctrl "loc_to_sync" = 4;

        out_mux_0 "omux0" = 4;
        out_mux_1 "omux1" = 4;

        in_data_delay "in_data_delay" = 4;
        in_reg_delay "in_reg_delay" = 4;
        @bool out_delay "out_delay" = 4;
    }
}
make_tile_fields! {
    self: tile::LeftRightIOTileRef {
        local_line "local_line" = 48;

        out_clock_global_to_local "out_glb2loc" = 6;
        out_clock_local_to_clock "out_loc2clk" = 6;
        out_clock_choice "out_clk" = 6;
        @bool out_use_reg "out_use_reg" = 6;
        out_async_mode "out_async_mode" = 6;
        out_sync_mode "out_sync_mode" = 6;
        @bool out_powerup_state "out_powerup_state" = 6;
        @bool oe_use_reg "oe_use_reg" = 6;
        oe_async_mode "oe_async_mode" = 6;
        oe_sync_mode "oe_sync_mode" = 6;
        @bool oe_powerup_state "oe_powerup_state" = 6;
        in_clock_global_to_local "in_glb2loc" = 6;
        in_clock_local_to_clock "in_loc2clk" = 6;
        in_clock_choice "in_clk" = 6;
        in_async_mode "in_async_mode" = 6;
        in_sync_mode "in_sync_mode" = 6;
        @bool in_powerup_state "in_powerup_state" = 6;
        local_to_io_out "loc_to_io_out" = 6;
        local_to_io_oe "loc_to_io_oe" = 6;
        local_to_out_clk_en "loc_to_out_cen" = 6;
        local_to_in_clk_en "loc_to_in_cen" = 6;
        local_to_async_ctrl "loc_to_async" = 6;
        local_to_sync_ctrl "loc_to_sync" = 6;

        out_mux_0 "omux0" = 6;
        out_mux_1 "omux1" = 6;

        in_data_delay "in_data_delay" = 6;
        in_reg_delay "in_reg_delay" = 6;
        @bool out_delay "out_delay" = 6;
    }
}

make_tile_fields! {
    self: tile::PLLTileRef {
        to_pll "to_pll" = 11;
        global_to_local "glb2loc" = 11;

        gclk_mux "gclk_mux";
        clock_mux_0 "clock_mux_0";
        in_div_lo_time "in_div_lo_time";
        in_div_hi_time "in_div_hi_time";
        @bool in_div_duty_cycle_adjust "in_div_duty_cycle_adjust";
        @bool in_div_bypass "in_div_bypass";

        clock_feedback_mux "clock_feedback_mux";
        @bool use_internal_fb "use_internal_fb";
        feedback_delay "feedback_delay";
        fb_div_lo_time "fb_div_lo_time";
        fb_div_hi_time "fb_div_hi_time";
        @bool fb_div_duty_cycle_adjust "fb_div_duty_cycle_adjust";
        @bool fb_div_bypass "fb_div_bypass";
        fb_phase_coarse "fb_phase_coarse";
        fb_phase_fine "fb_phase_fine";

        @bool out_enable "out_enable" = 5;
        @bool out_cascade "out_cascade" = 1..5;
        out_div_lo_time "out_div_lo_time" = 5;
        out_div_hi_time "out_div_hi_time" = 5;
        @bool out_div_duty_cycle_adjust "out_div_duty_cycle_adjust" = 5;
        @bool out_div_bypass "out_div_bypass" = 5;
        out_phase_coarse "out_phase_coarse" = 5;
        out_phase_fine "out_phase_fine" = 5;

        @bool vco_div2 "vco_div2";

        reg_ctrl "reg_ctrl";
        @bool enabled "enabled";
        @bool enable_dedicated_out_n "enable_dedicated_out_n";
        @bool enable_dedicated_out_p "enable_dedicated_out_p";

        analog_icp "analog_icp";
        analog_rlpf "analog_rlpf";
        analog_rref "analog_rref";
        analog_rvi "analog_rvi";
        analog_ivco "analog_ivco";
    }
}

make_tile_fields! {
    self: tile::GCLKSWTileRef {
        fabric_to_clock "fab2clk" = 6;
        clock_enable "ce" = 6;
        global_to_local "glb2loc" = 12;
        clock_to_fabric "clk2fab" = 4;
        @bool cen_is_registered "cen_registered" = 6;
        clock_dist_mux "clock_dist_mux" = 5;
    }
}
