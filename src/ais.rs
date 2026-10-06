//! AIS/0.1 — the Aura Intelligence Schema.
//!
//! AIS is a **stable, external semantic protocol** between the Aura compiler
//! and intelligent or developer tools (LSP, Playground, IDEs, future AI
//! tooling). It is deliberately not a serialization of internal Rust
//! structures: the compiler's `ast`/`check` types may change freely, while
//! this schema only changes when the *semantic interface* changes, and then
//! only with a version bump.
//!
//! ## What AIS is
//!
//! The compiler pipeline produces a semantic model
//! (lexer/parser/checker/module system), and AIS describes that model:
//! documents, symbols, declarations, types (including the Keystone families:
//! `T | none`, `never`, structs, collection families, function signatures),
//! scopes, modules, diagnostics, flow facts, and completion context.
//!
//! ## What AIS is not
//!
//! AIS is not an LLM provider, an agent framework, the Aura-0.4 AI stdlib, or
//! an authority source. Producing an AIS document grants **no** capability:
//! no filesystem, environment, secret, network, Python, or Host authority
//! flows through it. It describes what the compiler already knows about the
//! program the caller submitted.
//!
//! ## Versioning
//!
//! An AIS document carries both `ais_version` (the protocol) and
//! `aura_version` (the compiler that produced it). They are independent: a
//! tool negotiates capabilities against the protocol version and may record
//! the compiler version for reproducibility. Consumers must ignore unknown
//! fields, and a producer never removes a field within a minor version.

use serde::{Deserialize, Serialize};

use crate::ast::{Item, Module, Stmt, TypeExpr};
use crate::error::{Diag, Severity};

/// The protocol version this module produces.
pub const AIS_VERSION: &str = "0.1";

/// Semantic protocol capabilities a producer advertises (Keystone §36).
///
/// Capabilities describe *which parts of the model* this document can carry.
/// They are not authority grants: a consumer that sees `diagnostics` can read
/// diagnostics; it still cannot read a file or reach the network.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
// The capability set is deliberately a flat set of booleans: it is a wire
// contract where field names are the API, not an internal flag bag.
#[allow(clippy::struct_excessive_bools)]
pub struct Capabilities {
    /// The document carries declarations and their signatures.
    pub symbols: bool,
    /// The document carries resolved type spellings.
    pub types: bool,
    /// The document carries structured diagnostics.
    pub diagnostics: bool,
    /// The document carries flow facts (narrowing, divergence).
    pub flow: bool,
    /// The document carries completion context.
    pub completion: bool,
}

impl Default for Capabilities {
    fn default() -> Self {
        // Honest advertisement: a 0.1 document produced by `document()`
        // carries symbols (with their type spellings, so `types` too) and,
        // once the caller attaches them, diagnostics. It does not yet
        // populate flow facts or completion context — the schema reserves
        // the capability bits for them, but a producer must not claim a part
        // of the model it does not actually carry.
        Capabilities {
            symbols: true,
            types: true,
            diagnostics: true,
            flow: false,
            completion: false,
        }
    }
}

/// A position in a document: 1-based line and column, plus the byte offset.
///
/// Both the human coordinate and the byte coordinate are carried so a tool
/// never has to recompute one from the other (and never has to assume an
/// encoding).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Position {
    /// 1-based line.
    pub line: u32,
    /// 1-based column, in characters.
    pub column: u32,
    /// 0-based byte offset.
    pub offset: usize,
}

/// A half-open byte range with resolved positions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Range {
    /// Start.
    pub start: Position,
    /// One past the end.
    pub end: Position,
}

/// A diagnostic in protocol form.
///
/// This is the same semantic payload the CLI renders, without any
/// presentation: `severity` is data, and no ANSI or formatted text crosses
/// the protocol boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    /// The stable `E####` code, e.g. `3003`.
    pub code: u16,
    /// The canonical spelling, e.g. `E3003`.
    pub code_text: String,
    /// `error`, `warning`, or `note`.
    pub severity: String,
    /// The human message.
    pub message: String,
    /// Where it happened, when the document has a source for it.
    pub range: Option<Range>,
    /// Additional context.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    /// A suggested remedy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
}

impl Diagnostic {
    /// Convert a compiler diagnostic into its protocol form.
    ///
    /// `source` is the document text the offset resolves against; when it is
    /// absent or the span is not a valid range, `range` is omitted rather than
    /// fabricated.
    #[must_use]
    pub fn from_diag(d: &Diag, source: Option<&str>) -> Diagnostic {
        Diagnostic {
            code: d.code,
            code_text: format!("E{:04}", d.code),
            severity: Severity::Error.as_str().to_string(),
            message: d.message.clone(),
            range: source.and_then(|src| span_range(src, d.span.start, d.span.end)),
            notes: Vec::new(),
            help: None,
        }
    }

