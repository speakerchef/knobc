use core::panic;
use std::{error::Error, fmt::Display, rc::Rc};

use crate::{
    ast::{self, UnionNode},
    diagnostics::DiagHandler,
    lexer,
};

#[derive(Default, Debug)]
pub enum Target {
    #[default]
    UnknownArch,
    Aarch64,
    X86_64,
}

impl From<&str> for Target {
    fn from(value: &str) -> Self {
        match value {
            "aarch64" => Self::Aarch64,
            "x86_64" => Self::X86_64,
            _ => Self::X86_64,
        }
    }
}

#[derive(Default, Debug)]
pub struct KlirBlob {
    pub text: String,
    pub nodes: Vec<KlirNode>,
    pub target: Target,
}

impl Dump for KlirBlob {
    fn dump(&self) {
        for node in &self.nodes {
            match node {
                KlirNode::Alloca(alloca) => alloca.dump(),
                KlirNode::Store(store) => store.dump(),
                KlirNode::Call(call) => call.dump(),
                KlirNode::Expr(op) => op.dump(),
            }
        }
    }
}

pub struct IrGenerator<'a> {
    prog: &'a mut ast::Program,
    _diag: &'a mut DiagHandler,
    pub ir: KlirBlob,
    reg_counter: usize,
}

#[derive(Debug)]
pub enum ArgType {
    Sym(String),
    Temp(String),
    Imm(i128),
}

impl Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::Sym(id) => write!(f, "{}", id),
            ArgType::Temp(temp_reg) => write!(f, "t{}", temp_reg),
            ArgType::Imm(val) => write!(f, "{}", val),
        }
    }
}

trait Dump {
    fn dump(&self);
}

#[derive(Debug)]
pub struct Alloca {
    pub ty: ast::Type,
    pub dest: String, // %var
}

