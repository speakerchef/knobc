use crate::traits::Iter;
use std::{collections::HashMap, error::Error, fmt::Display, panic, rc::Rc};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Op {
    #[default]
    Nop,
    Add,
    Sub,
    Mul,
    Div,
    Pwr,
    Mod,
    Lsl,
    Lsr,
    Asr,
    BwNot, // '~'
    BwOr,
    BwAnd,
    BwXor,
    LgNot, // '!'
    LgOr,
    LgAnd,
    Asgn,
    AddAsgn,
    SubAsgn,
    MulAsgn,
    DivAsgn,
    PwrAsgn,
    ModAsgn,
    AndAsgn,
    OrAsgn,
    XorAsgn,
    LslAsgn,
    LsrAsgn,
    Eq,
    Neq,
    Lt,
    Gt,
    Lte,
    Gte,
    ThinArrow,
}

impl From<Token> for Op {
    fn from(value: Token) -> Self {
        match value.kind {
            TokenType::Op(op) => op,
            _ => Op::Nop,
        }
    }
}

impl Display for Op {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Op::Nop => write!(f, "NOP"),
            Op::Add => write!(f, "+"),
            Op::Sub => write!(f, "-"),
            Op::Mul => write!(f, "*"),
            Op::Div => write!(f, "/"),
            Op::Mod => write!(f, "%"),
            Op::Pwr => write!(f, "**"),
            Op::Lsl => write!(f, "<<"),
            Op::Lsr | Op::Asr => write!(f, ">>"),
            Op::BwNot => write!(f, "~"),
            Op::BwOr => write!(f, "|"),
            Op::BwAnd => write!(f, "&"),
            Op::BwXor => write!(f, "^"),
            Op::LgNot => write!(f, "!"),
            Op::LgOr => write!(f, "||"),
            Op::LgAnd => write!(f, "&&"),
            Op::Asgn => write!(f, "="),
            Op::AddAsgn => write!(f, "+="),
            Op::SubAsgn => write!(f, "-="),
            Op::MulAsgn => write!(f, "*="),
            Op::DivAsgn => write!(f, "/="),
            Op::PwrAsgn => write!(f, "**="),
            Op::ModAsgn => write!(f, "%="),
            Op::AndAsgn => write!(f, "&="),
            Op::OrAsgn => write!(f, "|="),
            Op::XorAsgn => write!(f, "^="),
            Op::LslAsgn => write!(f, "<<="),
            Op::LsrAsgn => write!(f, ">>-"),
            Op::Eq => write!(f, "=="),
            Op::Neq => write!(f, "!="),
            Op::Lt => write!(f, "<"),
            Op::Gt => write!(f, ">"),
            Op::Lte => write!(f, "<="),
            Op::Gte => write!(f, ">="),
            Op::ThinArrow => write!(f, "->"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Symbol(pub u32);

impl Display for Symbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum TokenType {
    Ti8,
    Ti16,
    Ti32,
    Ti64,
    Tu8,
    Tu16,
    Tu32,
    Tu64,
    Tf32,
    Tf64,
    Tusize,
    Tchar,
    Tstring,
    Tbool,
    Tvoid,
    KwReturn,
    KwFn,
    KwLet,
    KwMut,
    KwIf,
    KwElif,
    KwElse,
    KwWhile,
    KwExit,
    Op(Op),
    Semi,
    Colon,
    Lparen,
    Rparen,
    Lcurly,
    Rcurly,
    Lsquare,
    Rsquare,
    Comma,
    IntLit(i128),
    CharLit(i128),
    ByteLit(i128),
    BoolLit(bool),
    FloatLit(f64),
    VarIdent(Symbol),

    #[default]
    Null,

    WhiteSpace,
    NewLine,
}

impl TokenType {
    pub fn is_op(&self) -> bool {
        matches!(*self, TokenType::Op(_))
    }
    pub fn is_type_token(&self) -> bool {
        return matches!(
            self,
            TokenType::Ti8
                | TokenType::Ti16
                | TokenType::Ti32
                | TokenType::Ti64
                | TokenType::Tu8
                | TokenType::Tchar
                | TokenType::Tbool
                | TokenType::Tu16
                | TokenType::Tu32
                | TokenType::Tu64
                | TokenType::Tf32
                | TokenType::Tf64
                | TokenType::Tusize
                | TokenType::Tstring
                | TokenType::Tvoid,
        );
    }

    pub fn char_to_token(ch: char) -> TokenType {
        match ch {
            ' ' => TokenType::WhiteSpace,
            '\n' => TokenType::NewLine,
            ';' => TokenType::Semi,
            ',' => TokenType::Comma,
            ':' => TokenType::Colon,
            '(' => TokenType::Lparen,
            ')' => TokenType::Rparen,
            '{' => TokenType::Lcurly,
            '}' => TokenType::Rcurly,
            '[' => TokenType::Lsquare,
            ']' => TokenType::Rsquare,
            _ => panic!("Unknown char found: {ch}"),
        }
    }
}

impl Display for TokenType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenType::KwReturn => write!(f, "KwReturn"),
            TokenType::KwFn => write!(f, "KwFn"),
            TokenType::KwLet => write!(f, "KwLet"),
            TokenType::KwMut => write!(f, "KwMut"),
            TokenType::KwIf => write!(f, "KwIf"),
            TokenType::KwElif => write!(f, "KwElif"),
            TokenType::KwElse => write!(f, "KwElse"),
            TokenType::KwWhile => write!(f, "KwWhile"),
            TokenType::KwExit => write!(f, "KwExit"),
            TokenType::Op(op) => write!(f, "{}", op),
            TokenType::Semi => write!(f, "Semi"),
            TokenType::Colon => write!(f, "Color"),
            TokenType::Lparen => write!(f, "Lparen"),
            TokenType::Rparen => write!(f, "Rparen"),
            TokenType::Lcurly => write!(f, "Lcurly"),
            TokenType::Rcurly => write!(f, "Rcurly"),
            TokenType::Lsquare => write!(f, "Lsquare"),
            TokenType::Rsquare => write!(f, "Rsquare"),
            TokenType::Comma => write!(f, "Comma"),
            TokenType::IntLit(_) => write!(f, "IntLit"),
            TokenType::CharLit(_) => write!(f, "CharLit"),
            TokenType::ByteLit(_) => write!(f, "ByteLit"),
            TokenType::BoolLit(_) => write!(f, "BoolLit"),
            TokenType::FloatLit(_) => write!(f, "FloatLit"),
            TokenType::VarIdent(_) => write!(f, "VarIdent"),
            TokenType::Null => write!(f, "Null"),
            _ => write!(f, "Unknown Token"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub map: HashMap<Rc<str>, Symbol>, // for lookup eg. "foo": 24
    pub symbols: Vec<Rc<str>>,
}

impl SymbolTable {
    fn push(&mut self, id: &str) -> Symbol {
        let rc = Rc::from(id);
        self.symbols.push(Rc::clone(&rc));
        let sym = Symbol(self.symbols.len() as u32 - 1); // sym = index
        self.map.insert(rc, sym);
        sym
    }

    pub fn get(&self, sym: Symbol) -> Option<Rc<str>> {
        if let Some(v) = self.symbols.get(sym.0 as usize) {
            return Some(Rc::clone(v));
        }
        None
    }

    pub fn contains(&self, sym: Symbol) -> bool {
        self.symbols.get(sym.0 as usize).is_some()
    }
}

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq)]
pub struct LocData {
    pub line: usize,
    pub col: usize,
}

