use std::error::Error;
use std::fs;

use crate::diagnostics::DiagHandler;
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::semantics::Sema;
pub struct CompileOptions {
    pub src_pth: String,
    pub dst_pth: String,
    pub dst_name: String,
    pub options: Vec<(String, String)>, // flag, option
}

pub struct Compiler {
    pub has_errors: bool,
    pub has_warns: bool,
    pub has_notes: bool,
}

impl Compiler {
    pub fn compile(opts: CompileOptions) -> Result<(), Box<dyn Error>> {
        let file = fs::read_to_string(opts.src_pth)?;
        let mut diagnostics = DiagHandler::new();

        // Tokenization
        println!("Tokenizing...");
        let mut lex = Lexer::new();
        lex.tokenize(&file)?;

        // Parsing
        println!("Parsing...");
        let mut parser = Parser::new(&mut lex, &mut diagnostics)?;
        let mut program = parser.create_program()?;
        // let mut symbol_table = std::mem::take(&mut program.sym);
        let mut symbol_table = program.sym.clone();

        // Semantic analysis and type inference + checks
        Sema::validate_program(&mut program, &mut diagnostics, &mut symbol_table)?;
        diagnostics.display_diagnostics();
        println!("FINAL STATEMENTS: {:#?}", program.stmts);
        Ok(())
    }
}
