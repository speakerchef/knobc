use core::panic;
use std::{error::Error, fmt::Display, rc::Rc};

use crate::{
    ast::{self, UnionNode},
    diagnostics::DiagHandler,
    lexer::{self, Symbol, SymbolTable},
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
    //TODO: Assign method names to scopes
    //for classification
    pub nodes: Vec<KlirNode>,
}

impl Dump for KlirBlob {
    fn dump(&self) {
        for node in &self.nodes {
            match node {
                KlirNode::Alloca(alloca) => alloca.dump(),
                KlirNode::Store(store) => store.dump(),
                KlirNode::Define(define) => define.dump(),
                KlirNode::Call(call) => call.dump(),
                KlirNode::Expr(op) => op.dump(),
                KlirNode::Br(br) => br.dump(),
                KlirNode::Label(label) => label.dump(),
            }
        }
    }
}

// kinda like a basic block
#[derive(Debug, Default)]
pub struct ProgScope {
    pub id: String,
    pub ir: KlirBlob,
}

pub struct IrGenerator<'a> {
    prog: &'a mut ast::Program,
    _diag: &'a mut DiagHandler,
    sym: &'a mut SymbolTable,
    pub ir: KlirBlob,
    pub scopes: Vec<ProgScope>,
    pub target: Target,
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
            ArgType::Sym(id) => write!(f, "%{}", id),
            ArgType::Temp(temp_reg) => write!(f, "%{}", temp_reg),
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
        println!("    alloca {}, %{}", self.ty, self.dest.clone())
    }
}

#[derive(Debug)]
pub struct Define {
    pub return_ty: ast::Type,
    pub name: String,
    pub args: Option<Vec<(ArgType, ast::Type)>>,
}

