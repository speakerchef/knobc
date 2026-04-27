# KLIR Specification

- **KLIR** is the Intermediate Representation (IR) language for *KNOB*. It takes inspiration from many other established IRs and 3-address-code (3AC) IR but is its own thing. This spec document is early stage and subject to changes in structure, syntax, calling conventions, etc...

> - *KLIR* intends to be a type of *MIR*.
> - `%var` denotes a *variable* stored on the stack.
> - `&%var` denotes the *address* at which %var is stored at (similar to C `&` address-of operator).
> - `*%var` denote the *value* at address stored in %var *IFF* %var stores a stack address; else garbage may be returned.
> - `i1` type denotes boolean flags. The result to `cmp` ops are always stored in `i1` implicitly.
> - All arithmetic ops will use `xN` 64-bit registers; Only store ops determine overflow semantics.

### Types supported in <T>:
| Type | Assembly (Aarch64)| Register |
|------|--------------------|----------|
|`i1` | STRB/LDRB | xN |
|`i8` | STRB/LDRSB | xN |
|`i16` | STRH/LDRSH | xN |
|`i32` | STR/LDRSW | xN |
|`i64` | STR/LDR | xN |
|`u8` | STRB/LDRB | xN |
|`u16` | STRH/LDRH | xN |
|`u32` | STR/LDR | xN |
|`u64` | STR/LDR | xN |
|`void` | N/A | N/A |
> [!NOTE] The `*` operator before a typename will denote a pointer to that type: Eg. "store *i32, &%foo, %bar"

### Opcodes

| Statments | usage |
|-|-|
| `alloca` \<T>, \<symbol>  | `alloca` i32, %foo |
| `store` \<T>, \<src>, \<dest> | `store` i32, 1337, %foo |
| `call` \<T>, \<function>(\[\<T> args...])  | `call` void print(i32 %foo, u8 %bar) |
| `define` \<T>, \<function>(\[\<T> args...])  | `define` i8 fizzbuzz(i32 %fizz, i32 %buzz)  |
| `label` \<name>:  | `label` mycondition: |
| `cmp` \<T>, \<cond>, \<lhs>, \<rhs>, \<dest>  | `cmp` i32, `lte`, %foo, %bar, %result  |
| `br` \<label>, \<optional-flag>  | `br` lb_body, %result *OR* `br` lb_else |
| Conditions \<cond> |
|-|
| `lt`
| `gt`|
| `lte` |
| `gte` |
| `eq` |
| `neq` |
| Arithmetic | usage |
|-|-|
| `add` \<T>, \<lhs>, \<rhs>, \<dest>  | `add` i32, 1, 2, %foo |
| `sub` \<T>, \<lhs>, \<rhs>, \<dest>  | `sub` i32, 1, 2, %foo |
| `mul` \<T>, \<lhs>, \<rhs>, \<dest>  | `mul` i32, 1, 2, %foo |
| `div` \<T>, \<lhs>, \<rhs>, \<dest>  | `div` i32, 4, 2, %foo |
| `mod` \<T>, \<lhs>, \<rhs>, \<dest>  | `mod` i32, 1, 2, %foo |
| `pwr` \<T>, \<lhs>, \<rhs>, \<dest>  | `pwr` i32, 2, 32, %foo |
| `or` \<T>, \<lhs>, \<rhs>, \<dest>  | `or` i1, 0, 1, %foo |
| `and` \<T>, \<lhs>, \<rhs>, \<dest>  | `and` i1, 0, 1, %foo |
| `xor` \<T>, \<lhs>, \<rhs>, \<dest>  | `xor` i1, 45, 45, %foo |
| `not` \<T>, \<operand>, \<dest>  | `not` i32, -100, %foo |
| `lsl` \<T>, \<operand>, \<shift-amt>, \<dest>  | `lsl` u8, 1, 8, %foo |
| `lsr` \<T>, \<operand>, \<shift-amt>, \<dest>  | `lsr` u8, 256, 8, %foo |
| `asl` \<T>, \<operand>, \<shift-amt>, \<dest>  | `lsl` i8, 1, 8, %foo |
| `asr` \<T>, \<operand>, \<shift-amt>, \<dest>  | `lsr` i8, 256, 8, %foo |

-----------------------------------------------

### Example IR:
- front end:
```knob
    let foo = 15;
    let bar = foo + 10;
    exit bar;
```
- IR (.klir)
```c
    alloca i32, %foo // allocate i32 on stack and store address in %foo
    store i32, 15, %foo // store 15 at stack address pointed to by %foo
    alloca i32, %bar
    add i32, %foo, 10, %bar // %bar = %foo + 10
    call void exit(i32 %bar)
```
-----------------------------------
