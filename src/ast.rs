use crate::lexer::{LocData, SymbolTable, TokenType};
use std::cell::{Cell, RefCell};
use std::fmt::Display;
use std::ops::DerefMut;
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Type {
    #[default]
    None,
    Int,
    Float,
    Char,
    Bool,
    String,
}

impl From<TokenType> for Type {
    fn from(value: TokenType) -> Self {
        match value {
            TokenType::KwInt => Type::Int,
            TokenType::KwFloat => Type::Float,
            TokenType::KwChar => Type::Char,
            TokenType::KwBool => Type::Bool,
            TokenType::KwString => Type::String,
            _ => Type::None,
        }
    }
}

impl Display for Type {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {
            Type::Int => write!(f, "Int"),
            Type::Float => write!(f, "Float"),
            Type::Char => write!(f, "Char"),
            Type::Bool => write!(f, "Bool"),
            Type::String => write!(f, "String"),
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
pub enum ExprKind {
    #[default]
    None,
    Ident(Ident),
    IntLit(IntLit),
}

#[derive(Debug, Default, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub lhs: Option<Box<Expr>>,
    pub rhs: Option<Box<Expr>>,
    pub ty: Cell<Option<Type>>,
    pub loc: LocData,
}

#[derive(Debug, Default, Clone)]
pub struct VarDecl {
    pub kind: VarType,
    pub id: Ident,
    pub decl_type: Option<Type>, // user defined
    pub ty: Cell<Option<Type>>,  // must exist before IR (inferred at sema)
    pub value: Box<Expr>,        // expr type must match local ty
    pub loc: LocData,
}

#[derive(Debug, Clone)]
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
    pub exit_code: Expr,
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

impl Default for Scope {
    fn default() -> Self {
        Self {
            stmts: Default::default(),
            vars: Default::default(),
            fns: Default::default(),
        }
    }
}
