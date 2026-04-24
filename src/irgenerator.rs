use core::panic;
use std::{collections::HashMap, error::Error, fmt::Display, rc::Rc};

use crate::{
    ast::{self, UnionNode},
    diagnostics::DiagHandler,
    lexer::{self, Op},
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
pub struct KLIRBlob {
    data: String,
    target: Target,
    vars: HashMap<lexer::Symbol, (ast::Type, usize /* register counter */)>,
}

pub struct IrGenerator<'a> {
    prog: &'a mut ast::Program,
    _diag: &'a mut DiagHandler,
    ir: KLIRBlob,
    reg_counter: usize,
}

enum ArgType {
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

impl IrGenerator<'_> {
    pub fn new<'a>(prog: &'a mut ast::Program, diag: &'a mut DiagHandler) -> IrGenerator<'a> {
        IrGenerator {
            prog,
            _diag: diag,
            ir: KLIRBlob::default(),
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
            match expr.op {
                Op::Add => {
                    let dest = format!("%t{}", self.reg_counter);
                    self.ir.data.push_str(&format!(
                        "    alloca {}, {}\n",
                        expr.ty
                            .get()
                            .expect("Could not resolve type at dest register allocation"),
                        dest
                    ));
                    self.reg_counter += 1;
                    let mut lhs: ArgType;
                    let mut rhs: ArgType;
                    match lvalue.0 {
                        ast::AtomKind::Ident(id) => lhs = ArgType::Sym(format!("%{}", id.name)),
                        ast::AtomKind::IntLit(value) => lhs = ArgType::Imm(value.val),
                        _ => panic!("AOOEY"),
                    }
                    match rvalue.0 {
                        ast::AtomKind::Ident(id) => rhs = ArgType::Sym(format!("%{}", id.name)),
                        ast::AtomKind::IntLit(value) => rhs = ArgType::Imm(value.val),
                        _ => panic!("AOOEY but for rhs"),
                    }
                    if let Some(temp_idx) = lvalue.1 {
                        let tempname = format!("%{}", temp_idx);
                        self.ir.data.push_str(&format!(
                            "    alloca {}, {}\n",
                            expr.ty
                                .get()
                                .expect("Could not resolve type at temp register allocation"),
                            tempname
                        ));
                        lhs = ArgType::Temp(format!("%{}", temp_idx));
                    }
                    if let Some(temp_idx) = rvalue.1 {
                        let tempname = format!("%{}", temp_idx);
                        self.ir.data.push_str(&format!(
                            "    alloca {}, {}\n",
                            expr.ty
                                .get()
                                .expect("Could not resolve type at temp register allocation"),
                            tempname
                        ));
                        rhs = ArgType::Temp(format!("%{}", temp_idx));
                    }
                    self.ir.data.push_str(&format!(
                        "    add {}, {}, {}, {}\n",
                        expr.ty.get().expect("Couldnt get type. aooey"),
                        lhs,
                        rhs,
                        dest
                    ));
                    return (expr.atom.clone(), Some(dest));
                }
                Op::Nop => {
                    return (expr.atom.clone(), None);
                }
                _ => todo!("Not implemented yet"),
            };
        }

        if !matches!(expr.atom, ast::AtomKind::None) {
            return (expr.atom.clone(), None);
        }
        (ast::AtomKind::default(), None)
    }
    fn visit_decl(&mut self, decl: &ast::VarDecl) {
        let (atom, temp) = self.visit_expr(decl.value.as_ref());
        self.ir.data.push_str(&format!(
            "    store {}, {}, %{}\n",
            decl.ty
                .get()
                .expect("Could not resolve type at variable decl IR"),
            if let Some(ref temp) = temp {
                temp.clone()
            } else {
                format!("{}", atom)
            },
            decl.id.name
        ));
        // println!(
        //     "atom: {}, temp: {}",
        //     atom,
        //     temp.unwrap_or(String::from("No Temp"))
        // );
    }
    fn visit_stmt_exit(&mut self, enode: &ast::StmtExit) {
        let (atom, temp) = self.visit_expr(
            enode
                .exit_code
                .as_ref()
                .expect("Error: Could not get exit_code!"),
        );
        let ty = enode
            .exit_code
            .as_ref()
            .unwrap()
            .ty
            .get()
            .expect("Could not get type for exit_code");

        self.ir.data.push_str(&format!(
            "    call void exit({} {})",
            ty,
            if let Some(ref temp) = temp {
                temp.clone()
            } else {
                format!("{}", atom)
            }
        ));
    }
    pub fn emit_klir(&mut self) -> Result<(), Box<dyn Error>> {
        let arch = std::env::consts::ARCH;
        self.ir = KLIRBlob::default();
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
        println!("IR: \n{}", self.ir.data);
        self.prog.stmts = stmts;
        Ok(())
    }
}
