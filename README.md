# KLC: KNOB-Lang-Compiler
>(This is a Rust Re-write of the original; [Original C++ Version](https://github.com/speakerchef/klc-compiler))

A from scratch compiler for **KNOB** (**K**ompiled **NOB**) — a statically typed, semicolon delimited language that I'm creating that compiles down to AArch64 assembly: Uses `.knv` as the file extension.

> *Why `.knv` and not `.knb`?*
> Everyone knows the true perfect language prioritizes ergonomics over sensible standards. `v` is easier to hit from the home row than `b`. You're welcome.

---

## Architecture

```
source.knv → Lexer → Pratt Parser → AST → Codegen → AArch64 ASSEMBLY → clang/ld → executable
```

- **Lexer/Tokenizer** — tokenizes `.knv` source into a stream of typed tokens
- **Parser** — Pratt-Parsing with precedence climbing for binary expressions and Recursive-Descent parsing for the rest, producing an AST.
- **Codegen** — Currently direct AST emission to assembly targeting AArch64 (Apple Silicon / macOS Darwin ABI). (x86_64 support in the future). No LLVM IR or other backends/deps.

---

## Language features

>KNOB is a fun project of mine still under active construction.

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
# Run
cargo run <FILE.knv> <EXEC-NAME>

# Optional: Build and alias
cargo build
cd target/debug
alias klc="path/to/klc"
# Run
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
- [x] Functions
- [ ] Floating point support (Harder than you think)
- [ ] Standard library functions like print()
- [ ] Loop optimizations
- [ ] Register allocation pass
- [ ] x86_64 generation
- [ ] ...and many more!