impl Dump for Define {
    fn dump(&self) {
        print!("    define {}, %{}", self.return_ty, self.name);
        print!("(");
        if let Some(args) = &self.args {
            for arg in args {
                print!("{} ", arg.0);
                print!("{}", arg.1);
                if arg.ne(args.iter().last().unwrap()) {
                    print!(", ")
                }
            }
        }
        print!("):");
        println!();
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Call {
    pub return_ty: ast::Type,
    pub name: String,
    pub args: Option<Vec<(ArgType, ast::Type)>>,
}

impl Dump for Call {
    fn dump(&self) {
        print!("    call {}, {}", self.return_ty, self.name);
        print!("(");
        if let Some(args) = &self.args {
            for arg in args {
                print!("{} ", arg.0);
                print!("{}", arg.1);
                if arg.ne(args.iter().last().unwrap()) {
                    print!(", ")
                }
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
            "    {} {}, {}, {}, %{}",
            strop, self.ty, self.lhs, self.rhs, self.dest
        )
    }
}

#[derive(Debug)]
pub struct Label {
    pub name: String, // unconditional
}
impl Dump for Label {
    fn dump(&self) {
        println!("label {}:", self.name,)
    }
}

#[derive(Debug)]
pub struct Br {
    pub label: String,        // unconditional
    pub flag: Option<String>, // %result
}

impl Dump for Br {
    fn dump(&self) {
        println!(
            "    br {}, {}",
            self.label,
            if let Some(flag) = &self.flag {
                flag.clone()
            } else {
                "unconditional".to_string()
            }
        )
    }
}

#[derive(Debug)]
pub enum KlirNode {
    Alloca(Alloca),
    Store(Store),
    Define(Define),
    Call(Call),
    Expr(Expr),
    Br(Br),
    Label(Label),
}

impl IrGenerator<'_> {
    pub fn new<'a>(
        prog: &'a mut ast::Program,
        diag: &'a mut DiagHandler,
        sym: &'a mut SymbolTable,
    ) -> IrGenerator<'a> {
        IrGenerator {
            prog,
            _diag: diag,
            sym,
            ir: KlirBlob::default(),
            target: Target::UnknownArch,
            scopes: Vec::new(),
            reg_counter: 0,
            label_counter: 0,
        }
    }
    #[must_use]
    fn visit_expr(
        &mut self,
        expr: &ast::Expr,
        outer_scp: &mut ProgScope,
    ) -> (ast::AtomKind, Option<String /*temp register*/>) {
        if let (Some(lhs), Some(rhs)) = (&expr.lhs, &expr.rhs) {
            let lvalue = self.visit_expr(lhs, outer_scp);
            let rvalue = self.visit_expr(rhs, outer_scp);
            let dest = format!("t{}", self.reg_counter);
            self.reg_counter += 1; // NOTE: this

            let lhs;
            let rhs;
            match lvalue.0 {
                ast::AtomKind::Ident(id) => lhs = ArgType::Sym(id.name.to_string()),
                ast::AtomKind::IntLit(value) => lhs = ArgType::Imm(value.val),
                ast::AtomKind::None => lhs = ArgType::Temp(lvalue.1.as_ref().unwrap().clone()),
            }
            match rvalue.0 {
                ast::AtomKind::Ident(id) => rhs = ArgType::Sym(id.name.to_string()),
                ast::AtomKind::IntLit(value) => rhs = ArgType::Imm(value.val),
                ast::AtomKind::None => rhs = ArgType::Temp(rvalue.1.as_ref().unwrap().clone()),
            }
            // opnode
            outer_scp.ir.nodes.push(KlirNode::Expr(Expr {
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
    fn visit_decl(&mut self, decl: &ast::VarDecl, outer_scp: &mut ProgScope) {
        let (atom, temp) = self.visit_expr(decl.value.as_ref(), outer_scp);
        outer_scp.ir.nodes.push(KlirNode::Alloca(Alloca {
            ty: decl
                .ty
                .get()
                .expect("Could not resolve type at temp register allocation"),
            dest: format!("{}", decl.name),
        }));
        outer_scp.ir.nodes.push(KlirNode::Store(Store {
            ty: decl.ty.get().expect("failed to get decl.ty at visit_decl"),
            src: if let Some(ref temp) = temp {
                ArgType::Temp(temp.clone())
            } else {
                match atom {
                    ast::AtomKind::Ident(id) => ArgType::Sym(id.name.to_string()),
                    ast::AtomKind::IntLit(lit) => ArgType::Imm(lit.val),
                    ast::AtomKind::None => panic!("unexpected None atomkind"),
                }
            },
            dest: decl.name.to_string(),
        }))
    }
    fn visit_stmt_exit(&mut self, enode: &ast::StmtExit, outer_scp: &mut ProgScope) {
        let (atom, temp) = self.visit_expr(enode.exit_code.as_ref(), outer_scp);
        let ty = enode
            .exit_code
            .as_ref()
            .ty
            .get()
            .expect("Could not get type for exit_code");

        outer_scp.ir.nodes.push(KlirNode::Call(Call {
            return_ty: ast::Type::Void,
            name: String::from("_exit"),
            args: Some(vec![(
                if let Some(ref temp) = temp {
                    ArgType::Temp(temp.clone())
                } else {
                    match atom {
                        ast::AtomKind::Ident(id) => ArgType::Sym(id.name.to_string()),
                        ast::AtomKind::IntLit(val) => ArgType::Imm(val.val),
                        ast::AtomKind::None => panic!("unexpected None atomkind here"),
                    }
                },
                ty,
            )]),
        }))
    }
    fn visit_stmt_if(&mut self, stmt_if: &ast::StmtIf, outer_scp: &mut ProgScope) {
        let (atom, temp) = self.visit_expr(&stmt_if.cond, outer_scp);
        let result = if let Some(temp) = temp {
            temp.clone()
        } else {
            atom.to_string()
        };

        let if_body_label = format!("LABEL_IF_BODY_{}", self.label_counter);
        let init_elif_body_label = format!("LABEL_ELIF_INIT_{}", self.label_counter);
        let else_body_label = format!("LABEL_ELSE_BODY_{}", self.label_counter);
        let endif_label = format!("LABEL_ENDIF_{}", self.label_counter);
        self.label_counter += 1;

        // Conditional branch to if body
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
            label: if_body_label.clone(),
            flag: Some(result),
        }));
        // Branch to elif, else, or end
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
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
        outer_scp.ir.nodes.push(KlirNode::Label(Label {
            name: if_body_label.clone(),
        }));
        // Body scope
        self.visit_scope(&stmt_if.scope.stmts, outer_scp);
        // Jump to end
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
            label: endif_label.clone(),
            flag: None,
        }));

