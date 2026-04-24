use std::{cell::Cell, error::Error, rc::Rc};

use crate::{
    ast::{self, AtomKind, Type, VarType},
    diagnostics::DiagHandler,
    lexer::{Lexer, LocData, Op, Token, TokenType},
    traits::Iter,
};

pub struct Parser<'a> {
    program: ast::Program,
    diagnostics: &'a mut DiagHandler,
    lex: &'a mut Lexer,
}

impl Parser<'_> {
    pub fn new<'a>(
        lex: &'a mut Lexer,
        diagnostics: &'a mut DiagHandler,
    ) -> Result<Parser<'a>, Box<dyn Error>> {
        Ok(Parser {
            program: ast::Program::default(),
            diagnostics,
            lex,
        })
    }

    #[must_use]
    fn validate_tok(&mut self, kind: TokenType) -> bool {
        if let Some(&tok) = self.lex.peek()
            && tok.kind == kind
        {
            return true;
        }
        false
    }

    fn get_infix_bp(&self, op: Op) -> (f64, f64) {
        match op {
            // right associative
            Op::Asgn => (1.1, 1.0),

            Op::LgOr => (2.0, 2.1),
            Op::LgAnd => (3.0, 3.1),

            Op::BwOr => (4.0, 4.1),
            Op::BwXor => (5.0, 5.1),
            Op::BwAnd => (6.0, 6.1),

            Op::Eq | Op::Neq => (7.0, 7.1),

            Op::Lt | Op::Gt | Op::Lte | Op::Gte => (8.0, 8.1),

            Op::Lsl | Op::Lsr => (9.0, 9.1),

            Op::Sub | Op::Add => (10.0, 10.1),
            Op::Mod | Op::Div | Op::Mul => (11.0, 11.1),

            // right associative
            Op::Pwr => (12.1, 12.0),
            _ => todo!("Invalid Op or received unary op"),
        }
    }

    fn parse_var_decl(&mut self, t: Token) -> Option<ast::VarDecl> {
        let mut decl = ast::VarDecl {
            loc: t.loc,
            kind: if matches!(t.kind, TokenType::KwLet) {
                VarType::Let
            } else {
                VarType::Mut
            },
            ..Default::default()
        };

        // Get identifier
        if let Some(&idtok) = self.lex.peek() {
            let TokenType::VarIdent(sym) = idtok.kind else {
                self.diagnostics.push_err(
                    idtok.loc,
                    &format!("expected identifier; got `{:?}`", idtok.kind),
                );
                return None;
            };
            decl.id = ast::Ident {
                name: sym,
                loc: idtok.loc,
            };
            self.lex.next(); // eat ident

            // Check for declared type
            if let Some(&colon) = self.lex.peek()
                && matches!(colon.kind, TokenType::Colon)
            {
                let default = Token::default();
                let tytok = self.lex.next().unwrap_or(&default); // eat ':'
                if !matches!(
                    tytok.kind,
                    TokenType::Ti8
                        | TokenType::Ti16
                        | TokenType::Ti32
                        | TokenType::Ti64
                        | TokenType::Tu8
                        | TokenType::Tu16
                        | TokenType::Tu32
                        | TokenType::Tu64
                        | TokenType::Tf32
                        | TokenType::Tf64
                        | TokenType::Tusize
                        | TokenType::Tchar
                        | TokenType::Tbyte
                        | TokenType::Tstring
                        | TokenType::Tbool
                ) {
                    self.diagnostics.push_err(
                        tytok.loc,
                        &format!("expected valid type specifier; got {} instead", tytok.kind),
                    );
                } else {
                    decl.decl_type = Some(Type::from(tytok.kind));
                }
                self.lex.next(); // eat typename
            }
            if !self.validate_tok(TokenType::Op(Op::Asgn)) {
                self.diagnostics
                    .push_err(t.loc, "expected `=` after variable declaration");
            }
            self.lex.next(); // eat '='

            decl.value = Box::new(if let Some(expr) = self.parse_expr(0.) {
                self.check_semi(self.lex.peek_behind().unwrap().loc);
                expr
            } else {
                self.diagnostics
                    .push_err(idtok.loc, "expected variable declaration");
                return None;
            });
            Some(decl)
        } else {
            self.diagnostics.push_err(
                t.loc,
                &format!(
                    "expected identifier after variable declaration `{}`",
                    decl.kind
                ),
            );
            None
        }
    }

    fn parse_expr(&mut self, min_rbp: f64) -> Option<ast::Expr> {
        let mut lhs: Option<ast::Expr> = None;

        // parenthesized expressions
        if let Some(&lparen) = self.lex.peek()
            && matches!(lparen.kind, TokenType::Lparen)
        {
            self.lex.next(); // eat '('
            lhs = self.parse_expr(0.);
            if !self.validate_tok(TokenType::Rparen) {
                self.diagnostics.push_err(
                    self.lex.peek().unwrap_or(&Token::default()).loc,
                    "expected closing `)`",
                );
            } else {
                self.lex.next(); // eat ')'
            }
        }

        // early return on delimiters like ; ) }
        if let Some(&tok) = self.lex.peek() {
            match tok.kind {
                TokenType::Rparen => {
                    return lhs;
                }
                TokenType::Semi | TokenType::Rcurly => {
                    return lhs;
                }
                _ => { /* fall-through */ }
            }
        } else {
            let prev_token = self.lex.peek_behind().unwrap();
            self.diagnostics.push_err(
                prev_token.loc,
                &format!("expected expression after `{:?}`", prev_token),
            );
            return lhs;
        }

        // check operand types
        if let Some(operand) = self.lex.peek() {
            match operand.kind {
                TokenType::IntLit(val) => {
                    lhs = Some(ast::Expr::default());
                    lhs.as_mut().unwrap().atom = AtomKind::IntLit(ast::IntLit {
                        val,
                        loc: operand.loc,
                    });
                    lhs.as_mut().unwrap().loc = operand.loc;
                    self.lex.next(); // eat literal
                }
                TokenType::VarIdent(sym) => {
                    lhs = Some(ast::Expr::default());
                    lhs.as_mut().unwrap().atom = AtomKind::Ident(ast::Ident {
                        name: sym,
                        loc: operand.loc,
                    });
                    lhs.as_mut().unwrap().loc = operand.loc;
                    self.lex.next(); // eat ident
                }
                TokenType::Op(op) => match op {
                    Op::Add => {
                        self.lex.next();
                    }
                    Op::Sub => {
                        todo!("Unary negation")
                    }
                    Op::Nop => {
                        return None;
                    }
                    _ => {
                        // see what op failed
                        dbg!(&op);
                    }
                },
                _ => {
                    //TODO: Make this a sema analysis error
                    self.diagnostics.push_err(
                        operand.loc,
                        &format!("invalid operand of type `{:?}` in expression", operand.kind),
                    );
                    return lhs;
                }
            }
        } else {
            self.diagnostics.push_err(
                self.lex.peek_behind().unwrap().loc,
                "expected operands to expression",
            );
            return lhs;
        }

        // Binary expressions
        while let Some(&tok) = self.lex.peek()
            && tok.kind.is_op()
        {
            let op: Op = Op::from(tok);
            assert_ne!(op, Op::Nop);
            let (lbp, rbp) = self.get_infix_bp(op);
            if lbp < min_rbp {
                break;
            }
            self.lex.next(); // eat operator
            if let Some(rhs) = self.parse_expr(rbp) {
                let aggregate_node = ast::Expr {
                    atom: AtomKind::None,
                    loc: lhs.as_mut().unwrap().loc,
                    op,
                    lhs: Some(Box::new(lhs.unwrap())),
                    rhs: Some(Box::new(rhs)),
                    ty: Cell::new(None),
                };
                lhs = Some(aggregate_node);
            } else {
                self.diagnostics.push_err(
                    tok.loc,
                    &format!("expected rhs operand in binary expression after `{}`", op),
                );
                return lhs;
            }
        }
        lhs
    }

    fn check_semi(&mut self, loc: LocData) {
        match self
            .lex
            .peek()
            .unwrap_or(&Token {
                kind: TokenType::Null,
                loc,
            })
            .kind
        {
            TokenType::Semi => return,
            TokenType::Rparen => self.diagnostics.push_err(loc, "extraneous closing `)`"),
            TokenType::Rcurly => self.diagnostics.push_err(loc, "extraneous closing `}`"),
            _ => self.diagnostics.push_err(loc, "expected `;`"),
        }
    }

    fn parse_stmt_exit(&mut self) -> ast::StmtExit {
        let expr = self.parse_expr(0.);
        let loc = self.lex.peek_behind().unwrap().loc;
        self.check_semi(loc);
        ast::StmtExit {
            exit_code: expr,
            loc,
        }
    }

    fn parse_stmt(&mut self, outer_scp: &mut ast::Scope) -> Result<(), Box<dyn Error>> {
        let mut loc_scp = ast::Scope::default();
        outer_scp.vars.iter().for_each(|(&k, v)| {
            loc_scp.vars.insert(k, Rc::clone(v));
        });

        while let Some(&tok) = self.lex.peek() {
            if matches!(tok.kind, TokenType::Rcurly) {
                break; // end of scope
            }
            match tok.kind {
                TokenType::KwExit => {
                    self.lex.next(); // eat 'exit'
                    let enode = self.parse_stmt_exit();
                    self.program.stmts.push(ast::UnionNode::StmtExit(enode));
                }
                TokenType::KwLet | TokenType::KwMut => {
                    self.lex.next(); // eat 'let' | 'mut'
                    let decl: ast::VarDecl = if let Some(decl_res) = self.parse_var_decl(tok) {
                        decl_res
                    } else {
                        self.diagnostics
                            .push_err(tok.loc, "expected variable declaration");
                        ast::VarDecl::default()
                    };
                    let sym = decl.id.name;
                    let rc = Rc::new(decl);

                    loc_scp.vars.insert(sym, Rc::clone(&rc));
                    self.program.stmts.push(ast::UnionNode::VarDecl(rc));
                }
                TokenType::VarIdent(sym) => {
                    if !loc_scp.vars.contains_key(&sym) && !loc_scp.fns.contains_key(&sym) {
                        self.diagnostics.push_err(
                            tok.loc,
                            &format!("undeclared identifier `{}`", self.lex.sym.get(sym).unwrap()),
                        );
                    }

                    if loc_scp.vars.contains_key(&sym) {
                        if !matches!(loc_scp.vars.get(&sym).unwrap().kind, VarType::Let) {
                            self.diagnostics.push_err(
                                tok.loc,
                                &format!(
                                    "cannot mutate immutable variable {} declared with `let`",
                                    self.lex.sym.get(sym).unwrap()
                                ),
                            );
                            self.diagnostics.push_note(
                                tok.loc,
                                &format!(
                                    "did you mean to use `mut` when declaring `{}`?",
                                    self.lex.sym.get(sym).unwrap()
                                ),
                            );
                            break;
                        }
                        let decl = self.parse_var_decl(tok).unwrap();
                        let sym = decl.id.name;
                        let rc = Rc::new(decl);
                        loc_scp.vars.insert(sym, Rc::clone(&rc));
                        self.program.stmts.push(ast::UnionNode::VarDecl(rc));
                    }
                }
                TokenType::Semi => {
                    self.lex.next();
                    continue;
                }
                _ => {
                    // println!("Symbols: {:?}", self.lex.sym.symbols);
                    println!("Stmts: {:#?}", self.program.stmts);
                    eprintln!("Unhandled Type");
                    self.lex.next();
                }
            }
        }
        Ok(())
    }
    pub fn create_program(&mut self) -> Result<ast::Program, Box<dyn Error>> {
        self.parse_stmt(&mut ast::Scope::default())?;
        Ok(ast::Program {
            sym: self.lex.sym.clone(),
            stmts: self.program.stmts.clone(),
        })
    }
}
