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
    // Const / variable declarations (TS: `export const X = {...}`, Rust: `const`/`static`)
    "lexical_declaration",    // TS/JS: const/let/var
    "variable_declaration",   // JS: var
    "const_item",             // Rust: const X: T = ...;
    "static_item",            // Rust: static X: T = ...;
    // Rust module declarations: `mod name;` and `pub mod name { ... }`
    // Also `use_declaration` for re-export skeletons in mod.rs-style files
    "mod_item",
    "use_declaration",
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
    // For lexical/variable declarations (TS/JS const/let/var), extract from variable_declarator
    let k = node.kind();
    if k.as_ref() == "lexical_declaration" || k.as_ref() == "variable_declaration" {
        for child in node.children() {
            if child.kind().as_ref() == "variable_declarator" {
                if let Some(name_node) = child.field("name") {
                    return Some(name_node.text().to_string());
                }
            }
        }
    }
    None
}

/// Node kinds that represent field declarations inside struct/class/interface types.
const FIELD_KINDS: &[&str] = &[
    "field_declaration",          // Rust, Go, C, C++
    "field_definition",           // C++
    "public_field_definition",    // JS/TS class fields
    "property_declaration",       // TS class properties
    "property_signature",         // TS interface properties
    "enum_variant",               // Rust enum variants
    "shorthand_property_identifier_pattern", // JS destructuring
    "pair",                       // JS/TS object literal: { key: value }
    "method_definition",          // JS/TS object literal: { method() {} }
    "shorthand_property_identifier", // JS/TS: { foo } (shorthand)
];

/// Check if a node kind is a field declaration.
fn is_field_kind(kind: &str) -> bool {
    FIELD_KINDS.contains(&kind)
}

