pub type Index = u32;

#[derive(Clone, Copy)]
pub struct Literal {
    pub sign: bool,
    pub index: Index,
}

pub type AssignmentLevel = u32;

pub type DecisionLevel = u32;
