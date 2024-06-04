use std::cmp::Eq;

pub type VariableSize = u32;

pub type ConstraintSize = u32;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Literal {
    pub sign: bool,
    pub index: VariableSize,
}

/// 割り当て理由
#[derive(Clone, Copy)]
pub enum Reason {
    // 12 byte まで切り詰めれば幸せになれるかもしれない
    Decision,
    Propagation {
        clause_index: ConstraintSize,
        lbd: VariableSize,           // TODO 1byte に
        clause_length: VariableSize, // TODO 1byte に
        assignment_level_at_propagated: VariableSize,
    },
}

// MEMO: binary 型とかつくるか？