impl Display for LocData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.line, self.col)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Token {
    pub kind: TokenType,
    pub loc: LocData,
}

#[derive(Debug)]
pub struct Lexer {
    pub sym: SymbolTable,
    tokens: Vec<Token>,
    tok_ptr: usize,
    line_ct: usize,
    col_ct: usize,
}

impl Iter for Lexer {
    type Item = Token;

    fn peek(&self) -> Option<&Self::Item> {
        self.tokens.get(self.tok_ptr)
    }

    fn peek_behind(&self) -> Option<&Self::Item> {
        self.tokens.get(self.tok_ptr - 1)
    }

    fn peek_ahead(&self) -> Option<&Self::Item> {
        self.tokens.get(self.tok_ptr + 1)
    }

    fn next(&mut self) -> Option<&Self::Item> {
        self.tok_ptr += 1;
        self.tokens.get(self.tok_ptr)
    }
}

impl Default for Lexer {
    fn default() -> Self {
        Lexer {
            tokens: vec![],
            sym: SymbolTable {
                map: HashMap::new(),
                symbols: Vec::new(),
            },
            tok_ptr: 0,
            line_ct: 1,
            col_ct: 1,
        }
    }
}

impl Lexer {
    pub fn new() -> Lexer {
        Self::default()
    }
    fn parse_delim(&mut self, kind: TokenType, buf: &mut String) -> Result<(), Box<dyn Error>> {
        let loc = LocData {
            line: self.line_ct,
            col: self.col_ct,
        };
        if !buf.is_empty() {
            let cls_tok = self.classify_token(buf, loc)?;
            self.tokens.push(cls_tok);
            buf.clear();
        }
        if kind == TokenType::NewLine {
            self.line_ct += 1;
            self.col_ct = 0;
        }
        if !matches!(kind, TokenType::WhiteSpace | TokenType::NewLine) {
            self.tokens.push(Token { kind, loc });
        }
        Ok(())
    }

