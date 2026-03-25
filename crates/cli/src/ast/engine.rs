use anyhow::{bail, Context, Result};
use ast_grep_core::AstGrep;
use ast_grep_language::SupportLang;
use std::path::Path;

/// Detect language from file extension using ast-grep's SupportLang.
pub fn detect_lang(path: &Path) -> Result<SupportLang> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let lang_str = match ext {
        "rs" => "rust",
        "ts" => "typescript",
        "tsx" => "tsx",
        "js" | "mjs" | "cjs" => "javascript",
        "jsx" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "rb" => "ruby",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "cs" => "csharp",
        "swift" => "swift",
        "kt" | "kts" => "kotlin",
        "scala" | "sc" => "scala",
        "php" => "php",
        "lua" => "lua",
        "sh" | "bash" => "bash",
        "css" => "css",
        "html" | "htm" => "html",
        "json" => "json",
        "yaml" | "yml" => "yaml",
        "sol" => "solidity",
        "ex" | "exs" => "elixir",
        "hs" => "haskell",
        "nix" => "nix",
        "hcl" | "tf" => "hcl",
        _ => bail!("Unsupported file extension: .{ext}"),
    };

    lang_str
        .parse::<SupportLang>()
        .map_err(|e| anyhow::anyhow!("Failed to parse language '{lang_str}': {e}"))
}

/// Parse file contents into an AST.
pub fn parse_file(
    path: &Path,
) -> Result<(
    AstGrep<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    String,
)> {
    let lang = detect_lang(path)?;
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("Cannot read file: {}", path.display()))?;
    let grep = AstGrep::new(&source, lang);
    Ok((grep, source))
}

/// Node kinds that represent "definitions" across common tree-sitter grammars.
const DEFINITION_KINDS: &[&str] = &[
    // Functions
    "function_declaration",
    "function_definition",
    "function_item", // Rust
    "arrow_function",
    "generator_function_declaration",
    // Classes
    "class_declaration",
    "class_definition",
    "struct_item", // Rust
    "enum_item",   // Rust
    // Methods
    "method_definition",
    "method_declaration",
    "function_signature_item", // Rust trait methods
    // Interfaces / Types
    "interface_declaration",
    "type_alias_declaration",
    "type_item", // Rust
    // Traits / Impls
    "trait_item", // Rust
    "impl_item",  // Rust
    // Go
    "function_declaration",
    "method_declaration",
    "type_declaration",
    // Python
    "class_definition",
    "function_definition",
    // Export wrappers (we want the inner decls, but also catch top-level exports)
    "export_statement",
];

/// Check if a node kind is a definition.
pub fn is_definition_kind(kind: &str) -> bool {
    DEFINITION_KINDS.contains(&kind)
}

/// Extract the "name" from a definition node by looking at common field names.
/// Tree-sitter grammars typically use "name" or "declarator" fields.
pub fn extract_name(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Option<String> {
    // Try "name" field first (most common)
    if let Some(name_node) = node.field("name") {
        return Some(name_node.text().to_string());
    }
    // For export statements, look inside for the actual declaration
    if node.kind().as_ref() == "export_statement" {
        for child in node.children() {
            if is_definition_kind(&child.kind()) {
                return extract_name(&child);
            }
        }
    }
    None
}

/// Information about a definition found in the AST.
pub struct DefinitionInfo {
    pub name: Option<String>,
    #[allow(dead_code)]
    pub kind: String,
    pub start_line: usize, // 0-based
    pub end_line: usize,   // 0-based
    pub text: String,
}

/// Collect all top-level (and nested method) definitions from the AST.
pub fn collect_definitions(
    root: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Vec<DefinitionInfo> {
    let mut defs = Vec::new();
    collect_definitions_recursive(root, &mut defs, 0);
    defs
}

fn collect_definitions_recursive(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    defs: &mut Vec<DefinitionInfo>,
    depth: usize,
) {
    let kind = node.kind().to_string();
    if is_definition_kind(&kind) {
        let name = extract_name(node);
        let start_line = node.start_pos().line();
        let end_line = node.end_pos().line();
        defs.push(DefinitionInfo {
            name,
            kind: kind.clone(),
            start_line,
            end_line,
            text: node.text().to_string(),
        });
        // For class/struct/impl bodies, recurse to find methods
        if kind.contains("class")
            || kind.contains("struct")
            || kind.contains("impl")
            || kind.contains("trait")
            || kind.contains("interface")
        {
            for child in node.children() {
                collect_definitions_recursive(&child, defs, depth + 1);
            }
        }
        return;
    }
    // Recurse into children for top-level traversal
    if depth < 3 {
        for child in node.children() {
            collect_definitions_recursive(&child, defs, depth + 1);
        }
    }
}

/// Find all identifier nodes that match a given symbol name.
pub struct ReferenceInfo {
    pub line: usize, // 0-based
    #[allow(dead_code)]
    pub column: usize, // 0-based
    pub line_text: String,
}

pub fn find_references_in_node(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    symbol: &str,
    source: &str,
) -> Vec<ReferenceInfo> {
    let mut refs = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for n in node.dfs() {
        let kind = n.kind();
        if (kind.as_ref() == "identifier"
            || kind.as_ref() == "type_identifier"
            || kind.as_ref() == "property_identifier"
            || kind.as_ref() == "shorthand_property_identifier_pattern")
            && n.text().as_ref() == symbol
        {
            let line = n.start_pos().line();
            let col = n.start_pos().byte_point().1;
            let line_text = lines.get(line).unwrap_or(&"").to_string();
            refs.push(ReferenceInfo {
                line,
                column: col,
                line_text,
            });
        }
    }
    refs
}
