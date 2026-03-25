use anyhow::Result;
use std::fs;
use std::path::Path;

const SKILL_VERSION: &str = env!("CARGO_PKG_VERSION");

const SKILL_TEMPLATE: &str = r#"---
description: How to use the Navi CLI for code navigation
navi-version: $$VERSION$$
---

# Navi — Headless Code Navigation CLI

Navi is a Rust-based CLI tool built on `ast-grep` that provides AI-optimized code structure navigation. All output is clean plain text with absolute line numbers — no ANSI colors, no noise.

## Commands

### 1. `navi list <FILE>` — Extract File Skeleton

Get a quick overview of all definitions (functions, classes, interfaces, structs) in a file with bodies collapsed.

```bash
navi list src/auth/user_service.ts
```

Output:
```
File: src/auth/user_service.ts
  12: export interface User { ... }
  25: export class UserService { ... }
  30:   public async login(req: LoginReq): Promise<Token> { ... }
  88:   private hashPassword(pwd: string): string { ... }
```

**When to use:** Start here to understand a file's structure before diving into specifics.

### 2. `navi jump <SYMBOL> [--path <DIR>]` — Jump to Definition

Find and display the complete source code of a function, class, or type definition with ±3 lines of context.

```bash
navi jump login --path src/
```

Output:
```
Found definition for 'login' in src/auth/user_service.ts:
  28:   // Authenticates a user and returns a JWT token
  29:   @TrackActivity()
  30:   public async login(req: LoginReq): Promise<Token> {
  31:       const user = await this.db.find(req.username);
  32:       if (!user) throw new Error("Not found");
  33:       return generateToken(user);
  34:   }
```

**When to use:** When you encounter an unfamiliar symbol and need its full implementation.

### 3. `navi refs <SYMBOL> [--path <DIR>]` — Find All References

Locate every file and line where a symbol is referenced. Essential for assessing blast radius before refactoring.

```bash
navi refs login --path src/
```

Output:
```
Found 3 references for 'login':
- src/api/routes.ts: 45 | const token = await userService.login(req.body);
- src/tests/auth.test.ts: 12 | const res = await service.login(mockUser);
- src/auth/user_service.ts: 30 | public async login(req: LoginReq): Promise<Token> {
```

**When to use:** Before modifying a function signature or renaming a symbol.

### 4. `navi read <FILE> <START-END>` — Read Line Range

Read a precise slice of a file by line numbers. No AST parsing — just raw lines with line numbers.

```bash
navi read src/main.rs 10-25
```

**When to use:** When you already know the exact lines you need to inspect or include in context.

## Exit Codes

| Code | Meaning |
|------|---------|
| `0`  | Success (even if no results found — check stdout) |
| `1`  | File path error or file does not exist |
| `2`  | Argument parsing failure (bad range format, etc.) |
| `3`  | AST engine crash or internal error |

## Supported Languages

Navi supports 26+ languages including: Rust, TypeScript, JavaScript, Python, Go, Java, C, C++, Ruby, Swift, Kotlin, Scala, PHP, Lua, Bash, CSS, HTML, Solidity, Elixir, Haskell, and more.

## Recommended Workflow

1. **Explore** → `navi list <file>` to see what's in a file
2. **Dive** → `navi jump <symbol>` to read a specific definition
3. **Assess** → `navi refs <symbol>` to check who depends on it
4. **Slice** → `navi read <file> <range>` to grab exact lines for diff/patch
"#;

fn skill_content() -> String {
    SKILL_TEMPLATE.replace("$$VERSION$$", SKILL_VERSION)
}

/// Extract the navi-version value from SKILL.md frontmatter.
fn extract_version(content: &str) -> Option<&str> {
    for line in content.lines() {
        if let Some(version) = line.strip_prefix("navi-version:") {
            return Some(version.trim());
        }
        // Stop searching after frontmatter ends
        if line == "---" && content.starts_with("---") && !line.is_empty() {
            // We might be at the closing ---
            let mut seen_open = false;
            for l in content.lines() {
                if l == "---" && !seen_open {
                    seen_open = true;
                    continue;
                }
                if l == "---" && seen_open {
                    break;
                }
            }
        }
    }
    None
}

pub fn run(path: Option<&Path>) -> Result<()> {
    let base = path.unwrap_or_else(|| Path::new("."));
    let agent_dir = base.join(".agent");
    let skill_dir = agent_dir.join("skills").join("navi");
    let skill_file = skill_dir.join("SKILL.md");
    let content = skill_content();

    if skill_file.exists() {
        let existing = fs::read_to_string(&skill_file)?;
        let existing_ver = extract_version(&existing);

        if existing_ver == Some(SKILL_VERSION) {
            println!(
                "Skill file is up to date (v{}): {}",
                SKILL_VERSION,
                skill_file.display()
            );
            return Ok(());
        }

        // Version mismatch or missing — update
        fs::write(&skill_file, &content)?;
        match existing_ver {
            Some(v) => println!(
                "Updated Navi skill document ({v} → {SKILL_VERSION}): {}",
                skill_file.display()
            ),
            None => println!(
                "Updated Navi skill document (→ v{SKILL_VERSION}): {}",
                skill_file.display()
            ),
        }
        return Ok(());
    }

    fs::create_dir_all(&skill_dir)?;
    fs::write(&skill_file, &content)?;

    println!(
        "Created Navi skill document (v{SKILL_VERSION}) at: {}",
        skill_file.display()
    );
    Ok(())
}