    /// Attach structured presentation from a compiler report.
    #[must_use]
    pub fn with_presentation(mut self, p: &crate::error::Presentation) -> Diagnostic {
        self.severity = p.severity.as_str().to_string();
        self.notes.clone_from(&p.notes);
        self.help.clone_from(&p.help);
        self
    }
}

fn position_of(src: &str, offset: usize) -> Position {
    let (line, column) = crate::error::line_col(src, offset);
    Position {
        line,
        column,
        offset,
    }
}

fn span_range(src: &str, start: usize, end: usize) -> Option<Range> {
    if start > end || end > src.len() || !src.is_char_boundary(start) || !src.is_char_boundary(end)
    {
        return None;
    }
    Some(Range {
        start: position_of(src, start),
        end: position_of(src, end),
    })
}

/// The kind of a symbol, in the Keystone families.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SymbolKind {
    /// A function.
    Function,
    /// A struct (nominal data).
    Struct,
    /// An enum (nominal variants).
    Enum,
    /// A variant of an enum.
    Variant,
    /// A field of a struct.
    Field,
    /// A type alias.
    TypeAlias,
    /// A generic type parameter.
    TypeParameter,
    /// A module-scope binding (`const`/top-level `let`).
    Constant,
    /// An imported name (`use`).
    Import,
    /// A trait.
    Trait,
    /// A module.
    Module,
}

/// A function parameter in protocol form.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Parameter {
    /// Name (`_` is a discard).
    pub name: String,
    /// The declared type, when annotated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Whether `mut` was declared.
    pub mutable: bool,
    /// The parameter's range.
    pub range: Range,
}

/// A field of a struct.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Field {
    /// Field name.
    pub name: String,
    /// Declared type spelling.
    pub type_name: String,
    /// The field's range.
    pub range: Range,
}

/// A symbol: a declaration the compiler knows about.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Symbol {
    /// The declared name.
    pub name: String,
    /// Which family it belongs to.
    pub kind: SymbolKind,
    /// Where it is declared.
    pub range: Range,
    /// Whether it is exported (`pub`).
    pub public: bool,
    /// The resolved type or signature spelling, when the kind has one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,
    /// Generic type parameters, in declaration order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_parameters: Vec<String>,
    /// Parameters, for callables.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parameters: Vec<Parameter>,
    /// Fields, for structs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// Variant names, for enums.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub variants: Vec<String>,
}

fn type_spelling(t: &TypeExpr) -> String {
    t.name().clone()
}

fn symbol_of_item(item: &Item) -> Option<Symbol> {
    let (name, kind, span, public) = match item {
        Item::Fn {
            name, span, public, ..
        } => (name.clone(), SymbolKind::Function, *span, *public),
        Item::Struct {
            name, span, public, ..
        } => (name.clone(), SymbolKind::Struct, *span, *public),
        Item::Enum {
            name, span, public, ..
        } => (name.clone(), SymbolKind::Enum, *span, *public),
        Item::Alias {
            name, span, public, ..
        } => (name.clone(), SymbolKind::TypeAlias, *span, *public),
        Item::Const {
            name, span, public, ..
        } => (name.clone(), SymbolKind::Constant, *span, *public),
        Item::Trait {
            name, span, public, ..
        } => (name.clone(), SymbolKind::Trait, *span, *public),
        Item::Module {
            name, span, public, ..
        } => (name.clone(), SymbolKind::Module, *span, *public),
        Item::Use { span, public, .. } => (String::new(), SymbolKind::Import, *span, *public),
        _ => return None,
    };
    let mut sym = Symbol {
        name,
        kind,
        range: Range {
            start: Position {
                line: 0,
                column: 0,
                offset: span.start,
            },
            end: Position {
                line: 0,
                column: 0,
                offset: span.end,
            },
        },
        public,
        type_name: None,
        type_parameters: Vec::new(),
        parameters: Vec::new(),
        fields: Vec::new(),
        variants: Vec::new(),
    };
    match item {
        Item::Fn {
            type_params,
            params,
            ret,
            ..
        } => {
            sym.type_parameters = type_params.iter().map(|p| p.name.clone()).collect();
            sym.parameters = params
                .iter()
                .map(|p| Parameter {
                    name: p.name.clone(),
                    type_name: p.ty.as_ref().map(type_spelling),
                    mutable: p.mutable,
                    range: Range {
                        start: Position {
                            line: 0,
                            column: 0,
                            offset: p.span.start,
                        },
                        end: Position {
                            line: 0,
                            column: 0,
                            offset: p.span.end,
                        },
                    },
                })
                .collect();
            sym.type_name = ret.as_ref().map(type_spelling);
        }
        Item::Struct {
            type_params,
            fields,
            ..
        } => {
            sym.type_parameters = type_params.iter().map(|p| p.name.clone()).collect();
            sym.fields = fields
                .iter()
                .map(|f| Field {
                    name: f.name.clone(),
                    type_name: type_spelling(&f.ty),
                    range: Range {
                        start: Position {
                            line: 0,
                            column: 0,
                            offset: f.span.start,
                        },
                        end: Position {
                            line: 0,
                            column: 0,
                            offset: f.span.end,
                        },
                    },
                })
                .collect();
        }
        Item::Enum {
            type_params,
            variants,
            ..
        } => {
            sym.type_parameters = type_params.iter().map(|p| p.name.clone()).collect();
            sym.variants = variants.iter().map(|v| v.tag.clone()).collect();
        }
        Item::Alias { target, .. } => sym.type_name = Some(type_spelling(target)),
        _ => {}
    }
    Some(sym)
}

