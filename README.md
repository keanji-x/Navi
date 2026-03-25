# 🧭 Navi

**Headless code navigation CLI for AI agents.**

Built on [ast-grep](https://ast-grep.github.io/) and tree-sitter. Zero ANSI noise, token-friendly output, absolute line numbers on everything.

## Quick Start

```bash
# 1. Install
cargo install navi-code

# 2. Initialize skill document for your AI agent
cd your-project/
navi init

# Done! Your AI agent can now read .agent/skills/navi/SKILL.md
# to learn how to use Navi for code navigation.
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

### `navi list <FILE>` — File Skeleton

Extract all definitions with bodies collapsed. Lets the AI understand a file's structure without consuming thousands of tokens.

```
$ navi list src/auth/user_service.ts

File: src/auth/user_service.ts
  12: export interface User { ... }
  25: export class UserService { ... }
  30:   public async login(req: LoginReq): Promise<Token> { ... }
  88:   private hashPassword(pwd: string): string { ... }
```

### `navi jump <SYMBOL> [--path <DIR>]` — Jump to Definition

Find a symbol's complete source with ±3 lines of context.

```
$ navi jump login --path src/

Found definition for 'login' in src/auth/user_service.ts:
  28:   // Authenticates a user and returns a JWT token
  29:   @TrackActivity()
  30:   public async login(req: LoginReq): Promise<Token> {
  31:       const user = await this.db.find(req.username);
  32:       if (!user) throw new Error("Not found");
  33:       return generateToken(user);
  34:   }
```

### `navi refs <SYMBOL> [--path <DIR>]` — Find References

Locate every usage of a symbol across the codebase. Assess blast radius before refactoring.

```
$ navi refs login --path src/

Found 3 references for 'login':
- src/api/routes.ts: 45 | const token = await userService.login(req.body);
- src/tests/auth.test.ts: 12 | const res = await service.login(mockUser);
- src/auth/user_service.ts: 30 | public async login(req: LoginReq): Promise<Token> {
```

### `navi read <FILE> <START-END>` — Read Line Range

Read exact lines. No AST — just raw text with line numbers.

```
$ navi read src/main.rs 10-25
```

### `navi init [--path <DIR>]` — Generate AI Skill Doc

Create `.agent/skills/navi/SKILL.md` so AI agents can discover and learn Navi automatically.

```
$ navi init
Created Navi skill document at: ./.agent/skills/navi/SKILL.md
```

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
