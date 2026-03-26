---
description: How to use the Navi CLI for code navigation
navi-version: $$VERSION$$
---

# Navi — Headless Code Navigation CLI

Navi is a Rust-based CLI tool built on `ast-grep` that provides AI-optimized code structure navigation. All output is clean plain text with absolute line numbers — no ANSI colors, no noise.

> See [COMMANDS.md](./COMMANDS.md) for the full reference of all supported commands.

## Quick Reference

| Command | Purpose |
|---------|---------|
| `navi list <FILE>` | Extract file skeleton (collapsed bodies, struct fields, `pub mod`/`use`) |
| `navi jump <SYMBOL> [--path <DIR>] [--all]` | Jump to symbol definition (fuzzy suggestions on no match) |
| `navi refs <SYMBOL> [--path <DIR>]` | Find all references to a symbol |
| `navi read <FILE> <RANGE\|SYMBOL> [--hints]` | Read line range, symbol body, or `Parent.child` dot-path; `--hints` for type annotations |
| `navi tree [DIR] [--depth <N>] [-n <N>]` | Recursive directory skeleton (compact mode for large dirs) |
| `navi outline [DIR]` | Project architecture overview |
| `navi callers <SYMBOL> [--path <DIR>]` | Find call-sites including `new Class(...)` instantiation |
| `navi deps <FILE>` | Show file import/reverse-import graph (handles path-based imports) |
| `navi types <SYMBOL> [--path <DIR>] [--depth <N>]` | Recursively expand type definitions |
| `navi scope <FILE> <LINE\|SYMBOL>` | Show enclosing scope at a line, or children of a symbol |
| `navi diff [SYMBOL] [--since <N>] [--changes]` | Git diff filtered to a symbol, commit summary, or symbol-level changelog |
| `navi impls <TRAIT> [--path <DIR>]` | Find implementations (Rust `impl`, TS `implements`, TS `const: Interface`) |
| `navi grep <PATTERN> [--path <DIR>]` | AST-aware search (shows enclosing function) |
| `navi exports <FILE\|DIR>` | List public API surface |
| `navi flow <SYMBOL> [--path <DIR>] [--depth <N>]` | Recursive caller chain graph |
| `navi search <PATTERN> [--path <DIR>] [--kind <K>]` | Global symbol search by regex + kind filter |
| `navi xref <SYMBOL> [--path <DIR>]` | Cross-reference graph (definition + callers + all refs) |
| `navi sg [ARGS...]` | Passthrough to ast-grep CLI |
| `navi init [DIR]` | Write/update this skill document |

## Recommended Workflow

1. **Orient** → `navi outline` or `navi tree` to map the project
2. **Explore** → `navi list <file>` to see a file's structure
3. **Dive** → `navi jump <symbol>` to read a definition
4. **Assess** → `navi refs <symbol>` or `navi callers <symbol>` to gauge blast radius
5. **Xref** → `navi xref <symbol>` for the full picture (definition + callers + refs in one shot)
6. **Trace types** → `navi types <symbol> --depth 2` to understand data shapes
7. **Slice** → `navi read <file> <range>` for exact lines, `navi read <file> <symbol>` for a body, or `navi read <file> Class.method` for a nested member; add `--hints` for type annotations
8. **Search** → `navi search 'Pattern' --kind function` to find symbols by name pattern and kind
9. **Grep** → `navi grep <symbol>` to find matches with enclosing function context
10. **APIs** → `navi exports <dir>` to see public API surface
11. **Diff** → `navi diff <symbol>` for recent changes; `navi diff --changes --since 5` for symbol-level changelog
12. **Flow** → `navi flow <symbol> --depth 3` to trace the caller chain
13. **Scope** → `navi scope <file> <line>` for enclosing scope; `navi scope <file> ClassName` for child symbols

## Exit Codes

| Code | Meaning |
|------|---------|
| `0`  | Success (even if no results found — check stdout) |
| `1`  | File path error or file does not exist |
| `2`  | Argument parsing failure (bad range format, etc.) |
| `3`  | AST engine crash or internal error |

## Supported Languages

Navi supports 26+ languages including: Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, Ruby, Swift, Kotlin, Scala, PHP, Lua, Bash, CSS, HTML, Solidity, Elixir, Haskell, and more.