/// Resolve every `line`/`column` in a symbol against `source`.
fn resolve_symbol(sym: &mut Symbol, source: &str) {
    fn resolve(pos: &mut Position, source: &str) {
        let (line, column) = crate::error::line_col(source, pos.offset);
        pos.line = line;
        pos.column = column;
    }
    resolve(&mut sym.range.start, source);
    resolve(&mut sym.range.end, source);
    for p in &mut sym.parameters {
        resolve(&mut p.range.start, source);
        resolve(&mut p.range.end, source);
    }
    for f in &mut sym.fields {
        resolve(&mut f.range.start, source);
        resolve(&mut f.range.end, source);
    }
}

/// A conservative flow fact the checker can rely on at a program point.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FlowFact {
    /// The binding the fact is about.
    pub name: String,
    /// The narrowed type, when a `none` guard removed a member.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub narrowed_type: Option<String>,
    /// Whether the path cannot continue (a `never`/diverging point).
    pub diverges: bool,
}

/// Where completions would be offered.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompletionContext {
    /// The position the request is about.
    pub position: Position,
    /// Names visible in the enclosing scope, sorted.
    pub visible_symbols: Vec<String>,
}

/// A semantic document: the AIS/0.1 payload for one source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    /// The AIS protocol version.
    pub ais_version: String,
    /// The compiler version that produced this document.
    pub aura_version: String,
    /// The language version the compiler implements.
    pub language_version: String,
    /// What this document carries.
    pub capabilities: Capabilities,
    /// The source display name.
    pub source_name: String,
    /// Declarations, in source order.
    pub symbols: Vec<Symbol>,
    /// Structured diagnostics, when checking produced any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// Build an AIS document for a parsed module.
///
/// The document describes the module's declarations and their signatures. It
/// performs no checking itself: pass `diagnostics` from the checker to attach
/// structured diagnostics. No capability is granted by producing a document.
#[must_use]
pub fn document(source_name: &str, source: &str, module: &Module) -> Document {
    let mut symbols: Vec<Symbol> = module.items.iter().filter_map(symbol_of_item).collect();
    for s in &mut symbols {
        resolve_symbol(s, source);
    }
    Document {
        ais_version: AIS_VERSION.to_string(),
        aura_version: crate::VERSION.to_string(),
        language_version: crate::LANGUAGE_VERSION.to_string(),
        capabilities: Capabilities::default(),
        source_name: source_name.to_string(),
        symbols,
        diagnostics: Vec::new(),
    }
}

