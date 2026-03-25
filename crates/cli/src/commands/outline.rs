use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::ast::engine::{detect_lang, extract_imports, parse_file};

pub fn run(path: Option<&Path>) -> Result<()> {
    let search_dir = path.unwrap_or_else(|| Path::new("."));

    if !search_dir.exists() {
        anyhow::bail!("Path does not exist: {}", search_dir.display());
    }

    // Collect all files and their imports
    let mut file_imports: HashMap<String, Vec<String>> = HashMap::new();
    let mut all_files: Vec<String> = Vec::new();

    let walker = ignore::WalkBuilder::new(search_dir)
        .hidden(true)
        .git_ignore(true)
        .sort_by_file_path(|a, b| a.cmp(b))
        .build();

    for entry in walker {
        let entry = entry?;
        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }
        if detect_lang(entry_path).is_err() {
            continue;
        }
        let file_str = entry_path.display().to_string();
        all_files.push(file_str.clone());

        if let Ok((grep, source)) = parse_file(entry_path) {
            let root = grep.root();
            let imports = extract_imports(&root, &source);
            let import_sources: Vec<String> = imports.into_iter().map(|i| i.source).collect();
            file_imports.insert(file_str, import_sources);
        }
    }

    // Group files by package/directory
    let mut packages: HashMap<String, Vec<String>> = HashMap::new();
    for file in &all_files {
        let pkg = extract_package_name(file, search_dir);
        packages.entry(pkg).or_default().push(file.clone());
    }

    // Build package-level dependency graph
    let mut pkg_deps: HashMap<String, HashSet<String>> = HashMap::new();
    let pkg_names: Vec<String> = packages.keys().cloned().collect();

    for (pkg_name, pkg_files) in &packages {
        let mut deps = HashSet::new();
        for file in pkg_files {
            if let Some(imports) = file_imports.get(file) {
                for imp in imports {
                    // Try to match import to a package
                    for other_pkg in &pkg_names {
                        if other_pkg != pkg_name && imp.contains(other_pkg.as_str()) {
                            deps.insert(other_pkg.clone());
                        }
                    }
                }
            }
        }
        pkg_deps.insert(pkg_name.clone(), deps);
    }

    // Output
    println!("Project outline: {}", search_dir.display());
    println!();

    let mut sorted_pkgs: Vec<&String> = packages.keys().collect();
    sorted_pkgs.sort();

    for pkg in &sorted_pkgs {
        let file_count = packages[*pkg].len();
        let deps = pkg_deps.get(*pkg).cloned().unwrap_or_default();
        let mut dep_list: Vec<&String> = deps.iter().collect();
        dep_list.sort();

        if dep_list.is_empty() {
            println!("{pkg} ({file_count} files)");
        } else {
            let dep_str: Vec<&str> = dep_list.iter().map(|s| s.as_str()).collect();
            println!("{} ({} files) → {}", pkg, file_count, dep_str.join(", "));
        }
    }

    Ok(())
}

fn extract_package_name(file: &str, base: &Path) -> String {
    let base_str = base.display().to_string();
    let relative = file.strip_prefix(&base_str).unwrap_or(file);
    let relative = relative.trim_start_matches('/');

    // Extract first meaningful directory component as "package"
    let parts: Vec<&str> = relative.split('/').collect();
    if parts.len() >= 2 {
        // For monorepo: crates/cli/src/... → "crates/cli"
        // For packages/foo/src/... → "packages/foo"
        if (parts[0] == "crates" || parts[0] == "packages" || parts[0] == "libs")
            && parts.len() >= 2
        {
            return format!("{}/{}", parts[0], parts[1]);
        }
        // For src/commands/... → "src"
        parts[0].to_string()
    } else {
        "(root)".to_string()
    }
}
