use std::cmp::Eq;

pub type VariableSize = u32;

pub type ConstraintSize = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Literal {
    pub sign: bool,
    pub index: VariableSize,
}

// MEMO: binary 型とかつくるか？