impl Dump for Alloca {
    fn dump(&self) {
        println!("    alloca {}, {}", self.ty, self.dest.clone())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Call {
    pub return_ty: ast::Type,
    pub methodname: String,
    pub args: Vec<(ast::Type, String /* argname */)>,
}

impl Dump for Call {
    fn dump(&self) {
        print!("    call {}, {}", self.return_ty, self.methodname);
        print!("(");
        for arg in &self.args {
            print!("{} ", arg.0);
            print!("{}", arg.1);
            if arg.ne(self.args.iter().last().unwrap()) {
                print!(",")
            }
        }
        print!(")");
        println!();
    }
}

#[derive(Debug)]
pub struct Store {
    pub ty: ast::Type,
    pub src: String,
    pub dest: String,
}

impl Dump for Store {
    fn dump(&self) {
        println!("    store {}, {}, {}", self.ty, self.src, self.dest)
    }
}

#[derive(Debug)]
pub struct Expr {
    pub ty: ast::Type,
    pub lhs: ArgType,
    pub rhs: ArgType,
    pub op: lexer::Op,
    pub dest: String,
}

impl Dump for Expr {
    fn dump(&self) {
        let strop = match self.op {
            lexer::Op::Add => "add",
            lexer::Op::Sub => "sub",
            lexer::Op::Mul => "mul",
            lexer::Op::Div => "div",
            lexer::Op::Mod => "mod",
            lexer::Op::Pwr => "pwr",
            lexer::Op::BwAnd => "and",
            lexer::Op::BwOr => "or",
            lexer::Op::BwXor => "xor",
            lexer::Op::BwNot => "not",
            lexer::Op::LgAnd => "lgand",
            lexer::Op::LgOr => "lgor",
            lexer::Op::LgNot => "lgnot",
            lexer::Op::Lt => "lt",
            lexer::Op::Gt => "gt",
            lexer::Op::Lte => "lte",
            lexer::Op::Gte => "gte",
            lexer::Op::Eq => "eq",
            lexer::Op::Neq => "neq",
            lexer::Op::Lsl => "lsl",
            lexer::Op::Lsr => "lsr",
            lexer::Op::Asr => "asr",
            _ => panic!("unimplemented op"),
        };
        println!(
            "    {} {}, {}, {}, {}",
            strop, self.ty, self.lhs, self.rhs, self.dest
        )
    }
}

#[derive(Debug)]
pub enum KlirNode {
    Alloca(Alloca),
    Store(Store),
    Call(Call),
    Expr(Expr),
}

impl IrGenerator<'_> {
    pub fn new<'a>(prog: &'a mut ast::Program, diag: &'a mut DiagHandler) -> IrGenerator<'a> {
        IrGenerator {
            prog,
            _diag: diag,
            ir: KlirBlob::default(),
            reg_counter: 0,
        }
    }
    #[must_use]
    fn visit_expr(
        &mut self,
        expr: &ast::Expr,
    ) -> (ast::AtomKind, Option<String /*temp register*/>) {
        if let (Some(lhs), Some(rhs)) = (&expr.lhs, &expr.rhs) {
            let lvalue = self.visit_expr(lhs);
            let rvalue = self.visit_expr(rhs);
            let dest = format!("t{}", self.reg_counter);

            // Node Data
            self.ir.nodes.push(KlirNode::Alloca(Alloca {
                ty: expr
                    .ty
                    .get()
                    .expect("Could not resolve type at dest register allocation"),
                dest: dest.clone(),
            }));
            self.reg_counter += 1;

            let mut lhs: ArgType;
            let mut rhs: ArgType;

            match lvalue.0 {
                ast::AtomKind::Ident(id) => lhs = ArgType::Sym(format!("{}", id.name)),
                ast::AtomKind::IntLit(value) => lhs = ArgType::Imm(value.val),
                _ => panic!("AOOEY"),
            }
            match rvalue.0 {
                ast::AtomKind::Ident(id) => rhs = ArgType::Sym(format!("{}", id.name)),
                ast::AtomKind::IntLit(value) => rhs = ArgType::Imm(value.val),
                _ => panic!("AOOEY but for rhs"),
            }

            if let Some(temp_idx) = lvalue.1 {
                self.ir.nodes.push(KlirNode::Alloca(Alloca {
                    ty: expr
                        .ty
                        .get()
                        .expect("Could not resolve type at temp register allocation"),
                    dest: temp_idx.clone(),
                }));
                lhs = ArgType::Temp(temp_idx);
            }

            if let Some(temp_idx) = rvalue.1 {
                self.ir.nodes.push(KlirNode::Alloca(Alloca {
                    ty: expr
                        .ty
                        .get()
                        .expect("Could not resolve type at temp register allocation"),
                    dest: temp_idx.clone(),
                }));
                rhs = ArgType::Temp(temp_idx);
            }

            // opnode
            self.ir.nodes.push(KlirNode::Expr(Expr {
                ty: expr
                    .ty
                    .get()
                    .expect("Could not resolve type at temp register allocation"),
                lhs,
                rhs,
                op: if matches!(expr.op, lexer::Op::Lsr) && expr.ty.get().unwrap().is_signed() {
                    lexer::Op::Asr
                } else {
                    expr.op
                },
                dest: dest.clone(),
            }));
            return (expr.atom.clone(), Some(dest));
        }

        if !matches!(expr.atom, ast::AtomKind::None) {
            return (expr.atom.clone(), None);
        }
        (ast::AtomKind::default(), None)
    }
    fn visit_decl(&mut self, decl: &ast::VarDecl) {
        let (atom, temp) = self.visit_expr(decl.value.as_ref());
        self.ir.nodes.push(KlirNode::Store(Store {
            ty: decl.ty.get().expect(""),
            src: if let Some(ref temp) = temp {
                temp.clone()
            } else {
                format!("{}", atom)
            },
            dest: format!("{}", decl.id.name),
        }))
    }
    fn visit_stmt_exit(&mut self, enode: &ast::StmtExit) {
        let (atom, temp) = self.visit_expr(enode.exit_code.as_ref());
        let ty = enode
            .exit_code
            .as_ref()
            .ty
            .get()
            .expect("Could not get type for exit_code");

        self.ir.nodes.push(KlirNode::Call(Call {
            return_ty: ast::Type::Void,
            methodname: String::from("_exit"),
            args: vec![(
                ty,
                if let Some(ref temp) = temp {
                    temp.clone()
                } else {
                    format!("{}", atom)
                },
            )],
        }))
    }
    pub fn emit_klir(&mut self) -> Result<(), Box<dyn Error>> {
        let arch = std::env::consts::ARCH;
        self.ir = KlirBlob::default();
        self.ir.target = Target::from(arch);
        dbg!(&self.ir.target);

        let stmts = std::mem::take(&mut self.prog.stmts);
        for stmt in &stmts {
            match stmt {
                UnionNode::VarDecl(decl) => {
                    self.visit_decl(Rc::clone(decl).as_ref());
                }
                UnionNode::StmtExit(enode) => {
                    self.visit_stmt_exit(enode);
                }
                UnionNode::Expr(expr) => {
                    let _ = self.visit_expr(expr);
                }
                _ => todo!("No visitor for this node type in IRGen"),
            }
        }
        // println!("IR: \n{}", self.ir.data);
        println!("IR: \n{:#?}", self.ir.nodes);
        println!("IR String Dump: \n");
        self.ir.dump();
        self.prog.stmts = stmts;
        Ok(())
    }
}
