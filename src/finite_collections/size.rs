
use num::{PrimInt, Unsigned};

pub trait Size: PrimInt + Unsigned
{
    const ZERO: Self;
    const MAX: Self;
    fn as_usize(&self) -> usize;
}

impl Size for usize {
    const ZERO: Self = 0usize;
    const MAX: Self = usize::MAX;
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl Size for u64 {
    const ZERO: Self = 0u64;
    const MAX: Self = u64::MAX;
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl Size for u32 {
    const ZERO: Self = 0u32;
    const MAX: Self = u32::MAX;
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl Size for u16 {
    const ZERO: Self = 0u16;
    const MAX: Self = u16::MAX;
    fn as_usize(&self) -> usize {
        *self as usize
    }
}

impl Size for u8 {
    const ZERO: Self = 0u8;
    const MAX: Self = u8::MAX;
    fn as_usize(&self) -> usize {
        *self as usize
    }
}
