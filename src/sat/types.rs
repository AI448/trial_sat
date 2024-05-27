pub type VariableSize = u32;

pub type ConstraintSize = u32;

#[derive(Clone, Copy)]
pub struct Literal {
    pub sign: bool,
    pub index: VariableSize,
}

// MEMO: binary 型とかつくるか？
