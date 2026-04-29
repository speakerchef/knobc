use std::{collections::HashMap, rc::Rc};

use crate::{
    ast,
    irgenerator::{ArgType, KlirBlob, KlirNode},
    lexer,
};

struct AsmMetadata {
    entry: Rc<str>,
    align: usize,
}

struct FuncScopes {
    name: Rc<str>,
    data: String,
    stackptr: usize,
    vars: HashMap<String, (ast::Type, usize /* register counter */)>,
}

pub struct CodeGenerator {
    ir: KlirBlob,
    pub asm: String,
    fns: Vec<Box<FuncScopes>>, // non-inlined function bodies

    stackptr: usize,
    vars: HashMap<String, (ast::Type, usize /* register counter */)>,
}

impl CodeGenerator {
    pub fn new(ir: KlirBlob) -> Self {
        CodeGenerator {
            ir,
            asm: String::new(),
            fns: Vec::new(),
            stackptr: 0,
            vars: HashMap::new(),
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
    fn emit_typed_store(&mut self, ty: &ast::Type, reg_idx: usize, reassign_addr: Option<usize>) {
        let addr = if let Some(cached_adr) = reassign_addr {
            cached_adr
        } else {
            self.stackptr
        };
        match ty {
            ast::Type::I8 | ast::Type::U8 | ast::Type::Char | ast::Type::Bool => {
                self.asm
                    .push_str(&format!("    strb    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I16 | ast::Type::U16 => {
                self.asm
                    .push_str(&format!("    strh    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I32 | ast::Type::U32 => {
                self.asm
                    .push_str(&format!("    str     w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I64 | ast::Type::U64 => {
                self.asm
                    .push_str(&format!("    str     x{}, [sp, {}]\n", reg_idx, addr));
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
    fn emit_typed_move(&mut self, ty: &ast::Type, reg_idx: usize, val: i128) {
        let low = (val & 0xFFFF) as u16;
        let low_med = (val >> 16) as u16;
        let high_med = (val >> 32) as u16;
        let high = (val >> 48) as u16;
        match ty {
            ast::Type::I8
            | ast::Type::Bool
            | ast::Type::Char
            | ast::Type::I16
            | ast::Type::U8
            | ast::Type::U16
            | ast::Type::U32 => {
                self.asm
                    .push_str(&format!("    mov     w{}, 0x{:X}\n", reg_idx, low));
                if low_med != 0 {
                    self.asm.push_str(&format!(
                        "    movk    w{}, 0x{:X}, lsl 16\n",
                        reg_idx, low_med
                    ));
                }
            }
            ast::Type::I32 | ast::Type::I64 | ast::Type::U64 | ast::Type::Usize => {
                self.asm
                    .push_str(&format!("    mov     w{}, 0x{:X}\n", reg_idx, low));
                if low_med != 0 {
                    self.asm.push_str(&format!(
                        "    movk    w{}, 0x{:X}, lsl 16\n",
                        reg_idx, low_med
                    ));
                }
                if high_med != 0 {
                    self.asm.push_str(&format!(
                        "    movk    w{}, 0x{:X}, lsl 32\n",
                        reg_idx, high_med
                    ));
                }
                if high != 0 {
                    self.asm
                        .push_str(&format!("    movk    w{}, 0x{:X}, lsl 48\n", reg_idx, high));
                }
            }
            _ => todo!("Type not impl for `mov` yet"),
        }
    }
    fn emit_epilogue(&mut self, amt: usize) {
        self.asm.push_str(&format!("    add     sp, sp, {}\n", amt));
    }
    fn emit_prologue(&mut self, amt: usize) {
        self.asm
            .insert_str(0, &format!("    sub     sp, sp, {}\n", amt));
    }
    fn emit_metadata(&mut self, md: AsmMetadata) {
        self.asm.insert_str(
            0,
            &format!(".global {}\n.align {}\n{}:\n", md.entry, md.align, md.entry),
        );
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let nodes = std::mem::take(&mut self.ir.nodes);
        for node in &nodes {
            match node {
                KlirNode::Alloca(alloca) => {}
                KlirNode::Store(store) => match &store.src {
                    ArgType::Imm(val) => {
                        self.emit_typed_move(&store.ty, 8, *val);
                        self.emit_typed_store(&store.ty, 8, None);
                        self.vars
                            .insert(store.dest.clone(), (store.ty, self.stackptr));
                    }
                    ArgType::Sym(name) | ArgType::Temp(name) => {
                        let &(src_ty, src_addr) = self.vars.get(name).unwrap();
                        if let Some(&(dst_ty, dst_addr)) = self.vars.get(&store.dest) {
                            self.emit_typed_load(&src_ty, 8, src_addr);
                            self.emit_typed_store(&dst_ty, 8, Some(dst_addr));
                            self.vars.insert(store.dest.clone(), (dst_ty, dst_addr));
                            self.stackptr += 8;
                        } else {
                            self.vars.insert(store.dest.clone(), (src_ty, src_addr));
                        }
                    }
                },
                KlirNode::Expr(expr) => {
                    let mut reassign_addr = None;
                    match &expr.lhs {
                        ArgType::Sym(name) | ArgType::Temp(name) => {
                            println!("Sym Name: {name}, Expr Dest: {}", expr.dest);
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
                            println!("Sym Name: {name}, Expr Dest: {}", expr.dest);
                            let &(ty, sym_addr) = self.vars.get(name).unwrap_or_else(|| {
                                panic!("Error loading address for variable {name}")
                            });
                            self.emit_typed_load(&ty, 10, sym_addr);
                        }
                        ArgType::Imm(val) => {
                            self.asm.push_str(&format!("    mov     x10, {}\n", val));
                        }
                    }
                    let mut ty_to_store = expr.ty;
                    if let Some(&(ty, sym_addr)) = self.vars.get(&expr.dest) {
                        reassign_addr = Some(sym_addr);
                        ty_to_store = ty;
                    }
                    self.emit_operation(&expr.op, &expr.ty);
                    self.emit_typed_store(&ty_to_store, 8, reassign_addr);
                    self.vars.insert(
                        expr.dest.clone(),
                        (
                            ty_to_store,
                            if let Some(readdr) = reassign_addr {
                                readdr
                            } else {
                                let ret = self.stackptr;
                                self.stackptr += 8;
                                ret
                            },
                        ),
                    );
                }
                KlirNode::Define(define) => {
                    self.asm.push_str(&format!("{}:\n", define.name));
                    if let Some(args) = &define.args {
                        for (argc, (ty, arg_type)) in args.iter().enumerate() {
                            match arg_type {
                                ArgType::Imm(val) => self.emit_typed_move(ty, argc, *val),
                                ArgType::Temp(name) | ArgType::Sym(name) => {
                                    self.vars.insert(name.clone(), (*ty, 0)); // forward decl of these vars
                                    // let &(var_ty, addr) = self.vars.get(name).unwrap();
                                    // self.emit_typed_load(&var_ty, argc, addr);
                                }
                            }
                        }
                    }
                    // self.asm.push_str(&format!("    bl      {}\n", define.name));
                }
                KlirNode::Call(call) => {
                    if let Some(args) = &call.args {
                        for (argc, (ty, arg_type)) in args.iter().enumerate() {
                            // TODO: Emit loads for argument variables passed in
                            match arg_type {
                                ArgType::Imm(val) => self.emit_typed_move(ty, argc, *val),
                                ArgType::Temp(name) | ArgType::Sym(name) => {
                                    let &(var_ty, addr) = self.vars.get(name).unwrap();
                                    self.emit_typed_load(&var_ty, argc, addr);
                                }
                            }
                        }
                    }
                    self.asm.push_str(&format!("    bl      {}\n", call.name));
                }
                KlirNode::Br(br) => {
                    if let Some(flag) = &br.flag {
                        let &(ty, addr) = self
                            .vars
                            .get(flag)
                            .unwrap_or_else(|| panic!("Could not get addr for {flag}"));
                        self.emit_typed_load(&ty, 8, addr);
                        self.asm
                            .push_str(&format!("    cbnz    w8, {}\n", br.label));
                    } else {
                        self.asm.push_str(&format!("    b       {}\n", br.label));
                    }
                }
                KlirNode::Label(label) => {
                    self.asm.push_str(&format!("{}:\n", label.name));
                }
                _ => todo!(),
            }
        }

        // NOTE: This is globally effective
        // Arm64 16byte alignment requirement
        let aligned_size = self.stackptr.next_multiple_of(16);
        let md = AsmMetadata {
            entry: "_main".into(),
            align: 4,
        };
        self.emit_prologue(aligned_size);
        self.emit_metadata(md);
        self.emit_epilogue(aligned_size);
        println!("ASSEMBLY: \n{}", self.asm);
        self.ir.nodes = nodes;
        Ok(())
    }
}