/// Whether a statement list contains a diverging trailing statement.
///
/// This mirrors the checker's conservative divergence rule for the `never`
/// contract and exposes it as a flow fact. It is intentionally conservative:
/// an unrecognized shape reports `false`.
#[must_use]
pub fn block_diverges(body: &[Stmt]) -> bool {
    matches!(body.last(), Some(Stmt::Return(..) | Stmt::Throw(..)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;
    use crate::parse::parse;

    const SRC: &str = "struct User { name: string, email: string | none }\n\
                       fn find(ok: bool) -> User | none {\n    return none\n}\n\
                       fn fail(msg: string) -> never { throw msg }\n\
                       enum Color { Red, Green }\n\
                       trait Show { fn show(self) -> int }\n\
                       type Id = int\n\
                       const LIMIT = 10\n";

    #[test]
    fn document_carries_versions_and_capabilities() {
        let module = parse(SRC).expect("parses");
        let doc = document("main.aura", SRC, &module);
        assert_eq!(doc.ais_version, "0.1");
        assert_eq!(doc.aura_version, crate::VERSION);
        assert_eq!(doc.language_version, crate::LANGUAGE_VERSION);
        assert!(doc.capabilities.symbols);
        assert!(doc.capabilities.diagnostics);
    }

    #[test]
    fn struct_fields_and_optional_types_are_described() {
        let module = parse(SRC).expect("parses");
        let doc = document("main.aura", SRC, &module);
        let user = doc
            .symbols
            .iter()
            .find(|s| s.name == "User")
            .expect("User symbol");
        assert_eq!(user.kind, SymbolKind::Struct);
        let fields: Vec<(String, String)> = user
            .fields
            .iter()
            .map(|f| (f.name.clone(), f.type_name.clone()))
            .collect();
        assert!(fields.contains(&("name".to_string(), "string".to_string())));
        // The optional field keeps its full spelling: the union is not erased.
        assert!(fields
            .iter()
            .any(|(n, t)| n == "email" && t.contains("none") && t.contains("string")));
    }

    #[test]
    fn never_and_function_signatures_are_described() {
        let module = parse(SRC).expect("parses");
        let doc = document("main.aura", SRC, &module);
        let fail = doc
            .symbols
            .iter()
            .find(|s| s.name == "fail")
            .expect("fail symbol");
        assert_eq!(fail.kind, SymbolKind::Function);
        assert_eq!(fail.type_name.as_deref(), Some("never"));
        assert_eq!(fail.parameters.len(), 1);
        assert_eq!(fail.parameters[0].name, "msg");
        assert_eq!(fail.parameters[0].type_name.as_deref(), Some("string"));
    }

    #[test]
    fn enum_variants_and_traits_are_described() {
        let module = parse(SRC).expect("parses");
        let doc = document("main.aura", SRC, &module);
        let color = doc
            .symbols
            .iter()
            .find(|s| s.name == "Color")
            .expect("Color");
        assert_eq!(color.variants, vec!["Red", "Green"]);
        let show = doc.symbols.iter().find(|s| s.name == "Show").expect("Show");
        assert_eq!(show.kind, SymbolKind::Trait);
    }

    #[test]
    fn positions_resolve_to_line_and_column() {
        let module = parse(SRC).expect("parses");
        let doc = document("main.aura", SRC, &module);
        let user = doc.symbols.iter().find(|s| s.name == "User").unwrap();
        assert_eq!(user.range.start.line, 1);
        let field = user.fields.first().unwrap();
        assert_eq!(field.range.start.line, 1);
        assert!(field.range.start.column > 1);
    }

    #[test]
    fn diagnostics_are_structured_and_never_carry_ansi() {
        let d = Diag::new(
            crate::error::codes::POSSIBLE_NONE,
            "may be none",
            crate::error::Span::new(0, 3),
        );
        let diag = Diagnostic::from_diag(&d, Some("abc"));
        assert_eq!(diag.code_text, "E3003");
        assert_eq!(diag.severity, "error");
        assert!(diag.range.is_some());
        assert!(!diag.message.contains('\x1b'));
    }

    #[test]
    fn document_serializes_to_stable_json() {
        let module = parse(SRC).expect("parses");
        let doc = document("main.aura", SRC, &module);
        let json = serde_json::to_string(&doc).expect("serializes");
        assert!(json.contains("\"ais_version\":\"0.1\""));
        assert!(json.contains("\"never\""));
        // Round-trips: the schema is an external contract, not a debug dump.
        let back: Document = serde_json::from_str(&json).expect("deserializes");
        assert_eq!(back, doc);
    }

    #[test]
    fn unknown_fields_are_ignored_for_forward_compatibility() {
        // A future producer may add fields; a 0.1 consumer must not break.
        let json = r#"{"ais_version":"0.1","aura_version":"0.3.0","language_version":"0.3.0",
            "capabilities":{"symbols":true,"types":true,"diagnostics":true,"flow":true,"completion":true},
            "source_name":"x.aura","symbols":[],"diagnostics":[],"future_field":42}"#;
        let doc: Document = serde_json::from_str(json).expect("forward compatible");
        assert_eq!(doc.ais_version, "0.1");
    }
}