    pub fn tokenize(&mut self, file: &str) -> Result<(), Box<dyn Error>> {
        let mut file_it = file.chars().peekable();
        let mut buf = String::new();

        while let Some(ch) = file_it.next() {
            self.col_ct += 1;

            match ch {
                ';' | ':' | ' ' | '\n' | '(' | ')' | '{' | '}' | '[' | ']' | ',' => {
                    self.parse_delim(TokenType::char_to_token(ch), &mut buf)?
                }
                _ => {
                    // names and keywords
                    if ch.is_ascii_alphanumeric() || ch.eq(&'_') {
                        buf.push(ch);
                    } else {
                        // else flush buffer before operator lexing
                        if !buf.is_empty()
                            && buf.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                        {
                            let cls_tok = self.classify_token(
                                &buf,
                                LocData {
                                    line: self.line_ct,
                                    col: self.col_ct,
                                },
                            )?;
                            self.tokens.push(cls_tok);
                            buf.clear();
                        }

                        // Operators
                        buf.push(ch);
                        if let Some(&doub_op) = file_it.peek()
                            && "+-*/<>=|&^!%".contains(doub_op)
                        {
                            buf.push(doub_op);
                            file_it.next();
                            self.col_ct += 1;
                            if let Some(&trip_op) = file_it.peek()
                                && "+-*/<>=|&^!%".contains(trip_op)
                            {
                                buf.push(trip_op);
                                file_it.next();
                                self.col_ct += 1;
                            }
                        }

                        self.tokens.push(Token {
                            kind: TokenType::Op(self.classify_op(&buf)),
                            loc: LocData {
                                line: self.line_ct,
                                col: self.col_ct,
                            },
                        });
                        buf.clear();
                    }
                }
            }
        }
        // println!("Tokens: {:#?}", self.tokens);
        Ok(())
    }