        // First elif init
        if !stmt_if._elif.is_empty() {
            outer_scp.ir.nodes.push(KlirNode::Label(Label {
                name: init_elif_body_label.clone(),
            }));
        }
        let mut elif_collection = stmt_if._elif.iter().flatten().peekable();
        while let Some(elif) = elif_collection.next() {
            let (atom, temp) = self.visit_expr(&elif.cond, outer_scp);
            let elif_result = if let Some(temp) = temp {
                temp.clone()
            } else {
                atom.to_string()
            };

            let elif_body_label = format!("LABEL_ELIF_BODY_{}", self.label_counter);
            self.label_counter += 1;
            let next_elif_body_label = format!("LABEL_ELIF_BODY_{}", self.label_counter);
            // Conditional branch to elif body
            outer_scp.ir.nodes.push(KlirNode::Br(Br {
                label: elif_body_label.clone(),
                flag: Some(elif_result),
            }));

            //fallback Branch to other elifs, else, or end
            outer_scp.ir.nodes.push(KlirNode::Br(Br {
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
            outer_scp.ir.nodes.push(KlirNode::Label(Label {
                name: elif_body_label.clone(),
            }));
            self.visit_scope(&elif.scope.stmts, outer_scp);
            // Jump to end
            outer_scp.ir.nodes.push(KlirNode::Br(Br {
                label: endif_label.clone(),
                flag: None,
            }));

            // other elifs labels
            if elif_collection.peek().is_some() {
                outer_scp.ir.nodes.push(KlirNode::Label(Label {
                    name: next_elif_body_label,
                }));
            }
        }

        // else body start
        outer_scp.ir.nodes.push(KlirNode::Label(Label {
            name: else_body_label.clone(),
        }));
        // else body scope
        if let Some(maybeelse) = &stmt_if._else {
            self.visit_scope(&maybeelse.scope.stmts, outer_scp);
        }
        // Jump to end
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
            label: endif_label.clone(),
            flag: None,
        }));
        // end start
        outer_scp.ir.nodes.push(KlirNode::Label(Label {
            name: endif_label.clone(),
        }));
    }

    fn visit_stmt_while(&mut self, stmt_while: &ast::StmtWhile, outer_scp: &mut ProgScope) {
        let start_while_label = format!("START_WHILE_{}", self.label_counter);
        let loop_while_label = format!("LOOP_WHILE_{}", self.label_counter);
        let end_while_label = format!("END_WHILE_{}", self.label_counter);

        outer_scp.ir.nodes.push(KlirNode::Label(Label {
            name: start_while_label.clone(),
        }));

        let (atom, temp) = self.visit_expr(&stmt_while.cond, outer_scp);
        self.label_counter += 1;
        let result = if let Some(temp) = temp {
            temp
        } else {
            atom.to_string()
        };
        // Conditional br to while loop
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
            label: loop_while_label.clone(),
            flag: Some(result),
        }));
        // unconditional br to end
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
            label: end_while_label.clone(),
            flag: None,
        }));
        // start while loop
        outer_scp.ir.nodes.push(KlirNode::Label(Label {
            name: loop_while_label.clone(),
        }));
        self.visit_scope(&stmt_while.scope.stmts, outer_scp);
        // unconditional br to start
        outer_scp.ir.nodes.push(KlirNode::Br(Br {
            label: start_while_label.clone(),
            flag: None,
        }));

        // end while loop
        outer_scp.ir.nodes.push(KlirNode::Label(Label {
            name: end_while_label.clone(),
        }));
    }

    fn visit_stmt_fn(&mut self, stmt_fn: &ast::StmtFn) -> ProgScope {
        let mut fn_scope = ProgScope::default();
        let fn_sym_as_str = self.sym.get(stmt_fn.name).unwrap();
        let fn_name_as_str = fn_sym_as_str.as_ref().to_string();
        fn_scope.id = fn_name_as_str.clone();
        fn_scope.ir.nodes.push(KlirNode::Define(Define {
            return_ty: stmt_fn.return_ty,
            name: fn_name_as_str,
            args: stmt_fn.args.as_ref().map(|args| {
                args.iter()
                    .map(|&(sym, ty)| (ArgType::Sym(sym.to_string()), ty))
                    .collect()
            }),
        }));
        self.visit_scope(&stmt_fn.body.stmts, &mut fn_scope);
        fn_scope
    }

    fn visit_fn_call(&mut self, call: &ast::Call, outer_scp: &mut ProgScope) {
        let fn_sym_as_str = self.sym.get(call.name).unwrap();
        let fn_name_as_str = fn_sym_as_str.as_ref().to_string();
        let mut nodes = std::mem::take(&mut outer_scp.ir.nodes);
        nodes.push(KlirNode::Call(Call {
            return_ty: call.return_ty,
            name: fn_name_as_str,
            args: {
                if let Some(call_args) = &call.args {
                    let mut arg_list = Vec::<(ArgType, ast::Type)>::new();
                    for expr in call_args {
                        let (atom, temp) = self.visit_expr(expr, outer_scp);
                        let argkind = if let Some(temp) = temp {
                            ArgType::Temp(temp)
                        } else {
                            match atom {
                                ast::AtomKind::Ident(id) => ArgType::Sym(id.name.to_string()),
                                ast::AtomKind::IntLit(lit) => ArgType::Imm(lit.val),
                                _ => panic!("Impossible for now"),
                            }
                        };
                        arg_list
                            .push((argkind, *expr.ty.get().as_ref().unwrap_or(&ast::Type::None)));
                    }
                    Some(arg_list)
                } else {
                    None
                }
            },
        }));
        outer_scp.ir.nodes = nodes;
    }

    fn visit_scope(&mut self, stmts: &[UnionNode], outer_scp: &mut ProgScope) {
        for stmt in stmts {
            match stmt {
                UnionNode::VarDecl(decl) => {
                    self.visit_decl(Rc::clone(decl).as_ref(), outer_scp);
                }
                UnionNode::StmtExit(enode) => {
                    self.visit_stmt_exit(enode, outer_scp);
                }
                UnionNode::Expr(expr) => {
                    let _ = self.visit_expr(expr, outer_scp);
                }
                UnionNode::StmtIf(stmt_if) => {
                    self.visit_stmt_if(stmt_if, outer_scp);
                }
                UnionNode::StmtWhile(stmt_while) => {
                    self.visit_stmt_while(stmt_while, outer_scp);
                }
                UnionNode::StmtFn(stmt_fn) => {
                    let fn_scope = self.visit_stmt_fn(Rc::clone(stmt_fn).as_ref());
                    self.scopes.push(fn_scope);
                }
                UnionNode::Call(call) => {
                    self.visit_fn_call(call, outer_scp);
                }
                UnionNode::Scope(scp) => {
                    self.visit_scope(&scp.stmts, outer_scp);
                }
                _ => todo!("No visitor for this node type in IRGen"),
            }
        }
    }

    pub fn emit_klir(&mut self) -> Result<(), Box<dyn Error>> {
        let arch = std::env::consts::ARCH;
        self.ir = KlirBlob::default();
        self.target = Target::from(arch);
        dbg!(&self.target);

        let stmts = std::mem::take(&mut self.prog.stmts);
        self.visit_scope(&stmts, &mut ProgScope::default());
        println!("IR: \n{:#?}", self.ir.nodes);
        println!("IR TEXT DUMP:");
        self.ir.dump();
        println!("SCOPE DUMP:\n{:#?}", self.scopes);
        self.prog.stmts = stmts;
        Ok(())
    }
}
