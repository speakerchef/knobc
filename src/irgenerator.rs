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
                KlirNode::Br(br) => br.dump(),
                KlirNode::Label(label) => label.dump(),
            }
        }
    }
}

pub struct IrGenerator<'a> {
    prog: &'a mut ast::Program,
    _diag: &'a mut DiagHandler,
    pub ir: KlirBlob,
    reg_counter: usize,
    label_counter: usize,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ArgType {
    Sym(String),
    Temp(String),
    Imm(i128),
}

impl Display for ArgType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgType::Sym(id) => write!(f, "{}", id),
            ArgType::Temp(temp_reg) => write!(f, "{}", temp_reg),
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
    pub args: Vec<(ast::Type, ArgType)>,
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
    pub src: ArgType,
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
pub struct Cmp {
    pub ty: ast::Type,
    pub flag: String, // %result
    pub jump: String,
}

#[derive(Debug)]
pub struct Label {
    pub name: String, // unconditional
}
impl Dump for Label {
    fn dump(&self) {
        println!("    {}", self.name,)
    }
}

#[derive(Debug)]
pub struct Br {
    pub label: String,        // unconditional
    pub flag: Option<String>, // %result
}

impl Dump for Br {
    fn dump(&self) {
        println!("    br {}", self.label,)
    }
}

#[derive(Debug)]
pub enum KlirNode {
    Alloca(Alloca),
    Store(Store),
    Call(Call),
    Expr(Expr),
    Br(Br),
    Label(Label),
}

