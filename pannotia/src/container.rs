//! Bitstream container file format
//!
//! This module handles the "raw binary" bitstream container file format,
//! which is the highest abstraction level handled directly by hardware.
//!
//! This format organizes the following information:
//! - IO pad configuration
//! - PLL/clock configuration
//! - "miscellaneous" configuration
//! - the "actual logic"
//!
//! However, even though this format is structured, there is typically only one
//! (or rarely, a small number) instance of each of the above pieces of information,
//! and the "actual logic" is one gigantic block, so most details of this format
//! can be thought of as "useless" or conceptually "irrelevant" overhead.
//!
//! ## Overall structure
//!
//! A bitstream binary consists of the following:
//! 1. Device ID
//! 2. User ID
//! 3. Commands
//!     - Array writes
//!     - Register writes
//! 4. CRC32
//!
//! The device ID is a 32-bit word indicating the target FPGA device.
//!
//! The user ID is a 32-bit word. If not specified, the vendor tools default to `0x0000ffff`.
//! TODO: It can presumably be read out via JTAG on devices other than the AGRV2K, but it is useless on the AGRV2K.
//!
//! The CRC32 polynomial is `0x04c11db7`, specifically the [`CRC-32/BZIP2`](https://reveng.sourceforge.io/crc-catalogue/all.htm#crc.cat.crc-32-bzip2) configuration.
//!
//! ## Registers
//!
//! There is only one known register, with address `2`. It is written with the value `0xf8f` at the end of the configuration process.
//! TODO: `DEV_OE` and `DEV_CLRn`
//!
//! ## Arrays
//!
//! Every other chunk of information is written to an array, which is identified by a "group" and "chain".
//!
//! For example, on the ARGV2K, group 1 chain 0 configures the IO pads and group 1 chain 1 configures the PLL.
//!
//! The "actual logic" is stored as one extremely-large array in group 0 chain 0.

/// A bitstream control word, describing how to process the data that follows
///
/// The format of the control word is as follows:
/// - `bits[31:29]` - type (`101` for array data, `001` for a register access)
/// - `bits[27]` - indicates that this is the last frame
/// - `bits[25]` - indicates that this is a write (it is unknown where a read would output data to)
///
/// ## For array data
/// - `bits[9:5]` - indicates the config "group"
/// - `bits[4:0]` - indicates the config "chain"
///
/// This is then followed by:
/// - `word2[31:8]` - length in bits, minus 1
/// - `word2[7:4]` - "idle clocks" (it is unknown what precisely this does)
///
/// ## For a register access
/// - `bits[24:10]` - this is always 0x3f
/// - `bits[9:0]` - this is the register address
///
/// This is then followed by a 32-bit value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeaderWord(pub u32);
impl HeaderWord {
    pub const fn hdr_type(self) -> u32 {
        self.0 >> 29
    }
    pub const fn last_frame(self) -> bool {
        (self.0 & (1 << 27)) != 0
    }
    pub const fn config_group(self) -> u32 {
        (self.0 >> 5) & 0b11111
    }
    pub const fn config_chain(self) -> u32 {
        self.0 & 0b11111
    }
    pub const fn register(self) -> u32 {
        self.0 & 0b1111111111
    }

    pub const fn make_config_hdr(last_frame: bool, config_group: u32, config_chain: u32) -> Self {
        let x = (0b101 << 29)
            | (if last_frame { 1 << 27 } else { 0 })
            | (1 << 25)
            | (config_group << 5)
            | config_chain;
        Self(x)
    }
    pub const fn make_reg_write_hdr(last_frame: bool, reg_addr: u32) -> Self {
        let x = (0b001 << 29)
            | (if last_frame { 1 << 27 } else { 0 })
            | (1 << 25)
            | (0x3f << 10)
            | reg_addr;
        Self(x)
    }
}

/// A second word for config arrays, specifying the length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConfigWord(pub u32);
impl ConfigWord {
    pub const fn bits(self) -> u32 {
        (self.0 >> 8) + 1
    }

    pub const fn make_config_word(bits: u32) -> Self {
        let x = ((bits - 1) << 8) | (2 << 4);
        Self(x)
    }
}

pub(crate) const BITSTREAM_CRC: crc::Crc<u32> = crc::Crc::<u32>::new(&crc::CRC_32_BZIP2);
