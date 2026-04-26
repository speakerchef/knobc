use std::collections::HashMap;

use crate::{
    ast,
    irgenerator::{ArgType, KlirBlob, KlirNode},
    lexer,
};
pub struct CodeGenerator {
    ir: KlirBlob,
    pub asm: String,
    stackptr: usize,
    stacksz: usize,
    vars: HashMap<String, (ast::Type, usize /* register counter */)>,
    allocated: HashMap<String /* varname */, bool /* is allocated */>,
}

impl CodeGenerator {
    pub fn new(ir: KlirBlob) -> Self {
        CodeGenerator {
            ir,
            asm: String::new(),
            stackptr: 0,
            stacksz: 0,
            vars: HashMap::new(),
            allocated: HashMap::new(),
        }
    }
    fn emit_typed_load(&mut self, ty: &ast::Type, reg_idx: usize, addr: usize) {
        match ty {
            ast::Type::I8 => {
                self.asm
                    .push_str(&format!("    ldrsb   x{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I16 => {
                self.asm
                    .push_str(&format!("    ldrsh   x{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I32 => {
                self.asm
                    .push_str(&format!("    ldrsw   x{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::U8 | ast::Type::Char | ast::Type::Bool => {
                self.asm
                    .push_str(&format!("    ldrb    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::U16 => {
                self.asm
                    .push_str(&format!("    ldrh    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::U32 => {
                self.asm
                    .push_str(&format!("    ldr     w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I64 | ast::Type::U64 => {
                self.asm
                    .push_str(&format!("    ldr     x{}, [sp, {}]\n", reg_idx, addr));
            }
            _ => todo!("This type is not implemented for codegen"),
        }
    }
    fn emit_operation(&mut self, op: &lexer::Op, ty: &ast::Type) {
        match op {
            lexer::Op::Add => self.asm.push_str("    add     x8, x9, x10\n"),
            lexer::Op::Sub => self.asm.push_str("    sub     x8, x9, x10\n"),
            lexer::Op::Mul => self.asm.push_str("    mul     x8, x9, x10\n"),
            lexer::Op::Lsl => self.asm.push_str("    lsl     x8, x9, x10\n"),
            lexer::Op::Lsr => self.asm.push_str("    lsr     x8, x9, x10\n"),
            lexer::Op::Asr => self.asm.push_str("    asr     x8, x9, x10\n"),
            lexer::Op::BwAnd => self.asm.push_str("    and     x8, x9, x10\n"),
            lexer::Op::BwOr => self.asm.push_str("    orr     x8, x9, x10\n"),
            lexer::Op::BwXor => self.asm.push_str("    eor     x8, x9, x10\n"),
            lexer::Op::BwNot => self.asm.push_str("    mvn     x8, x9\n"),
            lexer::Op::Div => {
                if ty.is_signed() {
                    self.asm.push_str("    sdiv    x8, x9, x10\n");
                } else {
                    self.asm.push_str("    udiv    x8, x9, x10\n");
                }
            }
            lexer::Op::Mod => {
                self.emit_operation(&lexer::Op::Div, ty);
                self.asm.push_str("    mul     x10, x8, x10\n");
                self.asm.push_str("    sub     x8, x9, x10\n");
            }
            lexer::Op::LgAnd => {
                self.asm.push_str("    cmp     x9, 0\n");
                self.asm.push_str("    cset    x9, ne\n");
                self.asm.push_str("    cmp     x10, 0\n");
                self.asm.push_str("    cset    x10, ne\n");
                self.asm.push_str("    and     x8, x9, x10\n");
                self.asm.push_str("    cmp     x8, 0\n");
                self.asm.push_str("    cset    x8, ne\n");
            }
            lexer::Op::LgOr => {
                self.asm.push_str("    cmp     x9, 0\n");
                self.asm.push_str("    cset    x9, ne\n");
                self.asm.push_str("    cmp     x10, 0\n");
                self.asm.push_str("    cset    x10, ne\n");
                self.asm.push_str("    orr     x8, x9, x10\n");
                self.asm.push_str("    cmp     x8, 0\n");
                self.asm.push_str("    cset    x8, ne\n");
            }
            lexer::Op::Lt => {
                self.asm.push_str("    cmp     x9, x10\n");
                self.asm.push_str("    cset    x8, lt\n");
            }
            lexer::Op::Gt => {
                self.asm.push_str("    cmp     x9, x10\n");
                self.asm.push_str("    cset    x8, gt\n");
            }
            lexer::Op::Lte => {
                self.asm.push_str("    cmp     x9, x10\n");
                self.asm.push_str("    cset    x8, le\n");
            }
            lexer::Op::Gte => {
                self.asm.push_str("    cmp     x9, x10\n");
                self.asm.push_str("    cset    x8, ge\n");
            }
            lexer::Op::Eq => {
                self.asm.push_str("    cmp     x9, x10\n");
                self.asm.push_str("    cset    x8, eq\n");
            }
            lexer::Op::Neq => {
                self.asm.push_str("    cmp     x9, x10\n");
                self.asm.push_str("    cset    x8, ne\n");
            }
            lexer::Op::Pwr => {
                self.asm.push_str("    cbnz    x10, BASE_CASE_1\n"); // deg == 0
                self.asm.push_str("    mov     x8, 1\n");
                self.asm.push_str("    b       PWR_LOOP_END\n");

                self.asm.push_str("BASE_CASE_1:\n"); // deg == 1
                self.asm.push_str("    mov     x8, x9\n"); // move lhs into accum
                self.asm.push_str("    cmp     x10, 1\n");
                self.asm.push_str("    bne     PWR_LOOP_START\n");
                self.asm.push_str("    b       PWR_LOOP_END\n");

                self.asm.push_str("PWR_LOOP_START:\n");
                self.asm.push_str("    sub     x10, x10, 1\n");
                self.asm.push_str("    cbz    x10, PWR_LOOP_END\n");
                self.asm.push_str("    mul     x8, x8, x9\n"); // accum * lhs
                self.asm.push_str("    b       PWR_LOOP_START\n");
                self.asm.push_str("PWR_LOOP_END:\n");
            }
            _ => todo!("This operator is not implemented for codegen"),
        }
    }
    fn emit_epilogue(&mut self) {
        self.asm
            .push_str(&format!("    add     sp, sp, {}", self.stacksz));
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.asm.push_str(".global _main\n.align 4\n_main:\n");
        let nodes = std::mem::take(&mut self.ir.nodes);
        for node in &nodes {
            match node {
                KlirNode::Alloca(alloca) => {
                    if self.stackptr.is_multiple_of(16) {
                        self.asm.push_str("    sub     sp, sp, 16\n");
                        self.stacksz += 16;
                    }
                    self.allocated.insert(alloca.dest.clone(), true);
                }
                KlirNode::Store(store) => match store.ty {
                    ast::Type::I32 => {
                        let reg_idx = 8;
                        self.asm.push_str(&format!(
                            "    str     w{}, [sp, {}]\n",
                            reg_idx, self.stackptr
                        ));
                        // Store (varname, address) tuple
                        self.vars
                            .insert(store.dest.clone(), (store.ty, self.stackptr));
                        self.stackptr += 8;
                    }
                    _ => todo!("IR Node with Type not implemented"),
                },
                KlirNode::Expr(expr) => {
                    match &expr.lhs {
                        ArgType::Sym(name) | ArgType::Temp(name) => {
                            let &(ty, sym_addr) = self.vars.get(name).unwrap_or_else(|| {
                                panic!("Error loading address for variable {name}")
                            });
                            self.emit_typed_load(&ty, 9, sym_addr);
                        }
                        ArgType::Imm(val) => {
                            self.asm.push_str(&format!("    mov     x9, {}\n", val));
                        }
                    }
                    match &expr.rhs {
                        ArgType::Sym(name) | ArgType::Temp(name) => {
                            let &(ty, sym_addr) = self.vars.get(name).unwrap_or_else(|| {
                                panic!("Error loading address for variable {name}")
                            });
                            self.emit_typed_load(&ty, 10, sym_addr);
                        }
                        ArgType::Imm(val) => {
                            self.asm.push_str(&format!("    mov     x10, {}\n", val));
                        }
                    }

                    self.emit_operation(&expr.op, &expr.ty);
                }
                KlirNode::Call(call) => {
                    for (argc, argv) in call.args.iter().enumerate() {
                        let &(ty, addr) = self.vars.get(&argv.1).unwrap();
                        self.emit_typed_load(&ty, argc, addr);
                    }
                    self.asm
                        .push_str(&format!("    bl      {}\n", call.methodname));
                }
            }
        }
        self.emit_epilogue();
        println!("ASSEMBLY: \n{}", self.asm);
        self.ir.nodes = nodes;
        Ok(())
    }
}