impl IrGenerator<'_> {
    pub fn new<'a>(prog: &'a mut ast::Program, diag: &'a mut DiagHandler) -> IrGenerator<'a> {
        IrGenerator {
            prog,
            _diag: diag,
            ir: KlirBlob::default(),
            reg_counter: 0,
            label_counter: 0,
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

            let lhs;
            let rhs;
            match lvalue.0 {
                ast::AtomKind::Ident(id) => lhs = ArgType::Sym(format!("{}", id.name)),
                ast::AtomKind::IntLit(value) => lhs = ArgType::Imm(value.val),
                ast::AtomKind::None => lhs = ArgType::Temp(lvalue.1.as_ref().unwrap().clone()),
            }
            match rvalue.0 {
                ast::AtomKind::Ident(id) => rhs = ArgType::Sym(format!("{}", id.name)),
                ast::AtomKind::IntLit(value) => rhs = ArgType::Imm(value.val),
                ast::AtomKind::None => rhs = ArgType::Temp(rvalue.1.as_ref().unwrap().clone()),
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
        self.ir.nodes.push(KlirNode::Alloca(Alloca {
            ty: decl
                .ty
                .get()
                .expect("Could not resolve type at temp register allocation"),
            dest: format!("{}", decl.id.name),
        }));
        self.ir.nodes.push(KlirNode::Store(Store {
            ty: decl.ty.get().expect(""),
            src: if let Some(ref temp) = temp {
                ArgType::Temp(temp.clone())
            } else {
                match atom {
                    ast::AtomKind::Ident(id) => ArgType::Sym(format!("{}", id.name)),
                    ast::AtomKind::IntLit(lit) => ArgType::Imm(lit.val),
                    ast::AtomKind::None => panic!("unexpected None atomkind"),
                }
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
                    ArgType::Temp(temp.clone())
                } else {
                    match atom {
                        ast::AtomKind::Ident(id) => ArgType::Sym(format!("{}", id.name)),
                        ast::AtomKind::IntLit(val) => ArgType::Imm(val.val),
                        ast::AtomKind::None => panic!("unexpected None atomkind here"),
                    }
                },
            )],
        }))
    }
    fn visit_stmt_if(&mut self, stmt_if: &ast::StmtIf) {
        let (atom, temp) = self.visit_expr(&stmt_if.cond);
        let result = if let Some(temp) = temp {
            temp.clone()
        } else {
            format!("{}", atom)
        };

        let if_body_label = format!("LABEL_IF_BODY_{}", self.label_counter);
        let init_elif_body_label = format!("LABEL_ELIF_INIT_{}", self.label_counter);
        let else_body_label = format!("LABEL_ELSE_BODY_{}", self.label_counter);
        let endif_label = format!("LABEL_ENDIF_{}", self.label_counter);
        self.label_counter += 1;

        // Conditional branch to if body
        self.ir.nodes.push(KlirNode::Br(Br {
            label: if_body_label.clone(),
            flag: Some(result),
        }));
        // Branch to elif, else, or end
        self.ir.nodes.push(KlirNode::Br(Br {
            label: if !stmt_if._elif.is_empty() {
                init_elif_body_label.clone()
            } else if stmt_if._else.is_some() {
                else_body_label.clone()
            } else {
                endif_label.clone()
            },
            flag: None,
        }));
        // If body start
        self.ir.nodes.push(KlirNode::Label(Label {
            name: if_body_label.clone(),
        }));
        // Body scope
        self.visit_scope(&stmt_if.scope.stmts);
        // Jump to end
        self.ir.nodes.push(KlirNode::Br(Br {
            label: endif_label.clone(),
            flag: None,
        }));

        // First elif init
        if !stmt_if._elif.is_empty() {
            self.ir.nodes.push(KlirNode::Label(Label {
                name: init_elif_body_label.clone(),
            }));
        }
        let mut elif_collection = stmt_if._elif.iter().flatten().peekable();
        while let Some(elif) = elif_collection.next() {
            let (atom, temp) = self.visit_expr(&elif.cond);
            let elif_result = if let Some(temp) = temp {
                temp.clone()
            } else {
                format!("{}", atom)
            };

            let elif_body_label = format!("LABEL_ELIF_BODY_{}", self.label_counter);
            self.label_counter += 1;
            let next_elif_body_label = format!("LABEL_ELIF_BODY_{}", self.label_counter);
            // Conditional branch to elif body
            self.ir.nodes.push(KlirNode::Br(Br {
                label: elif_body_label.clone(),
                flag: Some(elif_result),
            }));

            //fallback Branch to other elifs, else, or end
            self.ir.nodes.push(KlirNode::Br(Br {
                label: if elif_collection.peek().is_some() {
                    next_elif_body_label.clone()
                } else if stmt_if._else.is_some() {
                    else_body_label.clone()
                } else {
                    endif_label.clone()
                },
                flag: None,
            }));
            // current elif body label
            self.ir.nodes.push(KlirNode::Label(Label {
                name: elif_body_label.clone(),
            }));
            self.visit_scope(&elif.scope.stmts);
            // Jump to end
            self.ir.nodes.push(KlirNode::Br(Br {
                label: endif_label.clone(),
                flag: None,
            }));

            // other elifs labels
            if elif_collection.peek().is_some() {
                self.ir.nodes.push(KlirNode::Label(Label {
                    name: next_elif_body_label,
                }));
            }
        }

        // else body start
        self.ir.nodes.push(KlirNode::Label(Label {
            name: else_body_label.clone(),
        }));
        // else body scope
        if let Some(maybeelse) = &stmt_if._else {
            self.visit_scope(&maybeelse.scope.stmts);
        }
        // Jump to end
        self.ir.nodes.push(KlirNode::Br(Br {
            label: endif_label.clone(),
            flag: None,
        }));
        // end start
        self.ir.nodes.push(KlirNode::Label(Label {
            name: endif_label.clone(),
        }));
    }

    fn visit_scope(&mut self, stmts: &[UnionNode]) {
        for stmt in stmts {
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
                UnionNode::StmtIf(stmt_if) => {
                    self.visit_stmt_if(stmt_if);
                }
                UnionNode::Scope(scp) => {
                    self.visit_scope(&scp.stmts);
                }
                _ => todo!("No visitor for this node type in IRGen"),
            }
        }
    }

    pub fn emit_klir(&mut self) -> Result<(), Box<dyn Error>> {
        let arch = std::env::consts::ARCH;
        self.ir = KlirBlob::default();
        self.ir.target = Target::from(arch);
        dbg!(&self.ir.target);

        let stmts = std::mem::take(&mut self.prog.stmts);
        self.visit_scope(&stmts);
        println!("IR: \n{:#?}", self.ir.nodes);
        println!("IR TEXT DUMP:");
        self.ir.dump();
        self.prog.stmts = stmts;
        Ok(())
    }
}
