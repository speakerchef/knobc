use klc::compiler::CompileOptions;
use klc::compiler::Compiler;
use std::process::exit;
use std::{env::args, error::Error};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();
    if args.len() != 3 {
        eprintln!("Error: Please supply arguments: path/to/klc <FILE.knv> <EXEC-NAME>");
        exit(1)
    }

    let path = &args[1];
    let exec_name = &args[2];

    Compiler::compile(CompileOptions {
        src_pth: path.clone(),
        dst_pth: String::from("./"),
        dst_name: exec_name.clone(),
        options: Vec::new(),
    })?;
    Ok(())
}