/// Information about a definition found in the AST.
pub struct DefinitionInfo {
    pub name: Option<String>,
    #[allow(dead_code)]
    pub kind: String,
    pub start_line: usize, // 0-based
    pub end_line: usize,   // 0-based
    pub text: String,
    pub depth: usize,    // nesting depth: 0 = top-level, 1 = class member, etc.
    #[allow(dead_code)]
    pub is_field: bool,  // true if this is a struct/class field rather than a method/type
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
            depth,
            is_field: false,
        });

        // Determine which node to recurse into for member extraction
        let is_container = |k: &str| {
            k.contains("class")
                || k.contains("struct")
                || k.contains("impl")
                || k.contains("trait")
                || k.contains("interface")
        };

        if is_container(&kind) {
            // Direct class/struct/impl/trait/interface — recurse into body
            collect_members_recursive(node, defs, depth + 1);
        } else if kind == "export_statement" {
            // Export wrapper — look for the inner declaration and recurse
            for child in node.children() {
                let child_kind = child.kind().to_string();
                if is_container(&child_kind) {
                    collect_members_recursive(&child, defs, depth + 1);
                } else if child_kind == "lexical_declaration" || child_kind == "variable_declaration" {
                    // export const X = { ... } — extract object properties
                    collect_const_object_members(&child, defs, depth + 1);
                }
            }
        } else if kind == "lexical_declaration" || kind == "variable_declaration" {
            // const X = { ... } — extract object properties
            collect_const_object_members(node, defs, depth + 1);
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

/// Recursively scan a container node's children for field declarations and nested definitions.
/// Handles wrapper nodes like `field_declaration_list`, `class_body`, `declaration_list`, etc.
fn collect_members_recursive(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    defs: &mut Vec<DefinitionInfo>,
    depth: usize,
) {
    for child in node.children() {
        let child_kind = child.kind().to_string();
        if is_field_kind(&child_kind) {
            collect_field(&child, defs, depth);
        } else if is_definition_kind(&child_kind) {
            collect_definitions_recursive(&child, defs, depth);
        } else {
            // Recurse into body/list wrapper nodes
            collect_members_recursive(&child, defs, depth);
        }
    }
}

/// Collect a field declaration as a DefinitionInfo with is_field = true.
fn collect_field(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    defs: &mut Vec<DefinitionInfo>,
    depth: usize,
) {
    let name = extract_name(node);
    let start_line = node.start_pos().line();
    let end_line = node.end_pos().line();
    // Use the first line only for clean display
    let full_text = node.text().to_string();
    let first_line = full_text.lines().next().unwrap_or("").to_string();
    defs.push(DefinitionInfo {
        name,
        kind: node.kind().to_string(),
        start_line,
        end_line,
        text: first_line,
        depth,
        is_field: true,
    });
}

/// Recurse into a const/let/var declaration to find object literal values and extract their
/// properties (pairs) as field-depth members. Handles: `const X: T = { key: val, ... }`
fn collect_const_object_members(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    defs: &mut Vec<DefinitionInfo>,
    depth: usize,
) {
    for child in node.children() {
        let ck = child.kind().to_string();
        if ck == "variable_declarator" {
            // Look at the "value" field for object/array literals
            if let Some(value_node) = child.field("value") {
                let vk = value_node.kind().to_string();
                if vk == "object" || vk == "object_expression" {
                    collect_members_recursive(&value_node, defs, depth);
                }
                // Array literals: expand each element as a child
                if vk == "array" || vk == "array_expression" {
                    collect_array_element_members(&value_node, defs, depth);
                }
                // Also handle: { ... } as const / { ... } satisfies T
                if vk == "as_expression" || vk == "satisfies_expression" {
                    for inner in value_node.children() {
                        let ik = inner.kind().to_string();
                        if ik == "object" || ik == "object_expression" {
                            collect_members_recursive(&inner, defs, depth);
                        }
                        if ik == "array" || ik == "array_expression" {
                            collect_array_element_members(&inner, defs, depth);
                        }
                    }
                }
            }
        }
    }
}

/// Expand array literal elements as child definitions.
/// Object elements get their keys extracted; scalar elements are shown as-is.
fn collect_array_element_members(
    array_node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    defs: &mut Vec<DefinitionInfo>,
    depth: usize,
) {
    for child in array_node.children() {
        let ck = child.kind().to_string();
        if ck == "object" || ck == "object_expression" {
            // Object element: extract as a named entry with its key fields
            // Use the first "pair" key as the display name
            let first_key = child.children().find(|c| c.kind().as_ref() == "pair")
                .and_then(|pair| pair.field("key"))
                .map(|k| k.text().to_string());
            let start_line = child.start_pos().line();
            let end_line = child.end_pos().line();
            let full_text = child.text().to_string();
            let first_line = full_text.lines().next().unwrap_or("").to_string();
            defs.push(DefinitionInfo {
                name: first_key,
                kind: ck,
                start_line,
                end_line,
                text: first_line,
                depth,
                is_field: true,
            });
        } else if ck == "identifier" || ck == "string" || ck == "string_fragment"
            || ck == "number" || ck == "call_expression" || ck == "member_expression"
            || ck == "new_expression"
        {
            // Scalar / reference element
            let start_line = child.start_pos().line();
            let text = child.text().to_string();
            defs.push(DefinitionInfo {
                name: Some(text.clone()),
                kind: ck,
                start_line,
                end_line: start_line,
                text,
                depth,
                is_field: true,
            });
        }
        // Skip punctuation nodes (commas, brackets, etc.)
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

/// Find all identifier nodes whose text matches a regex pattern.
pub fn find_references_by_pattern(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    pattern: &regex::Regex,
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
            && pattern.is_match(n.text().as_ref())
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

/// Node kinds that represent import/use declarations across languages.
const IMPORT_KINDS: &[&str] = &[
    "import_statement",      // TS/JS
    "import_declaration",    // Go
    "use_declaration",       // Rust
    "mod_item",              // Rust: `mod name;` (internal module declaration)
    "import_from_statement", // Python (from x import y)
    "import_specification",  // TS/JS (named imports)
    "export_statement",      // TS/JS re-exports
];

/// Information about an import found in the AST.
pub struct ImportInfo {
    pub line: usize,       // 0-based
    pub source: String,    // The module specifier / path (e.g. "./backend", "@pkg/shared")
    pub line_text: String, // Full line text for display
}

/// Extract all import/use statements from the AST root.
pub fn extract_imports(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    source_text: &str,
) -> Vec<ImportInfo> {
    let mut imports = Vec::new();
    let lines: Vec<&str> = source_text.lines().collect();

    for child in node.children() {
        let kind = child.kind().to_string();

        if !IMPORT_KINDS.contains(&kind.as_str()) {
            continue;
        }

        let line = child.start_pos().line();
        let line_text = lines.get(line).unwrap_or(&"").to_string();

        // Try to find the module source/specifier by looking for string nodes
        let mut module_source = None;

        // DFS to find the string/source field
        for n in child.dfs() {
            let nk = n.kind().to_string();
            // TS/JS: "string" or "string_fragment", source field
            // Rust: "scoped_identifier", "identifier" in use paths
            // Python: "dotted_name"
            // Go: "interpreted_string_literal"
            if nk == "string_fragment" || nk == "interpreted_string_literal" {
                module_source = Some(n.text().to_string());
                break;
            }
            // Rust use paths
            if kind == "use_declaration"
                && (nk == "scoped_identifier" || nk == "use_as_clause" || nk == "scoped_use_list")
            {
                module_source = Some(n.text().to_string());
                break;
            }
            // Rust mod declarations: extract the module name identifier
            // Only pick the first direct identifier child (the module name itself)
            if kind == "mod_item" && nk == "identifier" {
                module_source = Some(n.text().to_string());
                break;
            }
            // Python dotted name
            if nk == "dotted_name" {
                module_source = Some(n.text().to_string());
                break;
            }
        }

        if let Some(src) = module_source {
            imports.push(ImportInfo {
                line,
                source: src,
                line_text,
            });
        }
    }
    imports
}

/// Node kinds that represent function/method call expressions across languages.
const CALL_EXPRESSION_KINDS: &[&str] = &[
    "call_expression",       // TS/JS/Rust/Go/C/C++
    "method_invocation",     // Java
    "call",                  // Python
    "send",                  // Ruby (method call)
    "function_call_expr",    // Kotlin
    "invocation_expression", // C#
    "new_expression",              // JS/TS: new Foo(...)
    "object_creation_expression",  // Java: new Foo(...)
];

/// Find all call-site references to a symbol (filtering out imports, type annotations, etc.).
/// Only returns references where the symbol appears as the callee of a function/method call.
pub fn find_callers_in_node(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    symbol: &str,
    source: &str,
) -> Vec<ReferenceInfo> {
    let mut callers = Vec::new();
    let lines: Vec<&str> = source.lines().collect();

    for n in node.dfs() {
        let kind = n.kind();
        if (kind.as_ref() == "identifier"
            || kind.as_ref() == "property_identifier"
            || kind.as_ref() == "field_identifier")
            && n.text().as_ref() == symbol
        {
            // Walk up ancestors to check if this identifier is in a call expression
            let mut current = n.clone();
            let mut is_call = false;
            // Check up to 3 levels of parents (handles member_expression → call_expression chains)
            for _ in 0..4 {
                if let Some(p) = current.parent() {
                    let pk = p.kind().to_string();
                    if CALL_EXPRESSION_KINDS.contains(&pk.as_str()) {
                        is_call = true;
                        break;
                    }
                    // Stop climbing if we hit a statement or declaration boundary
                    if pk.contains("statement")
                        || pk.contains("declaration")
                        || pk.contains("definition")
                    {
                        break;
                    }
                    current = p;
                } else {
                    break;
                }
            }

            if is_call {
                let line = n.start_pos().line();
                let col = n.start_pos().byte_point().1;
                let line_text = lines.get(line).unwrap_or(&"").to_string();
                callers.push(ReferenceInfo {
                    line,
                    column: col,
                    line_text,
                });
            }
        }
    }
    callers
}

/// Callee information: function name + location.
pub struct CalleeInfo {
    pub name: String,
    pub line: usize,
    #[allow(dead_code)]
    pub line_text: String,
}

/// Find all functions/methods called within a specific line range (e.g. a function body).
/// Returns named callee info, deduplicated by name.
pub fn find_callees_named_in_range(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
    start_line: usize,
    end_line: usize,
    source: &str,
) -> Vec<CalleeInfo> {
    let mut callees = Vec::new();
    let lines: Vec<&str> = source.lines().collect();
    let mut seen = std::collections::HashSet::new();

    for n in node.dfs() {
        let kind = n.kind();
        if !CALL_EXPRESSION_KINDS.contains(&kind.as_ref()) {
            continue;
        }
        let call_line = n.start_pos().line();
        if call_line < start_line || call_line > end_line {
            continue;
        }

        // Extract the callee name from the call expression
        let callee_name = extract_callee_name(&n);
        if let Some(name) = callee_name {
            if seen.insert(name.clone()) {
                let line_text = lines.get(call_line).unwrap_or(&"").to_string();
                callees.push(CalleeInfo {
                    name,
                    line: call_line,
                    line_text,
                });
            }
        }
    }
    callees
}

/// Extract the callee function name from a call expression node.
fn extract_callee_name(
    node: &ast_grep_core::Node<ast_grep_core::tree_sitter::StrDoc<SupportLang>>,
) -> Option<String> {
    // The "function" field of a call_expression is the callee
    if let Some(func) = node.field("function") {
        let fk = func.kind().to_string();
        if fk == "identifier" {
            return Some(func.text().to_string());
        }
        // obj.method() — member_expression with property "property"
        if fk == "member_expression" {
            if let Some(prop) = func.field("property") {
                return Some(prop.text().to_string());
            }
        }
        // Fallback: just use the full text of the callee
        return Some(func.text().to_string());
    }
    // new Foo() — constructor field
    if let Some(ctor) = node.field("constructor") {
        return Some(ctor.text().to_string());
    }
    None
}
