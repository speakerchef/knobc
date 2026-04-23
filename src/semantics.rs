use crate::{
    ast::{self, UnionNode},
    diagnostics::DiagHandler,
    lexer::{self, SymbolTable},
};
use std::{collections::HashMap, error::Error, rc::Rc};

pub struct Sema;
impl Sema {
    fn resolve_integer_resolution(
        val: i128,
        diag: &mut DiagHandler,
        loc: crate::lexer::LocData,
    ) -> Option<ast::Type> {
        if i32::try_from(val).is_ok() {
            Some(ast::Type::I32)
        } else if i64::try_from(val).is_ok() {
            Some(ast::Type::I64)
        } else {
            println!("Type did not resolve");
            diag.push_err(
                loc,
                &format!(
                    "integer literal `{}` exceeds the range of default type i64",
                    val
                ),
            );
            if u64::try_from(val).is_ok() {
                diag.push_note(
                    loc,
                    "consider annotating the type at declaration `let digit: u64 = ...`",
                );
            } else {
                diag.push_warn(loc,
                    &format!("use of excessively large integer literal `{}`; consider using a smaller value", val));
            }
            None
        }
    }

    fn visit_expr(
        expr: &ast::Expr,
        diag: &mut DiagHandler,
        sym: &mut SymbolTable,
        vars: &mut HashMap<lexer::Symbol, ast::Type>,
    ) {
        if let Some(lhs) = &expr.lhs {
            Sema::visit_expr(lhs, diag, sym, vars);
        }
        if let Some(rhs) = &expr.rhs {
            Sema::visit_expr(rhs, diag, sym, vars);
        }
        match expr.atom {
            ast::AtomKind::Ident(id) => {
                if sym.contains(id.name)
                    && let Some(&stored_type) = vars.get(&id.name)
                {
                    expr.ty.set(Some(stored_type));
                } else {
                    diag.push_err(
                        id.loc,
                        &format!(
                            "could not resolve symbol or type for `{}`",
                            sym.get(id.name).unwrap()
                        ),
                    );
                    expr.ty.set(None);
                }
            }
            ast::AtomKind::IntLit(lit) => {
                expr.ty
                    .set(Sema::resolve_integer_resolution(lit.val, diag, lit.loc));
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
                            diag.push_err(
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

    fn visit_decl(
        decl: &ast::VarDecl,
        diag: &mut DiagHandler,
        sym: &mut SymbolTable,
        vars: &mut HashMap<lexer::Symbol, ast::Type>,
    ) {
        Sema::visit_expr(decl.value.as_ref(), diag, sym, vars);
        if decl.value.ty.get().is_none() {
            diag.push_err(decl.loc, "could not resolve type for variable declaration");
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
                diag.push_err(
                    decl.id.loc,
                    &format!("expected {} and got {}", declared_type, inferred_type),
                );
                diag.push_note(
                    decl.id.loc,
                    &format!(
                        "consider changing `{}` to `{}`",
                        declared_type, inferred_type
                    ),
                );
            }
        }
        decl.ty.set(decl.value.ty.get());
        vars.insert(decl.id.name, decl.ty.get().unwrap());
    }

    fn visit_stmt_exit(
        enode: &ast::StmtExit,
        diag: &mut DiagHandler,
        sym: &mut SymbolTable,
        vars: &mut HashMap<lexer::Symbol, ast::Type>,
    ) {
        if let Some(exit_code) = enode.exit_code.as_ref() {
            Sema::visit_expr(exit_code, diag, sym, vars);
        }
    }

    pub fn validate_program(
        prog: &mut ast::Program,
        diag: &mut DiagHandler,
        sym: &mut SymbolTable,
    ) -> Result<(), Box<dyn Error>> {
        // dbg!("AST: {:#?}", &prog);
        // print!("Diagnostics at validate_program()");
        // diag.display_diagnostics();
        let vars: &mut HashMap<lexer::Symbol, ast::Type> = &mut HashMap::new();
        for stmt in &prog.stmts {
            match stmt {
                UnionNode::VarDecl(decl) => {
                    let rc = &mut Rc::clone(decl);
                    Sema::visit_decl(rc, diag, sym, vars);
                }
                UnionNode::Expr(expr) => {
                    Sema::visit_expr(expr.as_ref(), diag, sym, vars);
                }
                UnionNode::StmtExit(enode) => {
                    Sema::visit_stmt_exit(enode, diag, sym, vars);
                }
                _ => todo!("Semantic analysis for this nodetype is not implemented"),
            }
        }
        Ok(())
    }
}
