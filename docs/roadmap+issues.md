- [ ] Parenthesized expressions with `sub` & `add` op `(1 + 2) - 4` fail
- [ ] No unary negation
- [ ] No working comments
- [x] No function call parsing -- **PRIORITY!!**

- [x] Refactor to function-like state where main is a function and not implicit. i.e. codegenerator structs for each individual function.
- [ ] Emit prologue and epilogue for every function
- [ ] Impl `return` and default-emit `ret` after every function; values are optionally provided
- [ ] Allow fwd decls and assign missing values at discovery

- [ ] Generate default behavior and expect main to be defined for program to start
