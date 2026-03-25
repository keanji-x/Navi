use anyhow::Result;
use std::fs;
use std::path::Path;

const SKILL_VERSION: &str = env!("CARGO_PKG_VERSION");

const SKILL_TEMPLATE: &str = include_str!("../../templates/SKILL.md");

const COMMANDS_TEMPLATE: &str = include_str!("../../templates/COMMANDS.md");

fn skill_content() -> String {
    SKILL_TEMPLATE.replace("$$VERSION$$", SKILL_VERSION)
}

fn commands_content() -> String {
    COMMANDS_TEMPLATE.replace("$$VERSION$$", SKILL_VERSION)
}

/// Extract the navi-version value from SKILL.md frontmatter.
fn extract_version(content: &str) -> Option<&str> {
    let mut in_frontmatter = false;
    for line in content.lines() {
        if line == "---" {
            if !in_frontmatter {
                in_frontmatter = true;
                continue;
            } else {
                break;
            }
        }
        if in_frontmatter {
            if let Some(version) = line.strip_prefix("navi-version:") {
                return Some(version.trim());
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
    let commands_file = skill_dir.join("COMMANDS.md");
    let skill = skill_content();
    let commands = commands_content();

    if skill_file.exists() {
        let existing = fs::read_to_string(&skill_file)?;
        let existing_ver = extract_version(&existing);

        if existing_ver == Some(SKILL_VERSION) {
            // Ensure COMMANDS.md also exists even if SKILL.md is current
            if !commands_file.exists() {
                fs::write(&commands_file, &commands)?;
                println!(
                    "Created COMMANDS.md (v{SKILL_VERSION}): {}",
                    commands_file.display()
                );
            }
            println!(
                "Skill file is up to date (v{}): {}",
                SKILL_VERSION,
                skill_file.display()
            );
            return Ok(());
        }

        // Version mismatch or missing — update both files
        fs::write(&skill_file, &skill)?;
        fs::write(&commands_file, &commands)?;
        match existing_ver {
            Some(v) => println!(
                "Updated Navi skill documents ({v} → {SKILL_VERSION}): {}",
                skill_dir.display()
            ),
            None => println!(
                "Updated Navi skill documents (→ v{SKILL_VERSION}): {}",
                skill_dir.display()
            ),
        }
        return Ok(());
    }

    fs::create_dir_all(&skill_dir)?;
    fs::write(&skill_file, &skill)?;
    fs::write(&commands_file, &commands)?;

    println!(
        "Created Navi skill documents (v{SKILL_VERSION}) at: {}",
        skill_dir.display()
    );
    Ok(())
}
