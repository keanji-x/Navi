# 🧭 Navi

**Headless code navigation CLI for AI agents.**

Built on [ast-grep](https://ast-grep.github.io/) and tree-sitter. Zero ANSI noise, token-friendly output, absolute line numbers on everything.

## Quick Start

```bash
# One-liner: installs Rust (if needed) + Navi + ast-grep
curl -sSf https://raw.githubusercontent.com/keanji-x/Navi/main/install.sh | bash

# Or install manually:
# 0. Install Rust toolchain (skip if you already have cargo)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 1. Install Navi
cargo install --git https://github.com/keanji-x/Navi.git

# 2. Install ast-grep (needed for `navi sg`)
cargo install ast-grep

# Update to latest (re-run with --force)
# cargo install --git https://github.com/keanji-x/Navi.git --force
# cargo install ast-grep

# 3. Initialize skill document for your AI agent
cd your-project/
navi init .

# Done! Your AI agent can now read .agent/skills/navi/SKILL.md
```

After `navi init`, the following file is created:

```
your-project/
└── .agent/
    └── skills/
        └── navi/
            └── SKILL.md    ← AI reads this to learn Navi commands
```

Any AI agent that scans `.agent/skills/` will automatically discover how to use Navi for structural code navigation.

---

## Commands

| Command | Purpose |
|---------|---------|
| `navi list <FILE>` | Extract file skeleton (collapsed bodies) |
| `navi jump <SYMBOL>` | Jump to symbol definition |
| `navi refs <SYMBOL>` | Find all references to a symbol |
| `navi read <FILE> <RANGE>` | Read exact line range |
| `navi tree [DIR]` | Recursive directory skeleton |
| `navi outline [DIR]` | Project architecture overview |
| `navi callers <SYMBOL>` | Find call-sites (excludes imports) |
| `navi deps <FILE>` | Show file import/reverse-import graph |
| `navi types <SYMBOL>` | Recursively expand type definitions |
| `navi scope <FILE> <LINE>` | Show enclosing scope at a line |
| `navi diff <SYMBOL>` | Git diff filtered to a symbol |
| `navi sg [ARGS...]` | Passthrough to ast-grep CLI |
| `navi init [DIR]` | Write/update AI skill documents |

> 📖 Full command reference with examples and flags: [COMMANDS.md](crates/cli/templates/COMMANDS.md)


## Exit Codes

| Code | Meaning |
|------|---------|
| `0` | Success (no results is still `0` — check stdout) |
| `1` | File/path does not exist |
| `2` | Argument parsing error |
| `3` | AST engine internal error |

## Supported Languages

Rust · TypeScript · TSX · JavaScript · Python · Go · Java · C · C++ · C# · Ruby · Swift · Kotlin · Scala · PHP · Lua · Bash · CSS · HTML · JSON · YAML · Solidity · Elixir · Haskell · Nix · HCL

26+ languages via tree-sitter grammars.

## Design Principles

- **Zero Noise** — No ANSI colors, no banners. Output goes straight into prompts.
- **Token Friendly** — Only return what the LLM needs. Bodies collapsed, context minimal.
- **Line Number Aware** — Every line is numbered for precise downstream edits and diffs.

## License

MIT
