use crate::{
    ast::{self, UnionNode},
    diagnostics::DiagHandler,
    lexer::{self},
};
use core::panic;
use std::{collections::HashMap, error::Error, rc::Rc};

type SemaScope = HashMap<lexer::Symbol, ast::VarType>;
pub struct Sema<'a> {
    prog: &'a mut ast::Program,
    diag: &'a mut DiagHandler,
    sym: &'a mut lexer::SymbolTable,
    cached_ty: HashMap<lexer::Symbol, ast::Type>,
    vars: SemaScope,
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
    #[allow(dead_code)]
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

    fn visit_stmt(&mut self, stmt: &ast::UnionNode, outer_scp: &mut SemaScope) {
        match stmt {
            UnionNode::VarDecl(decl) => {
                let rc = &mut Rc::clone(decl);
                if let Some(existing_decl_kind) = outer_scp.get(&decl.id.name)
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
                } else if decl.is_reassign && outer_scp.get(&decl.id.name).is_none() {
                    self.diag.push_err(
                        decl.id.loc,
                        &format!(
                            "use of undeclared identifier `{}`",
                            self.sym.get(decl.id.name).unwrap()
                        ),
                    );
                }
                self.visit_decl(rc, outer_scp);
            }
            UnionNode::Expr(expr) => {
                self.visit_expr(expr.as_ref(), outer_scp);
            }
            UnionNode::StmtExit(enode) => {
                self.visit_stmt_exit(enode, outer_scp);
            }
            UnionNode::StmtIf(stmt_if) => {
                self.visit_stmt_if(stmt_if, outer_scp);
            }
            UnionNode::StmtElif(v) => {
                self.diag
                    .push_err(v.loc, "expected accompanying `if` statement for `elif`");
            }
            UnionNode::StmtElse(v) => {
                self.diag
                    .push_err(v.loc, "expected accompanying `if` statement for `else`");
            }
            UnionNode::StmtWhile(stmt_while) => {
                self.visit_stmt_while(stmt_while, outer_scp);
            }
            UnionNode::Scope(scp) => {
                self.visit_scope(scp, outer_scp);
            }
            _ => todo!("Semantic analysis for this nodetype is not implemented"),
        }
    }

    fn visit_expr(&mut self, expr: &ast::Expr, outer_scp: &mut SemaScope) {
        if let Some(lhs) = &expr.lhs {
            self.visit_expr(lhs, outer_scp);
        }
        if let Some(rhs) = &expr.rhs {
            self.visit_expr(rhs, outer_scp);
        }
        match expr.atom {
            ast::AtomKind::Ident(id) => {
                if !outer_scp.contains_key(&id.name) {
                    self.diag.push_err(
                        id.loc,
                        &format!(
                            "use of undeclared identifier `{}`",
                            self.sym.get(id.name).unwrap()
                        ),
                    );
                }
                if let Some(&cached_ty) = self.cached_ty.get(&id.name) {
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
    fn visit_decl(&mut self, decl: &ast::VarDecl, outer_scp: &mut SemaScope) {
        self.visit_expr(decl.value.as_ref(), outer_scp);
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
        // self.vars.insert(decl.id.name, decl.kind);
        outer_scp.insert(decl.id.name, decl.kind);
        self.cached_ty.insert(decl.id.name, decl.ty.get().unwrap());
    }

    fn visit_scope(&mut self, scope: &ast::Scope, outer_scp: &mut SemaScope) {
        let mut loc_scp = SemaScope::new();
        outer_scp.iter().for_each(|(&k, &v)| {
            loc_scp.insert(k, v);
        });
        for stmt in &scope.stmts {
            self.visit_stmt(stmt, &mut loc_scp);
        }
    }

    fn visit_stmt_exit(&mut self, enode: &ast::StmtExit, outer_scp: &mut SemaScope) {
        self.visit_expr(&enode.exit_code, outer_scp);
    }
    fn visit_stmt_if(&mut self, stmt_if: &ast::StmtIf, outer_scp: &mut SemaScope) {
        self.visit_expr(&stmt_if.cond, outer_scp);
        self.visit_scope(&stmt_if.scope, outer_scp);

        for elif in stmt_if._elif.iter().flatten() {
            self.visit_expr(&elif.cond, outer_scp);
            self.visit_scope(&elif.scope, outer_scp);
        }
        if let Some(maybe_else) = &stmt_if._else {
            self.visit_scope(&maybe_else.scope, outer_scp);
        }
    }
    fn visit_stmt_while(&mut self, stmt_while: &ast::StmtWhile, outer_scp: &mut SemaScope) {
        self.visit_expr(&stmt_while.cond, outer_scp);
        self.visit_scope(&stmt_while.scope, outer_scp);
    }

    pub fn validate_program(&mut self) -> Result<(), Box<dyn Error>> {
        let mut global_scope = SemaScope::new();
        let stmts = std::mem::take(&mut self.prog.stmts);
        for stmt in &stmts {
            self.visit_stmt(stmt, &mut global_scope);
        }
        self.prog.stmts = stmts;
        println!("AST: \n{:#?}", self.prog.stmts);
        Ok(())
    }
}
