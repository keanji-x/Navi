---
description: How to use the Navi CLI for code navigation
navi-version: 0.5.1
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
| `navi read <FILE> <RANGE\|SYMBOL>` | Read line range (`10:20`) or symbol body |
| `navi tree [DIR] [--depth <N>] [-n <N>]` | Recursive directory skeleton (`-n` = min files) |
| `navi outline [DIR]` | Project architecture overview |
| `navi callers <SYMBOL> [--path <DIR>]` | Find call-sites (excludes imports) |
| `navi deps <FILE>` | Show file import/reverse-import graph |
| `navi types <SYMBOL> [--path <DIR>] [--depth <N>]` | Recursively expand type definitions |
| `navi scope <FILE> <LINE\|SYMBOL>` | Show enclosing scope at a line, or children of a symbol |
| `navi diff [SYMBOL] [--path <DIR>] [--since <N>]` | Git diff filtered to a symbol, or commit summary |
| `navi impls <TRAIT> [--path <DIR>]` | Find all implementations of a trait/interface |
| `navi grep <PATTERN> [--path <DIR>]` | AST-aware regex search (shows enclosing function) |
| `navi exports <FILE\|DIR>` | List public API surface |
| `navi flow <SYMBOL> [--path <DIR>] [--depth <N>]` | Recursive caller chain graph |
| `navi sg [ARGS...]` | Passthrough to ast-grep CLI |
| `navi init [DIR]` | Write/update this skill document |

## Recommended Workflow

1. **Orient** → `navi outline` or `navi tree` to map the project
2. **Explore** → `navi list <file>` to see a file's structure
3. **Dive** → `navi jump <symbol>` to read a definition
4. **Assess** → `navi refs <symbol>` or `navi callers <symbol>` to gauge blast radius
5. **Trace types** → `navi types <symbol> --depth 2` to understand data shapes
6. **Slice** → `navi read <file> <range>` to grab exact lines, or `navi read <file> <symbol>` to read a symbol's body
7. **Grep** → `navi grep 'pattern|or_pattern'` to find matches with enclosing function context (supports regex)
8. **APIs** → `navi exports <dir>` to see public API surface
9. **Diff** → `navi diff <symbol>` to see recent changes to a symbol, or `navi diff --since 5` for a commit summary
10. **Flow** → `navi flow <symbol> --depth 3` to trace the caller chain

## Exit Codes

| Code | Meaning |
|------|---------|
| `0`  | Success (even if no results found — check stdout) |
| `1`  | File path error or file does not exist |
| `2`  | Argument parsing failure (bad range format, etc.) |
| `3`  | AST engine crash or internal error |

## Supported Languages

Navi supports 26+ languages including: Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, Ruby, Swift, Kotlin, Scala, PHP, Lua, Bash, CSS, HTML, Solidity, Elixir, Haskell, and more.
