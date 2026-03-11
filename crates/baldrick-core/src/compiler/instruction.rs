use serde::{Deserialize, Serialize};

/// Bytecode instructions for the Baldrick VM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Instruction {
    // Stack
    Push(Constant),
    Pop,
    Dup,

    // Variables
    LoadLocal(usize),
    StoreLocal(usize),
    LoadGlobal(String),
    StoreGlobal(String),
    DeclareLocal(String),

    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Neg,
    BitNot,
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
    Ushr,

    // Comparison
    Eq,
    Neq,
    StrictEq,
    StrictNeq,
    Lt,
    Lte,
    Gt,
    Gte,

    // Logical
    Not,

    // Objects & Arrays
    CreateArray(usize),
    CreateObject(usize),
    GetProperty(String),
    SetProperty(String),
    GetIndex,
    SetIndex,
    Spread,
    In,
    InstanceOf,

    // Functions
    CreateClosure(usize),
    Call(usize),
    Return,
    CallExternal(String, usize),

    // Control flow
    Jump(usize),
    JumpIfFalse(usize),
    JumpIfTrue(usize),
    JumpIfNullish(usize),

    // Loops
    SetupLoop,
    Break,
    Continue,

    // Iterators
    GetIterator,
    IteratorNext,
    IteratorDone,

    // Error handling
    SetupTry(usize, Option<usize>),
    Throw,
    EndTry,

    // Typeof
    TypeOf,

    // Void
    Void,

    // Update
    Increment,
    Decrement,

    // Template literals
    ConcatStrings(usize),

    // Destructuring
    DestructureObject(Vec<String>),
    DestructureArray(usize),

    // Misc
    Nop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Constant {
    Undefined,
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    String(String),
}
