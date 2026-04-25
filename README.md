# Knobc: Knob-Compiler
>(This is a Rust re-design & re-write of my original; [Original C++ Version](https://github.com/speakerchef/klc-compiler))
>Re-write also includes and will include a great deal of architectural changes both internally and language-definition wise.

- Knobc is a compiler for the **KNOB** (**K**ompiled **NOB**) programming language —
`Knob` is a statically typed, AOT-compiled language I'm creating that emits a custom defined IR (Intermediate Representation) called `klir` - emitting assembly for a few backends: Namely AArch64 (Apple Silicon & Linux) & x86_64(eventually):
- As of now, only Apple Silicon Arm64 assembly is emitted. Linux Arm64 will be implemented as the compiler matures.
> [!NOTE] The original C++ version of the compiler emitted raw AArch64 assembly. The new re-write with an MIR level will allow for optimization passes, multiple backends, and other cool stuff!

Uses `.knv` as the file extension.
> *Why `.knv` and not `.knb`?*
> Everyone knows the true perfect language prioritizes ergonomics over sensible standards. `v` is easier to hit from the home row than `b`. You're welcome.
---
## Architecture

```
Source → Lexer → Parser → AST → Type-Checking / Semantic Analysis → Typed-AST → MIR → Optimization Pass(es) → Backend → Assembly Codegen → Link Runtime → Executable
```

- **Lexer/Tokenizer** — tokenizes `.knv` source into a stream of typed tokens
- **Parser** — Pratt-Parsing with precedence climbing for expressions and Recursive-Descent parsing for the rest, producing an untyped-AST.
- Type-Checking and Semantic Analysis that resolves types and mutates the untyped-AST into a typed-AST. Semantic errors are also evaluated here.
- Typed-AST is walked and Knob-MIR is emitted for each node/operation/etc...
- Optimization (Later scope): Will analyze the IR for patterns to exploit and optimize
- **Codegen** — Currently targeting only AArch64 (Apple Silicon / macOS Darwin ABI). (x86_64 support in the future). No LLVM or other backends/deps.
---
## Language features
>KNOB is a fun project of mine still under active construction.

### Types (so far)
>[!NOTE] Full type suite is not currently implemented.

| Class | Variants |
|---------|-------------|
| `Integers`   | `u8/i8`, `u16/i16`, `u32/i32`, `u64/i64`, `usize` (semantic alias to `u64/u32`)|
| `Characters`   | `char` (aliased to `u8`)|
| `Floating Point`  | `f32`, `f64`|
| `Strings`    | `string` - likely aliased to a `u8` array of valid UTF-8 (hello rust XD)|
| `Boolean`  | `bool` w/ opts `true` & `false`|

### Keywords (so far)

| Keyword | What it does |
|---------|-------------|
| `let`   | Const-defaulted variable declaration |
| `mut`   | Mutable variable declaration |
| `exit`  | Exit with an exit code |
| `if`    | Conditional branch |
| `elif`  | Alternate branch |
| `else`  | Fallback branch |
| `while` | Loop while condition true |
| `fn` | Function declaration |

### Operators (so far)

| Category | Operators | Notes |
|----------|-----------|-------|
| Arithmetic | `+` `-` `*` `/` `%`| Standard integer arithmetic (fp later)|
| Power | `**` | Right associative, eg. 2**3 = 8 |
| Comparison | `==` `!=` `<` `>` `<=` `>=` | 1 or 0 |
| Logical | `&&` `\|\|` | Boolean logic on truthy/falsy values |
| Bitwise | `&` `\|` `^` | AND, OR, XOR |
| Bit Shift | `<<` `>>` | LSL, LSR |
| Unary | `-` | negation |
| Operate-Assign | `+=` `-=` `*=` `/=` `%=` `**=` `&=` `\|=` `<<=` `>>=` | Combine operation and assignment |


>[!WARNING] The below is stale from the original C++ codebase - will update as things move

### Other Features
- Parenthesized expressions with correct grouping: `(a + b) * (c - d)`
- Scoping with nested blocks and proper variable resolution
- Variables from outer scopes are visible in inner scopes
- Local variables are inaccessible outside their scope
- Nested if/elif/else with arbitrary depth
- While loops with mutable state
- Function declarations and calling with optional arguments

### Performance

- KLC currently emits raw, AArch64 assembly code with zero optimization passes or heuristic-based register allocation methods. Even so, KLC is competitive with C using Clang `-O0` while being multiple orders of magnitudes faster than Python (low hanging fruit but I don't care lol). 
Run the benchmark below as a test!

### Example syntax and benchmark for `KLC`. 
> Benchmark: 50,000 iterations of popcount, Collatz sequence, GCD, Fibonacci, primality testing, and hash accumulation combined.
#### Results: (M4 Pro MacBook Pro)
|Lang / Compiler | Time | Magnitude |
|----------|------|-----------|
| KNOB (KLC) | 1.63s | - |
| C (Clang) | 0.64s | ~2.5x Faster |
| Python3 | 52.70s | ~32.3x Slower! |
> This benchmark code is a showcase of most currently available features in `KLC`.

#### Try to run this! `echo $?` after should give you 153 :D
```
mut hash = 1;
mut outer = 0;

while (outer < 50000) {
    mut x = outer;
    mut bits = 0;

    while (x > 0) {
        if (x & 1) {
            bits = bits + 1;
        }
        x = x / 2;
    }

    if (bits > 8) {
        hash = (hash * 31 + outer) & 16777215;
    } elif (bits > 4) {
        hash = (hash ^ outer) & 16777215;
    } else {
        hash = (hash + bits * 7) & 16777215;
    }

    mut collatz = outer + 1;
    mut steps = 0;
    while (collatz != 1) {
        mut r = collatz & 1;
        if (r == 1) {
            collatz = collatz * 3 + 1;
        } else {
            collatz = collatz / 2;
        }
        steps = steps + 1;
    }

    hash = (hash + steps) & 16777215;

    mut ga = outer + 17;
    mut gb = outer + 31;
    while (gb != 0) {
        mut temp = gb;
        gb = ga - (ga / gb) * gb;
        ga = temp;
    }

    hash = (hash ^ ga) & 16777215;

    mut fa = 1;
    mut fb = 1;
    mut fi = 0;
    while (fi < 20) {
        mut temp = fa + fb;
        fa = fb;
        fb = temp;
        fi = fi + 1;
    }

    hash = (hash + fb) & 16777215;

    mut pc = outer + 2;
    mut ip = 1;
    mut dv = 2;
    while (dv < pc / 2 + 1) {
        if (pc - (pc / dv) * dv == 0) {
            ip = 0;
        }
        dv = dv + 1;
    }

    if (ip) {
        hash = (hash * 37 + pc) & 16777215;
    } else {
        hash = (hash + pc) & 16777215;
    }

    outer = outer + 1;
}

exit hash & 255;
```

---
## Build & Generate Executable

> **Requires:** Cargo, Clang/GCC for linker, AArch64 target (Apple Silicon Mac)

```bash
# Build the compiler
cmake -S . -B build/
cd build
cmake --build .

# Optional: Alias to use `klc` anywhere on your system
alias klc='path/to/klc/build/klc'

# Generate assembly and executable (MacOS Only for now)
./klc <FILE.knv> <EXEC-NAME> 

# Or if aliased:
klc <FILE.knv> <EXEC-NAME>
```

### Execute

```bash
./executable

# Or benchmark
time ./executable

# Check exit code
echo $?
```

---

## Roadmap

- [ ] String literals
- [ ] Functions
- [ ] Floating point support (Harder than you think)
- [ ] Standard library functions like print()
- [ ] Loop optimizations
- [ ] Register allocation pass
- [ ] x86_64 generation
- [ ] ...and many more!
