//! Utilities for dealing with fields in programmable logic bitstreams

#![no_std]

use bitvec::prelude::*;

/// Hardcode that each field can be encoded with at most 32 bits
type FieldPackedBitsTy = u32;

pub trait BitGetter {
    fn get_bit(&self, biti: usize) -> bool;

    #[inline]
    fn get_bits<const N: usize>(&self) -> FieldPackedBitsTy {
        let mut bits = 0u32;
        let bits_ = BitSlice::<_, Lsb0>::from_element_mut(&mut bits);
        for i in 0..N {
            bits_.set(i, self.get_bit(i));
        }
        bits
    }
}

pub trait BitSetter {
    fn set_bit(&mut self, biti: usize, val: bool);

    #[inline]
    fn set_bits<const N: usize>(&mut self, val: FieldPackedBitsTy) {
        let val = BitSlice::<_, Lsb0>::from_element(&val);
        for i in 0..N {
            self.set_bit(i, val[i]);
        }
    }
}

pub trait BitEnum {
    fn get(b: impl BitGetter) -> Self;
    fn set(&self, b: impl BitSetter);
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TestEnum {
    ValA,
    ValB,
    ValC,
}
impl BitEnum for TestEnum {
    fn get(g: impl BitGetter) -> Self {
        let bits = g.get_bits::<2>();
        match bits {
            0b00 => Self::ValA,
            0b01 => Self::ValB,
            _ if bits & 0b10 == 0b10 => Self::ValC,
            _ => unreachable!(),
        }
    }

    fn set(&self, mut s: impl BitSetter) {
        match self {
            TestEnum::ValA => s.set_bits::<2>(0b00),
            TestEnum::ValB => s.set_bits::<2>(0b01),
            TestEnum::ValC => s.set_bits::<2>(0b11),
        }
    }
}

#[cfg(test)]
mod tests {
    mod test_bit_getter_setter {
        use super::super::*;

        impl BitGetter for u32 {
            fn get_bit(&self, biti: usize) -> bool {
                let self_ = BitSlice::<_, Lsb0>::from_element(self);
                self_[biti]
            }
        }
        impl BitSetter for u32 {
            fn set_bit(&mut self, biti: usize, val: bool) {
                let self_ = BitSlice::<_, Lsb0>::from_element_mut(self);
                self_.set(biti, val);
            }
        }

        #[test]
        fn test() {
            let mut x = 0x123u32;
            assert_eq!(x.get_bits::<32>(), 0x123);
            assert_eq!(x.get_bits::<8>(), 0x23);
            assert_eq!(x.get_bits::<4>(), 0x3);

            x.set_bits::<8>(0x345);
            assert_eq!(x, 0x145);
            x.set_bits::<4>(0x456);
            assert_eq!(x, 0x146);
        }
    }

    mod test_enum {
        use super::super::*;
        use core::borrow::{Borrow, BorrowMut};

        struct EnumRef<B: Borrow<u32>> {
            b: B,
            idx: usize,
        }

        impl<B: Borrow<u32>> BitGetter for EnumRef<B> {
            fn get_bit(&self, biti: usize) -> bool {
                let bits = self.b.borrow();
                let bits = BitSlice::<_, Lsb0>::from_element(bits);
                bits[self.idx * 4 + biti]
            }
        }
        impl<B: BorrowMut<u32>> BitSetter for EnumRef<B> {
            fn set_bit(&mut self, biti: usize, val: bool) {
                let bits = self.b.borrow_mut();
                let bits = BitSlice::<_, Lsb0>::from_element_mut(bits);
                bits.set(self.idx * 4 + biti, val);
            }
        }

        #[test]
        fn test() {
            let mut x = 0x1234u32;
            assert_eq!(TestEnum::get(EnumRef { b: &x, idx: 0 }), TestEnum::ValA);
            assert_eq!(TestEnum::get(EnumRef { b: &x, idx: 1 }), TestEnum::ValC);
            assert_eq!(TestEnum::get(EnumRef { b: &x, idx: 2 }), TestEnum::ValC);
            assert_eq!(TestEnum::get(EnumRef { b: &x, idx: 3 }), TestEnum::ValB);

            TestEnum::ValC.set(EnumRef { b: &mut x, idx: 0 });
            assert_eq!(x, 0x1237);
            TestEnum::ValA.set(EnumRef { b: &mut x, idx: 1 });
            assert_eq!(x, 0x1207);
            TestEnum::ValB.set(EnumRef { b: &mut x, idx: 2 });
            assert_eq!(x, 0x1107);
        }
    }
}