    fn classify_op(&self, op: &str) -> Op {
        match op {
            "+" => Op::Add,
            "-" => Op::Sub,
            "*" => Op::Mul,
            "/" => Op::Div,
            "%" => Op::Mod,
            "**" => Op::Pwr,
            "&" => Op::BwAnd,
            "|" => Op::BwOr,
            "^" => Op::BwXor,
            "~" => Op::BwNot,
            "<<" => Op::Lsl,
            ">>" => Op::Lsr,
            "=" => Op::Asgn,
            "+=" => Op::AddAsgn,
            "-=" => Op::SubAsgn,
            "*=" => Op::MulAsgn,
            "/=" => Op::DivAsgn,
            "%=" => Op::ModAsgn,
            "**=" => Op::PwrAsgn,
            "&=" => Op::AndAsgn,
            "|=" => Op::OrAsgn,
            "^=" => Op::XorAsgn,
            "<<=" => Op::LslAsgn,
            ">>=" => Op::LsrAsgn,
            ">" => Op::Gt,
            "<" => Op::Lt,
            ">=" => Op::Gte,
            "<=" => Op::Lte,
            "==" => Op::Eq,
            "!=" => Op::Neq,
            "&&" => Op::LgAnd,
            "||" => Op::LgOr,
            "!" => Op::LgNot,
            "->" => Op::ThinArrow,
            _ => {
                println!("NOP Operator: {}", op);
                Op::Nop
            }
        }
    }

    fn classify_token(&mut self, tok: &str, loc: LocData) -> Result<Token, Box<dyn Error>> {
        match tok {
            "i8" => Ok(Token {
                kind: TokenType::Ti8,
                loc,
            }),
            "i16" => Ok(Token {
                kind: TokenType::Ti16,
                loc,
            }),
            "i32" => Ok(Token {
                kind: TokenType::Ti32,
                loc,
            }),
            "i64" => Ok(Token {
                kind: TokenType::Ti64,
                loc,
            }),
            "u8" => Ok(Token {
                kind: TokenType::Tu8,
                loc,
            }),
            "u16" => Ok(Token {
                kind: TokenType::Tu16,
                loc,
            }),
            "u32" => Ok(Token {
                kind: TokenType::Tu32,
                loc,
            }),
            "u64" => Ok(Token {
                kind: TokenType::Tu64,
                loc,
            }),
            "usize" => Ok(Token {
                kind: TokenType::Tusize,
                loc,
            }),
            "f32" => Ok(Token {
                kind: TokenType::Tf32,
                loc,
            }),
            "f64" => Ok(Token {
                kind: TokenType::Tf64,
                loc,
            }),
            "char" => Ok(Token {
                kind: TokenType::Tchar,
                loc,
            }),
            "bool" => Ok(Token {
                kind: TokenType::Tbool,
                loc,
            }),
            "string" => Ok(Token {
                kind: TokenType::Tstring,
                loc,
            }),
            "void" => Ok(Token {
                kind: TokenType::Tvoid,
                loc,
            }),
            "exit" => Ok(Token {
                kind: TokenType::KwExit,
                loc,
            }),
            "let" => Ok(Token {
                kind: TokenType::KwLet,
                loc,
            }),
            "mut" => Ok(Token {
                kind: TokenType::KwMut,
                loc,
            }),
            "if" => Ok(Token {
                kind: TokenType::KwIf,
                loc,
            }),
            "elif" => Ok(Token {
                kind: TokenType::KwElif,
                loc,
            }),
            "else" => Ok(Token {
                kind: TokenType::KwElse,
                loc,
            }),
            "while" => Ok(Token {
                kind: TokenType::KwWhile,
                loc,
            }),
            "fn" => Ok(Token {
                kind: TokenType::KwFn,
                loc,
            }),
            "return" => Ok(Token {
                kind: TokenType::KwReturn,
                loc,
            }),
            symbol => {
                if !symbol.is_empty() {
                    if symbol.chars().all(|c| c.is_ascii_digit()) {
                        return Ok(Token {
                            kind: TokenType::IntLit(symbol.parse::<i128>()?),
                            loc,
                        });
                    } else {
                        let sym_id = if let Some(existing_value) = self.sym.map.get(symbol) {
                            *existing_value
                        } else {
                            self.sym.push(symbol)
                        };

                        return Ok(Token {
                            kind: TokenType::VarIdent(sym_id),
                            loc,
                        });
                    }
                }
                Ok(Token {
                    kind: TokenType::Null,
                    loc: LocData { line: 0, col: 0 },
                })
            }
        }
    }
}
