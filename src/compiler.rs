use std::error::Error;
use std::fs;
use std::process::{Command, exit};

use crate::backend::CodeGenerator;
use crate::diagnostics::DiagHandler;
use crate::irgenerator::IrGenerator;
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
        let mut symbol_table = std::mem::take(&mut program.sym);
        let mut fn_table = std::mem::take(&mut program.fns);

        // Semantic analysis and type inference + checks
        println!("Semantic Analysis & Type Checking...");
        let mut sema = Sema::new(
            &mut program,
            &mut diagnostics,
            &mut symbol_table,
            &mut fn_table,
        );
        sema.validate_program()?;
        if diagnostics.has_errors() {
            diagnostics.display_diagnostics();
            exit(1);
        }

        // KLIR Generation
        let mut irgenerator = IrGenerator::new(&mut program, &mut diagnostics, &mut symbol_table);
        irgenerator.emit_klir()?;

        // Assembly CodeGen
        let mut backend = CodeGenerator::new(irgenerator.scopes);
        backend.generate()?;

        std::fs::write("/tmp/knobc_asm_out.s", &backend.asm).expect("Error during compilation!");
        std::fs::write(format!("./{}.s", opts.dst_name), &backend.asm)
            .expect("Error during compilation!");
        let _assembler_out = Command::new("clang")
            .args(vec![
                "-c",
                "-g",
                "-Wno-missing-sysroot",
                "-o",
                "/tmp/knobc_asm_out.o",
                "/tmp/knobc_asm_out.s",
            ])
            .output()?;

        let sdk_path_shower = Command::new("xcrun")
            .args(vec!["--sdk", "macosx", "--show-sdk-path"])
            .output()?;

        let _linker_out = Command::new("ld")
            .args(vec![
                "-lSystem",
                "-syslibroot",
                str::from_utf8(&sdk_path_shower.stdout)?.trim(),
                "-o",
                &format!("./{}", opts.dst_name),
                "/tmp/knobc_asm_out.o",
            ])
            .output()?;

        let _ = Command::new("rm")
            .args(vec!["/tmp/knobc_asm_out.s", "/tmp/knobc_asm_out.o"])
            .output()?;

        println!("Compilation Complete");
        Ok(())
    }
}
