use std::{collections::HashMap, panic, rc::Rc};

use crate::{
    ast,
    irgenerator::{ArgType, Br, Call, Define, Expr, KlirNode, ProgScope, Store},
    lexer,
};

struct AsmMetadata {
    entry: Rc<str>,
    align: usize,
}

#[derive(Debug, Default)]
struct AsmScope {
    id: String,
    data: String,
    stackptr: usize,
    vars: HashMap<String, (ast::Type, usize /* register counter */)>,
    // fns_map: HashMap<
    //     String,
    //     (
    //         ast::Type,                        /* ret type */
    //         Option<Vec<(String, ast::Type)>>, /* args */
    //     ),
    // >,
}

pub struct CodeGenerator {
    scopes: Vec<ProgScope>,
    pub asm: String,
    stackptr: usize,
    vars: HashMap<String, (ast::Type, usize /* register counter */)>,
    fns_map: HashMap<
        String,
        (
            ast::Type,                        /* ret type */
            Option<Vec<(String, ast::Type)>>, /* args */
        ),
    >,
}

impl CodeGenerator {
    pub fn new(scopes: Vec<ProgScope>) -> Self {
        CodeGenerator {
            scopes,
            asm: String::new(),
            fns_map: HashMap::new(),
            stackptr: 0,
            vars: HashMap::new(),
        }
    }
    fn emit_typed_load(&mut self, ty: &ast::Type, reg_idx: usize, addr: usize, scp: &mut AsmScope) {
        match ty {
            ast::Type::I8 => {
                scp.data
                    .push_str(&format!("    ldrsb   x{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I16 => {
                scp.data
                    .push_str(&format!("    ldrsh   x{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I32 => {
                scp.data
                    .push_str(&format!("    ldrsw   x{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::U8 | ast::Type::Char | ast::Type::Bool => {
                scp.data
                    .push_str(&format!("    ldrb    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::U16 => {
                scp.data
                    .push_str(&format!("    ldrh    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::U32 => {
                scp.data
                    .push_str(&format!("    ldr     w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I64 | ast::Type::U64 => {
                scp.data
                    .push_str(&format!("    ldr     x{}, [sp, {}]\n", reg_idx, addr));
            }
            _ => todo!("This type is not implemented for codegen"),
        }
    }
    fn emit_typed_store(
        &mut self,
        ty: &ast::Type,
        reg_idx: usize,
        reassign_addr: Option<usize>,
        scp: &mut AsmScope,
    ) {
        let addr = if let Some(cached_adr) = reassign_addr {
            cached_adr
        } else {
            scp.stackptr
        };
        match ty {
            ast::Type::I8 | ast::Type::U8 | ast::Type::Char | ast::Type::Bool => {
                scp.data
                    .push_str(&format!("    strb    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I16 | ast::Type::U16 => {
                scp.data
                    .push_str(&format!("    strh    w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I32 | ast::Type::U32 => {
                scp.data
                    .push_str(&format!("    str     w{}, [sp, {}]\n", reg_idx, addr));
            }
            ast::Type::I64 | ast::Type::U64 => {
                scp.data
                    .push_str(&format!("    str     x{}, [sp, {}]\n", reg_idx, addr));
            }
            _ => todo!("This type is not implemented for codegen"),
        }
    }
    fn emit_operation(&mut self, op: &lexer::Op, ty: &ast::Type, scp: &mut AsmScope) {
        match op {
            lexer::Op::Add => scp.data.push_str("    add     x8, x9, x10\n"),
            lexer::Op::Sub => scp.data.push_str("    sub     x8, x9, x10\n"),
            lexer::Op::Mul => scp.data.push_str("    mul     x8, x9, x10\n"),
            lexer::Op::Lsl => scp.data.push_str("    lsl     x8, x9, x10\n"),
            lexer::Op::Lsr => scp.data.push_str("    lsr     x8, x9, x10\n"),
            lexer::Op::Asr => scp.data.push_str("    asr     x8, x9, x10\n"),
            lexer::Op::BwAnd => scp.data.push_str("    and     x8, x9, x10\n"),
            lexer::Op::BwOr => scp.data.push_str("    orr     x8, x9, x10\n"),
            lexer::Op::BwXor => scp.data.push_str("    eor     x8, x9, x10\n"),
            lexer::Op::BwNot => scp.data.push_str("    mvn     x8, x9\n"),
            lexer::Op::Div => {
                if ty.is_signed() {
                    scp.data.push_str("    sdiv    x8, x9, x10\n");
                } else {
                    scp.data.push_str("    udiv    x8, x9, x10\n");
                }
            }
            lexer::Op::Mod => {
                self.emit_operation(&lexer::Op::Div, ty, scp);
                scp.data.push_str("    mul     x10, x8, x10\n");
                scp.data.push_str("    sub     x8, x9, x10\n");
            }
            lexer::Op::LgAnd => {
                scp.data.push_str("    cmp     x9, 0\n");
                scp.data.push_str("    cset    x9, ne\n");
                scp.data.push_str("    cmp     x10, 0\n");
                scp.data.push_str("    cset    x10, ne\n");
                scp.data.push_str("    and     x8, x9, x10\n");
                scp.data.push_str("    cmp     x8, 0\n");
                scp.data.push_str("    cset    x8, ne\n");
            }
            lexer::Op::LgOr => {
                scp.data.push_str("    cmp     x9, 0\n");
                scp.data.push_str("    cset    x9, ne\n");
                scp.data.push_str("    cmp     x10, 0\n");
                scp.data.push_str("    cset    x10, ne\n");
                scp.data.push_str("    orr     x8, x9, x10\n");
                scp.data.push_str("    cmp     x8, 0\n");
                scp.data.push_str("    cset    x8, ne\n");
            }
            lexer::Op::Lt => {
                scp.data.push_str("    cmp     x9, x10\n");
                scp.data.push_str("    cset    x8, lt\n");
            }
            lexer::Op::Gt => {
                scp.data.push_str("    cmp     x9, x10\n");
                scp.data.push_str("    cset    x8, gt\n");
            }
            lexer::Op::Lte => {
                scp.data.push_str("    cmp     x9, x10\n");
                scp.data.push_str("    cset    x8, le\n");
            }
            lexer::Op::Gte => {
                scp.data.push_str("    cmp     x9, x10\n");
                scp.data.push_str("    cset    x8, ge\n");
            }
            lexer::Op::Eq => {
                scp.data.push_str("    cmp     x9, x10\n");
                scp.data.push_str("    cset    x8, eq\n");
            }
            lexer::Op::Neq => {
                scp.data.push_str("    cmp     x9, x10\n");
                scp.data.push_str("    cset    x8, ne\n");
            }
            lexer::Op::Pwr => {
                scp.data.push_str("    cbnz    x10, BASE_CASE_1\n"); // deg == 0
                scp.data.push_str("    mov     x8, 1\n");
                scp.data.push_str("    b       PWR_LOOP_END\n");

                scp.data.push_str("BASE_CASE_1:\n"); // deg == 1
                scp.data.push_str("    mov     x8, x9\n"); // move lhs into accum
                scp.data.push_str("    cmp     x10, 1\n");
                scp.data.push_str("    bne     PWR_LOOP_START\n");
                scp.data.push_str("    b       PWR_LOOP_END\n");

                scp.data.push_str("PWR_LOOP_START:\n");
                scp.data.push_str("    sub     x10, x10, 1\n");
                scp.data.push_str("    cbz    x10, PWR_LOOP_END\n");
                scp.data.push_str("    mul     x8, x8, x9\n"); // accum * lhs
                scp.data.push_str("    b       PWR_LOOP_START\n");
                scp.data.push_str("PWR_LOOP_END:\n");
            }
            _ => todo!("This operator is not implemented for codegen"),
        }
    }
    fn emit_typed_move(&mut self, ty: &ast::Type, reg_idx: usize, val: i128, scp: &mut AsmScope) {
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
                scp.data
                    .push_str(&format!("    mov     w{}, 0x{:X}\n", reg_idx, low));
                if low_med != 0 {
                    scp.data.push_str(&format!(
                        "    movk    w{}, 0x{:X}, lsl 16\n",
                        reg_idx, low_med
                    ));
                }
            }
            ast::Type::I32 | ast::Type::I64 | ast::Type::U64 | ast::Type::Usize => {
                scp.data
                    .push_str(&format!("    mov     w{}, 0x{:X}\n", reg_idx, low));
                if low_med != 0 {
                    scp.data.push_str(&format!(
                        "    movk    w{}, 0x{:X}, lsl 16\n",
                        reg_idx, low_med
                    ));
                }
                if high_med != 0 {
                    scp.data.push_str(&format!(
                        "    movk    w{}, 0x{:X}, lsl 32\n",
                        reg_idx, high_med
                    ));
                }
                if high != 0 {
                    scp.data
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
        self.asm.push_str(&format!("    sub     sp, sp, {}\n", amt));
    }
    fn emit_metadata(&mut self, md: AsmMetadata) {
        self.asm
            .insert_str(0, &format!(".global {}\n.align {}\n", md.entry, md.align));
    }
    fn visit_store(&mut self, store: &Store, scp: &mut AsmScope) {
        match &store.src {
            ArgType::Imm(val) => {
                self.emit_typed_move(&store.ty, 8, *val, scp);
                self.emit_typed_store(&store.ty, 8, None, scp);
                scp.vars
                    .insert(store.dest.clone(), (store.ty, scp.stackptr));
            }
            ArgType::Sym(name) | ArgType::Temp(name) => {
                let &(src_ty, src_addr) = scp.vars.get(name).unwrap();
                if let Some(&(dst_ty, dst_addr)) = scp.vars.get(&store.dest) {
                    self.emit_typed_load(&src_ty, 8, src_addr, scp);
                    self.emit_typed_store(&dst_ty, 8, Some(dst_addr), scp);
                    scp.vars.insert(store.dest.clone(), (dst_ty, dst_addr));
                    scp.stackptr += 8;
                } else {
                    scp.vars.insert(store.dest.clone(), (src_ty, src_addr));
                }
            }
        }
    }
    fn visit_expr(&mut self, expr: &Expr, scp: &mut AsmScope) {
        let mut reassign_addr = None;
        match &expr.lhs {
            ArgType::Sym(name) | ArgType::Temp(name) => {
                println!("Sym Name: {name}, Expr Dest: {}", expr.dest);
                let &(ty, sym_addr) = scp
                    .vars
                    .get(name)
                    .unwrap_or_else(|| panic!("Error loading address for variable {name}"));
                self.emit_typed_load(&ty, 9, sym_addr, scp);
            }
            ArgType::Imm(val) => {
                scp.data.push_str(&format!("    mov     x9, {}\n", val));
            }
        }
        match &expr.rhs {
            ArgType::Sym(name) | ArgType::Temp(name) => {
                println!("Sym Name: {name}, Expr Dest: {}", expr.dest);
                let &(ty, sym_addr) = scp
                    .vars
                    .get(name)
                    .unwrap_or_else(|| panic!("Error loading address for variable {name}"));
                self.emit_typed_load(&ty, 10, sym_addr, scp);
            }
            ArgType::Imm(val) => {
                scp.data.push_str(&format!("    mov     x10, {}\n", val));
            }
        }
        let mut ty_to_store = expr.ty;
        if let Some(&(ty, sym_addr)) = scp.vars.get(&expr.dest) {
            reassign_addr = Some(sym_addr);
            ty_to_store = ty;
        }
        self.emit_operation(&expr.op, &expr.ty, scp);
        self.emit_typed_store(&ty_to_store, 8, reassign_addr, scp);
        scp.vars.insert(
            expr.dest.clone(),
            (
                ty_to_store,
                if let Some(readdr) = reassign_addr {
                    readdr
                } else {
                    let ret = scp.stackptr;
                    scp.stackptr += 8;
                    ret
                },
            ),
        );
    }
    fn visit_define(&mut self, define: &Define, scp: &mut AsmScope) {
        //TODO: function arg issues with expression
        if define.name == "main" {
            scp.data.push_str("_main:\n");
            // let aligned_size = scp.stackptr.next_multiple_of(16);
            // self.emit_prologue(aligned_size + 16);
        } else {
            scp.data.push_str(&format!("{}:\n", define.name));
        }
        let mut args_vec = Vec::new();
        if let Some(args) = &define.args {
            for (argc, (arg_type, ty)) in args.iter().enumerate() {
                match arg_type {
                    ArgType::Imm(_val) => {
                        panic!("Cannot have imm in function def args")
                    }
                    ArgType::Temp(name) | ArgType::Sym(name) => {
                        scp.vars.insert(name.clone(), (*ty, 0)); // forward decl of these vars
                        args_vec.push((name.clone(), *ty));
                        // let &(var_ty, addr) = self.vars.get(name).unwrap();
                        // self.emit_typed_load(&var_ty, argc, addr);
                    }
                }
            }
            self.fns_map.insert(
                define.name.clone(),
                (
                    define.return_ty,
                    if !args_vec.is_empty() {
                        Some(args_vec)
                    } else {
                        None
                    },
                ),
            );
        }
    }
    fn visit_call(&mut self, call: &Call, scp: &mut AsmScope) {
        if let Some(args) = &call.args {
            for (argc, (arg_type, ty)) in args.iter().enumerate() {
                // TODO: Emit loads for argument variables passed in
                match arg_type {
                    ArgType::Imm(val) => {
                        self.emit_typed_move(ty, argc, *val, scp);
                        self.emit_typed_store(&ty, argc, None, scp);

                        // TODO: Lookup argnames from func decl and update address with these values;
                        if let Some(fn_def) = self.fns_map.get(&call.name) {
                            let argname = &fn_def.1.as_ref().unwrap()[argc].0;
                            let arg_ty = &fn_def.1.as_ref().unwrap()[argc].1;
                            scp.vars.insert(argname.clone(), (*arg_ty, scp.stackptr));
                            scp.stackptr += 8;
                        }
                    }
                    ArgType::Temp(name) | ArgType::Sym(name) => {
                        let &(var_ty, addr) = scp.vars.get(name).unwrap();
                        self.emit_typed_load(&var_ty, argc, addr, scp);
                    }
                }
            }
        }
        scp.data.push_str(&format!("    bl      {}\n", call.name));
    }
    fn visit_br(&mut self, br: &Br, scp: &mut AsmScope) {
        if let Some(flag) = &br.flag {
            let &(ty, addr) = scp
                .vars
                .get(flag)
                .unwrap_or_else(|| panic!("Could not get addr for {flag}"));
            self.emit_typed_load(&ty, 8, addr, scp);
            scp.data
                .push_str(&format!("    cbnz    w8, {}\n", br.label));
        } else {
            scp.data.push_str(&format!("    b       {}\n", br.label));
        }
    }

    pub fn generate(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let mut asm_scopes = Vec::<AsmScope>::new();
        let ir_scopes = std::mem::take(&mut self.scopes);
        for scope in &ir_scopes {
            let mut asm_scp = AsmScope::default();
            asm_scp.id = scope.id.clone();
            for node in &scope.ir.nodes {
                match node {
                    KlirNode::Alloca(_alloca) => {}
                    KlirNode::Store(store) => {
                        self.visit_store(store, &mut asm_scp);
                    }
                    KlirNode::Expr(expr) => {
                        self.visit_expr(expr, &mut asm_scp);
                    }
                    KlirNode::Define(define) => {
                        self.visit_define(define, &mut asm_scp);
                    }
                    KlirNode::Call(call) => {
                        self.visit_call(call, &mut asm_scp);
                    }
                    KlirNode::Br(br) => {
                        self.visit_br(br, &mut asm_scp);
                    }
                    KlirNode::Label(label) => {
                        asm_scp.data.push_str(&format!("{}:\n", label.name));
                    }
                    _ => todo!(),
                }
            }
            asm_scopes.push(asm_scp);
        }

        println!("ASM SCOPES: {:#?}", asm_scopes);

        // NOTE: This is globally effective
        // Arm64 16byte alignment requirement
        let aligned_size = self.stackptr.next_multiple_of(16);
        let md = AsmMetadata {
            entry: "_main".into(),
            align: 4,
        };
        for scp in &asm_scopes {
            self.asm.push_str(&scp.data);
        }
        self.emit_epilogue(aligned_size);
        self.emit_prologue(aligned_size);
        self.emit_metadata(md);
        println!("ASSEMBLY: \n{}", self.asm);
        Ok(())
    }
}
