use crate::lexer::{LocData, SymbolTable, TokenType};
use std::cell::Cell;
use std::fmt::Display;
use std::{collections::HashMap, rc::Rc};

use crate::lexer::{Op, Symbol};

#[derive(Debug, Default, Clone, Copy)]
pub enum VarType {
    #[default]
    Let,
    Mut,
}

impl Display for VarType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VarType::Let => write!(f, "let"),
            VarType::Mut => write!(f, "mut"),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Type {
    #[default]
    None,

    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,

    // Aliases
    Usize, // u32 on 32bit, u64 on 64bit
    Bool,  // u8
    Char,  // u8
    Byte,  // u8

    String, // [u8]
}

impl Type {
    fn numeric_type_info(
        &self,
    ) -> Option<(
        u8,   /* nbits */
        bool, /* signed */
        bool, /* fp */
    )> {
        match self {
            Type::I8 => Some((8, true, false)),
            Type::U8 => Some((8, false, false)),
            Type::I16 => Some((16, true, false)),
            Type::U16 => Some((16, false, false)),
            Type::I32 => Some((32, true, false)),
            Type::U32 => Some((32, false, false)),
            Type::I64 => Some((64, true, false)),
            Type::U64 => Some((64, false, false)),
            Type::F32 => Some((24, true, true)),
            Type::F64 => Some((53, true, true)),
            _ => {
                println!("Rejected Type at numeric_type_info(): {}", self);
                None
            }
        }
    }
    pub fn is_digit_convertible_to(&self, to: &Type) -> bool {
        if let (Some((from_bits, from_signed, from_is_fp)), Some((to_bits, to_signed, to_is_fp))) =
            (self.numeric_type_info(), to.numeric_type_info())
        {
            // eg. fit i32 inside i8 || eg. u32 -> i32 & vice versa
            if from_bits > to_bits || from_signed != to_signed {
                false
            } else if
            // If from-type is integral and to-type is fp,
            // we can coerce int to float iff
            // int nbits lower than fp mantissa nbits
            // eg. i32 -> f64
            !from_is_fp && to_is_fp {
                // 1.364 -> i32 will truncate.
                // so we only allow int to float,
                (from_bits < to_bits) && from_signed
            } else {
                true
            }
        } else {
            false
        }
    }
}

impl From<TokenType> for Type {
    fn from(value: TokenType) -> Self {
        match value {
            TokenType::Ti8 => Type::I8,
            TokenType::Tu8 => Type::U8,
            TokenType::Ti16 => Type::I16,
            TokenType::Tu16 => Type::U16,
            TokenType::Ti32 => Type::I32,
            TokenType::Tu32 => Type::U32,
            TokenType::Ti64 => Type::I64,
            TokenType::Tu64 => Type::U64,
            TokenType::Tf32 => Type::F32,
            TokenType::Tf64 => Type::F64,
            TokenType::Tusize => Type::Usize,
            TokenType::Tbyte => Type::Byte,
            TokenType::Tchar => Type::Char,
            TokenType::Tstring => Type::String,
            TokenType::Tbool => Type::Bool,
            _ => Type::None,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Type::I8 => write!(f, "i8"),
            Type::U8 => write!(f, "u8"),
            Type::I16 => write!(f, "i16"),
            Type::U16 => write!(f, "u16"),
            Type::I32 => write!(f, "i32"),
            Type::U32 => write!(f, "u32"),
            Type::I64 => write!(f, "i64"),
            Type::U64 => write!(f, "u64"),
            Type::F32 => write!(f, "f32"),
            Type::F64 => write!(f, "f64"),
            Type::Usize => write!(f, "usize"),
            Type::Byte => write!(f, "byte"),
            Type::Char => write!(f, "char"),
            Type::String => write!(f, "string"),
            Type::Bool => write!(f, "bool"),
            Type::None => write!(f, "None"),
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq, Hash, Clone, Copy)]
pub struct Ident {
    pub name: Symbol,
    pub loc: LocData,
}

#[derive(Debug, Clone, Copy)]
pub struct IntLit {
    pub val: i128,
    pub loc: LocData,
}

#[derive(Debug, Default, Clone)]
pub enum AtomKind {
    #[default]
    None,
    Ident(Ident),
    IntLit(IntLit),
}

#[derive(Debug, Default, Clone)]
pub struct Expr {
    pub atom: AtomKind,
    pub op: Op,
    pub lhs: Option<Box<Expr>>,
    pub rhs: Option<Box<Expr>>,
    pub ty: Cell<Option<Type>>,
    pub loc: LocData,
}

#[derive(Debug, Default, Clone)]
pub struct VarDecl {
    pub kind: VarType,
    pub id: Ident,
    pub decl_type: Option<Type>, // user declared
    pub ty: Cell<Option<Type>>,  // must exist before IR (inferred at sema)
    pub value: Box<Expr>,        // expr type must match local ty
    pub loc: LocData,
}

#[derive(Debug, Clone, Default)]
pub struct Scope {
    pub stmts: Vec<UnionNode>,
    pub vars: HashMap<Symbol, Rc<VarDecl>>,
    pub fns: HashMap<Symbol, Rc<StmtFn>>,
}

#[derive(Debug, Clone)]
pub struct StmtFn {
    pub id: Ident,
    pub args: Vec<Ident>,
    pub body: Scope,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub struct Call {
    pub id: Ident,
    pub args: Vec<Expr>,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub struct StmtExit {
    pub exit_code: Option<Expr>,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub struct StmtIf {
    pub cond: Expr,
    pub scope: Scope,
    pub _elif: Vec<Option<StmtElif>>,
    pub _else: Option<StmtElse>,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub struct StmtElif {
    pub cond: Expr,
    pub scope: Scope,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub struct StmtElse {
    pub scope: Scope,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub struct StmtWhile {
    pub cond: Expr,
    pub scope: Scope,
    pub loc: LocData,
}

#[derive(Debug, Clone)]
pub enum UnionNode {
    Ident(Ident),
    IntLit(IntLit),
    Expr(Box<Expr>),
    VarDecl(Rc<VarDecl>),
    Scope(Scope),
    Call(Call),
    StmtFn(StmtFn),
    StmtExit(StmtExit),
    StmtIf(StmtIf),
    StmtElif(StmtElif),
    StmtElse(StmtElse),
    StmtWhile(StmtWhile),
}

#[derive(Debug, Default)]
pub struct Program {
    pub sym: SymbolTable,
    pub stmts: Vec<UnionNode>,
}
