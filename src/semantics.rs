use crate::{
    ast::{self, UnionNode},
    diagnostics::DiagHandler,
    lexer::{self},
};
use core::panic;
use std::{collections::HashMap, error::Error, rc::Rc};

pub struct Sema<'a> {
    prog: &'a mut ast::Program,
    diag: &'a mut DiagHandler,
    sym: &'a mut lexer::SymbolTable,
    cached_ty: HashMap<lexer::Symbol, ast::Type>,
    vars: HashMap<lexer::Symbol, ast::VarType>,
}
impl Sema<'_> {
    pub fn new<'a>(
        prog: &'a mut ast::Program,
        diag: &'a mut DiagHandler,
        sym: &'a mut lexer::SymbolTable,
    ) -> Sema<'a> {
        Sema {
            prog,
            diag,
            sym,
            cached_ty: HashMap::new(),
            vars: HashMap::new(),
        }
    }
    fn default_integer_resolution(
        &mut self,
        val: i128,
        loc: crate::lexer::LocData,
    ) -> Option<ast::Type> {
        if i32::try_from(val).is_ok() {
            Some(ast::Type::I32)
        } else if i64::try_from(val).is_ok() {
            Some(ast::Type::I64)
        } else {
            println!("Type did not resolve");
            self.diag.push_err(
                loc,
                &format!(
                    "integer literal `{}` exceeds the range of default type i64",
                    val
                ),
            );
            if u64::try_from(val).is_ok() {
                self.diag.push_note(
                    loc,
                    "consider annotating the type at declaration `let digit: u64 = ...`",
                );
            } else {
                self.diag.push_warn(loc,
                    &format!("use of excessively large integer literal `{}`; consider using a smaller value", val));
            }
            None
        }
    }
    fn resolve_integer_resolution(
        &mut self,
        val: i128,
        loc: crate::lexer::LocData,
    ) -> Option<ast::Type> {
        match val {
            _ if i8::try_from(val).is_ok() => Some(ast::Type::I8),
            _ if i16::try_from(val).is_ok() => Some(ast::Type::I16),
            _ if i32::try_from(val).is_ok() => Some(ast::Type::I32),
            _ if i64::try_from(val).is_ok() => Some(ast::Type::I64),
            _ if u8::try_from(val).is_ok() => Some(ast::Type::U8),
            _ if u16::try_from(val).is_ok() => Some(ast::Type::U16),
            _ if u32::try_from(val).is_ok() => Some(ast::Type::U32),
            _ if u64::try_from(val).is_ok() => Some(ast::Type::U64),
            _ => {
                println!("Type did not resolve");
                self.diag.push_err(
                    loc,
                    &format!(
                        "integer literal `{}` exceeds the range of default type i64",
                        val
                    ),
                );
                if u64::try_from(val).is_ok() {
                    self.diag.push_note(
                        loc,
                        "consider annotating the type at declaration `let digit: u64 = ...`",
                    );
                } else {
                    self.diag.push_warn(loc,
                    &format!("use of excessively large integer literal `{}`; consider using a smaller value", val));
                }
                None
            }
        }
    }

    fn set_all_types_expr(&mut self, expr: &ast::Expr, ty: ast::Type) {
        if let Some(lhs) = &expr.lhs {
            self.set_all_types_expr(lhs, ty);
        }
        if let Some(rhs) = &expr.rhs {
            self.set_all_types_expr(rhs, ty);
        }
        expr.ty.set(Some(ty));
    }

    fn visit_stmt(&mut self, stmt: &ast::UnionNode) {
        match stmt {
            UnionNode::VarDecl(decl) => {
                let rc = &mut Rc::clone(decl);
                if let Some(existing_decl_kind) = self.vars.get(&decl.id.name)
                    && matches!(existing_decl_kind, ast::VarType::Let)
                {
                    self.diag.push_err(
                        decl.id.loc,
                        &format!(
                            "cannot re-assign `{}` declared with `let`",
                            self.sym.get(decl.id.name).unwrap()
                        ),
                    );
                    self.diag
                        .push_note(decl.id.loc, "consider using `mut` instead");
                }
                self.visit_decl(rc);
            }
            UnionNode::Expr(expr) => {
                self.visit_expr(expr.as_ref());
            }
            UnionNode::StmtExit(enode) => {
                self.visit_stmt_exit(enode);
            }
            UnionNode::StmtIf(stmt_if) => {
                self.visit_stmt_if(stmt_if);
            }
            UnionNode::StmtElif(v) => {
                self.diag
                    .push_err(v.loc, "expected accompanying `if` statement for `elif`");
            }
            UnionNode::StmtElse(v) => {
                self.diag
                    .push_err(v.loc, "expected accompanying `if` statement for `else`");
            }
            _ => todo!("Semantic analysis for this nodetype is not implemented"),
        }
    }

    fn visit_expr(&mut self, expr: &ast::Expr) {
        if let Some(lhs) = &expr.lhs {
            self.visit_expr(lhs);
        }
        if let Some(rhs) = &expr.rhs {
            self.visit_expr(rhs);
        }
        match expr.atom {
            ast::AtomKind::Ident(id) => {
                if let Some(&cached_ty) = self.cached_ty.get(&id.name) {
                    println!("Cached Type: {}", cached_ty);
                    expr.ty.set(Some(cached_ty));
                } else {
                    self.diag.push_err(
                        id.loc,
                        &format!(
                            "could not resolve symbol or type for `{}`",
                            self.sym.get(id.name).unwrap()
                        ),
                    );
                    expr.ty.set(None);
                }
            }
            ast::AtomKind::IntLit(lit) => {
                expr.ty
                    .set(self.default_integer_resolution(lit.val, lit.loc));
            }
            ast::AtomKind::None => {
                expr.ty
                    .set(if let (Some(lhs), Some(rhs)) = (&expr.lhs, &expr.rhs) {
                        let lty = lhs.ty.get().unwrap();
                        let rty = rhs.ty.get().unwrap();
                        let mut type_to_return = lty; // default to lhs type

                        /* Check if we can coerce both values
                         * if they're not the same type
                         */
                        if lty != rty
                            && !lty.is_digit_convertible_to(&rty)
                            && !rty.is_digit_convertible_to(&lty)
                        {
                            type_to_return = ast::Type::default();
                            self.diag.push_err(
                                lhs.loc,
                                &format!(
                                    "incompatible types found in expression `{}` & `{}`",
                                    lty, rty
                                ),
                            );
                        }
                        Some(type_to_return)
                    } else if let Some(lhs) = &expr.lhs {
                        lhs.ty.get()
                    } else if let Some(rhs) = &expr.rhs {
                        rhs.ty.get()
                    } else {
                        None
                    });
            }
        }
    }
    fn visit_decl(&mut self, decl: &ast::VarDecl) {
        self.visit_expr(decl.value.as_ref());
        if decl.value.ty.get().is_none() {
            self.diag
                .push_err(decl.loc, "could not resolve type for variable declaration");
        } else {
            // Set declaration type to inner expression type
            decl.ty.set(decl.value.ty.get());
        }
        if let (Some(declared_type), Some(inferred_type)) = (decl.decl_type, decl.ty.get())
            && declared_type != inferred_type
        {
            if inferred_type.is_digit_convertible_to(&declared_type) {
                decl.value.ty.set(Some(declared_type));
            } else {
                self.diag.push_err(
                    decl.id.loc,
                    &format!("expected {} and got {}", declared_type, inferred_type),
                );
                self.diag.push_note(
                    decl.id.loc,
                    &format!(
                        "consider changing `{}` to `{}`",
                        declared_type, inferred_type
                    ),
                );
            }
        } else if let Some(inferred_type) = decl.ty.get()
            && decl.decl_type.is_none()
        {
            if inferred_type.is_digit_convertible_to(&ast::Type::I32) {
                decl.ty.set(Some(ast::Type::I32));
            } else if inferred_type.is_digit_convertible_to(&ast::Type::I64) {
                decl.ty.set(Some(ast::Type::I64));
            } else {
                panic!("Could not set default type");
            }
        }
        self.set_all_types_expr(decl.value.as_ref(), decl.ty.get().unwrap());
        self.vars.insert(decl.id.name, decl.kind);
        self.cached_ty.insert(decl.id.name, decl.ty.get().unwrap());
    }

    fn visit_scope(&mut self, scope: &ast::Scope) {
        for stmt in &scope.stmts {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt_exit(&mut self, enode: &ast::StmtExit) {
        self.visit_expr(&enode.exit_code);
    }
    fn visit_stmt_if(&mut self, stmt_if: &ast::StmtIf) {
        self.visit_expr(&stmt_if.cond);
        self.visit_scope(&stmt_if.scope);

        for elif in stmt_if._elif.iter().flatten() {
            self.visit_expr(&elif.cond);
            self.visit_scope(&elif.scope);
        }
        if let Some(maybe_else) = &stmt_if._else {
            self.visit_scope(&maybe_else.scope);
        }
    }

    pub fn validate_program(&mut self) -> Result<(), Box<dyn Error>> {
        let stmts = std::mem::take(&mut self.prog.stmts);
        for stmt in &stmts {
            self.visit_stmt(stmt);
        }
        self.prog.stmts = stmts;
        println!("AST: {:#?}", self.prog.stmts);
        Ok(())
    }
}
