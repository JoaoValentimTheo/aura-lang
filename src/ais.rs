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
    /// The semantic type's capability families, in canonical order
    /// (`LANGUAGE_SPEC.md` §5.4 level 2). Projected from the resolved type,
    /// never a replacement for `type_name`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub families: Vec<String>,
    /// The runtime value kind the resolved type pins, when it pins exactly one
    /// (`LANGUAGE_SPEC.md` §5.4 level 3). Absent for a union, a generic
    /// parameter, an unresolved application, or the checker's `Unknown`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_kind: Option<String>,
    /// The capabilities implied by the families, so a consumer need not infer
    /// them: `iterable`, `indexable`, `sized`, `callable`, `keyed`,
    /// `orderable`, `mutable`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
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

/// The declarations a projection needs to resolve a type spelling into a
/// semantic type. A name in this environment resolves to a struct, an enum, an
/// alias target, or is not a value type at all (a trait, a module, a generic
/// parameter).
#[derive(Default)]
struct TypeEnv {
    structs: std::collections::HashSet<String>,
    enums: std::collections::HashSet<String>,
    aliases: std::collections::HashMap<String, TypeExpr>,
}

impl TypeEnv {
    fn from_module(module: &Module) -> TypeEnv {
        let mut env = TypeEnv::default();
        for item in &module.items {
            match item {
                Item::Struct { name, .. } => {
                    env.structs.insert(name.clone());
                }
                Item::Enum { name, .. } => {
                    env.enums.insert(name.clone());
                }
                Item::Alias { name, target, .. } => {
                    env.aliases.insert(name.clone(), target.clone());
                }
                _ => {}
            }
        }
        env
    }
}

/// Resolve a written type spelling to a checker type for projection.
///
/// Unlike the checker's `Ty::from_expr`, an unknown name resolves to
/// `Ty::Unknown` (no family claim) rather than an opaque nominal type, and a
/// generic parameter in scope stays `Ty::Param` (also no claim). Alias
/// resolution is bounded so a cyclic alias cannot loop here.
fn project_expr(t: &TypeExpr, env: &TypeEnv, params: &[String], depth: usize) -> crate::types::Ty {
    use crate::types::Ty;
    match t {
        TypeExpr::Int => Ty::Int,
        TypeExpr::Float => Ty::Float,
        TypeExpr::Bool => Ty::Bool,
        TypeExpr::String => Ty::String,
        TypeExpr::None => Ty::None,
        TypeExpr::Never => Ty::Never,
        TypeExpr::List(inner) => Ty::List(Box::new(project_expr(inner, env, params, depth))),
        TypeExpr::Map(k, v) => Ty::Map(
            Box::new(project_expr(k, env, params, depth)),
            Box::new(project_expr(v, env, params, depth)),
        ),
        TypeExpr::Union(members) => Ty::union(
            members
                .iter()
                .map(|m| project_expr(m, env, params, depth))
                .collect(),
        ),
        TypeExpr::Named(n) => {
            if params.iter().any(|p| p == n) {
                Ty::Param(n.clone())
            } else if env.structs.contains(n) {
                Ty::Named(n.clone())
            } else if env.enums.contains(n) {
                Ty::Enum(n.clone())
            } else if depth < 16 {
                match env.aliases.get(n) {
                    Some(target) => project_expr(target, env, params, depth + 1),
                    None => Ty::Unknown,
                }
            } else {
                Ty::Unknown
            }
        }
        TypeExpr::App(n, _) => {
            if env.structs.contains(n) {
                Ty::Named(n.clone())
            } else if env.enums.contains(n) {
                Ty::Enum(n.clone())
            } else {
                Ty::Unknown
            }
        }
    }
}

/// Project a checker type's three identity levels into protocol fields.
///
/// A symbol whose declared spelling is a user type is resolved through the
/// module's declarations before projection, so an alias to `[int]` reports the
/// `sequence` family rather than an opaque name, and an unresolved name yields
/// no family claim.
fn project_ty(t: &crate::types::Ty) -> (Vec<String>, Option<String>, Vec<String>) {
    use crate::types::{Ty, TypeFamily};
    let families: Vec<String> = t
        .families()
        .iter()
        .map(|f| f.as_str().to_string())
        .filter(|f| f != "unknown")
        .collect();
    let value_kind = match t.value_kind() {
        "unknown" | "union" | "never" => None,
        k => Some(k.to_string()),
    };
    // Capabilities mirror the executable truth (`LANGUAGE_SPEC.md` §5.3):
    //   `len`        accepts string, list, map, range
    //   iteration    accepts string, list, map (keys), range
    //   indexing     accepts string, list, map, struct (by field name)
    //   mutation     is shared for list, map, and struct fields
    //   ordering     is defined for int, float, bool, string
    let mut caps: Vec<String> = Vec::new();
    if matches!(t, Ty::String | Ty::List(_) | Ty::Map(_, _))
        || matches!(t, Ty::Named(n) if n != "range")
    {
        caps.push("indexable".into());
    }
    if matches!(t, Ty::String | Ty::List(_) | Ty::Map(_, _))
        || matches!(t, Ty::Named(n) if n == "range")
    {
        caps.push("iterable".into());
    }
    if matches!(t, Ty::String | Ty::List(_) | Ty::Map(_, _))
        || matches!(t, Ty::Named(n) if n == "range")
    {
        caps.push("sized".into());
    }
    if matches!(t, Ty::List(_) | Ty::Map(_, _)) || matches!(t, Ty::Named(n) if n != "range") {
        caps.push("mutable".into());
    }
    if matches!(t, Ty::Int | Ty::Float | Ty::Bool | Ty::String) {
        caps.push("orderable".into());
    }
    if t.families().contains(&TypeFamily::Callable) {
        caps.push("callable".into());
    }
    caps.sort();
    caps.dedup();
    (families, value_kind, caps)
}

fn symbol_of_item(item: &Item, env: &TypeEnv) -> Option<Symbol> {
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
        Item::Use {
            path,
            alias,
            span,
            public,
        } => {
            // An import's symbol name is the bound local name: the alias when
            // written, otherwise the last path segment. An empty name would
            // make the symbol unusable to a tool.
            let name = alias
                .clone()
                .or_else(|| path.last().cloned())
                .unwrap_or_default();
            (name, SymbolKind::Import, *span, *public)
        }
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
        families: Vec::new(),
        value_kind: None,
        capabilities: Vec::new(),
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
    // Project the three identity levels for the symbol's *value* type, when it
    // has one. A function's value type is its return type; a struct, enum, or
    // alias symbol projects its own declared type; a trait or module value has
    // no family claim.
    let value_ty = match item {
        Item::Fn { ret, .. } => ret
            .as_ref()
            .map(|t| project_expr(t, env, &sym.type_parameters, 0)),
        Item::Struct { name, .. } => Some(crate::types::Ty::Named(name.clone())),
        Item::Enum { name, .. } => Some(crate::types::Ty::Enum(name.clone())),
        Item::Alias { target, .. } => Some(project_expr(target, env, &sym.type_parameters, 0)),
        Item::Const { ann, .. } => ann.as_ref().map(|t| project_expr(t, env, &[], 0)),
        _ => None,
    };
    if let Some(t) = value_ty {
        let (families, value_kind, capabilities) = project_ty(&t);
        sym.families = families;
        sym.value_kind = value_kind;
        sym.capabilities = capabilities;
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
    /// A content-addressed revision identity for the source this document
    /// describes. Two documents of the same source text share a revision, so a
    /// consumer can detect "nothing changed" without comparing payloads, and a
    /// delta is computed between two revisions rather than two ad-hoc copies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// Declarations, in source order.
    pub symbols: Vec<Symbol>,
    /// Flow narrowings the checker proved, when the document was checked.
    /// A consumer must never redo flow analysis the compiler already did
    /// (Keystone §13): each fact names the binding and the type the checker
    /// proved inside a guarded region.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub narrowings: Vec<FlowFact>,
    /// Structured diagnostics, when checking produced any.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
}

/// A content-addressed source revision: the FNV-1a 64-bit hash of the exact
/// source bytes, rendered as `rev:<16 hex digits>`.
///
/// This is deliberately a *content* identity, not an edit counter: inserting an
/// unrelated line only changes the revision if it changes these bytes, and the
/// same bytes always produce the same identity. It is not cryptographic and
/// MUST NOT be used for integrity or security decisions — it exists so a
/// long-running agent session can ask "is this the revision I already hold?".
#[must_use]
pub fn revision_of(source: &str) -> String {
    // FNV-1a 64-bit: tiny, dependency-free, and stable across platforms.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in source.as_bytes() {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    format!("rev:{hash:016x}")
}

/// One changed, added, or removed symbol in a [`Delta`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SymbolChange {
    /// The symbol's name.
    pub name: String,
    /// `added`, `removed`, `changed`, or `moved`.
    ///
    /// `changed` means the declaration's *semantics* differ (kind, type,
    /// families, value kind, capabilities, type parameters, parameters,
    /// fields, variants, or visibility). `moved` means only its source range
    /// moved — an unrelated insertion above it shifts its position without
    /// changing what is declared — so an agent session is told to refresh
    /// positions without being told the declaration changed.
    pub change: String,
    /// The *canonical* serialized form of the symbol after the change (absent
    /// for `removed`). A consumer compares this to decide whether a change
    /// affects its task; the delta never asks the consumer to re-read the
    /// whole document.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Symbol>,
}

/// Compare two symbols ignoring their source ranges.
///
/// Two declarations with the same name are semantically the same when
/// everything except where they were written is equal: a comment inserted
/// above a declaration moves it without changing what it declares.
fn same_declaration(a: &Symbol, b: &Symbol) -> bool {
    let mut a = a.clone();
    let mut b = b.clone();
    let scrub = |s: &mut Symbol| {
        s.range = Range {
            start: Position {
                line: 0,
                column: 0,
                offset: 0,
            },
            end: Position {
                line: 0,
                column: 0,
                offset: 0,
            },
        };
        for p in &mut s.parameters {
            p.range = s.range.clone();
        }
        for f in &mut s.fields {
            f.range = s.range.clone();
        }
    };
    scrub(&mut a);
    scrub(&mut b);
    a == b
}

/// A semantic delta between two source revisions.
///
/// The delta is computed from two documents, so it is exactly the set of
/// symbol-level differences: changed, added, and removed declarations, plus
/// the diagnostics that resolved and the diagnostics that appeared. A consumer
/// applies a delta to the snapshot it holds instead of receiving the world.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Delta {
    /// The AIS protocol version.
    pub ais_version: String,
    /// The revision the consumer is expected to hold.
    pub from_revision: String,
    /// The revision this delta produces.
    pub to_revision: String,
    /// Symbol-level changes, sorted by name.
    pub symbols: Vec<SymbolChange>,
    /// Diagnostic codes present before and gone now (resolved).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved_diagnostics: Vec<u16>,
    /// Diagnostic codes present now and absent before (new).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub new_diagnostics: Vec<u16>,
}

impl Delta {
    /// Whether this delta carries no semantic change.
    ///
    /// A `moved` symbol is a position refresh, not a semantic change, so it
    /// does not make the delta non-empty: an unrelated edit above a
    /// declaration must not tell a consumer that the declaration changed.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.symbols.iter().any(|c| c.change != "moved")
            && self.resolved_diagnostics.is_empty()
            && self.new_diagnostics.is_empty()
    }

    /// Whether the delta carries any change at all, including position moves.
    #[must_use]
    pub fn has_moves(&self) -> bool {
        self.symbols.iter().any(|c| c.change == "moved")
    }
}

/// Compute the semantic delta between two documents.
///
/// Symbols are matched by name. A symbol whose canonical serialization differs
/// is `changed`; a name only in `to` is `added`; a name only in `from` is
/// `removed`. Diagnostic identity is the `E####` code.
#[must_use]
pub fn delta(from: &Document, to: &Document) -> Delta {
    use std::collections::BTreeMap;
    let before: BTreeMap<&str, &Symbol> =
        from.symbols.iter().map(|s| (s.name.as_str(), s)).collect();
    let after: BTreeMap<&str, &Symbol> = to.symbols.iter().map(|s| (s.name.as_str(), s)).collect();
    let mut changes: Vec<SymbolChange> = Vec::new();
    for (name, sym) in &after {
        match before.get(name) {
            Some(old) if old == sym => {}
            Some(old) if same_declaration(old, sym) => changes.push(SymbolChange {
                name: (*name).to_string(),
                change: "moved".into(),
                after: Some((*sym).clone()),
            }),
            Some(_) => changes.push(SymbolChange {
                name: (*name).to_string(),
                change: "changed".into(),
                after: Some((*sym).clone()),
            }),
            None => changes.push(SymbolChange {
                name: (*name).to_string(),
                change: "added".into(),
                after: Some((*sym).clone()),
            }),
        }
    }
    for name in before.keys() {
        if !after.contains_key(name) {
            changes.push(SymbolChange {
                name: (*name).to_string(),
                change: "removed".into(),
                after: None,
            });
        }
    }
    changes.sort_by(|a, b| a.name.cmp(&b.name));
    let old_codes: std::collections::BTreeSet<u16> =
        from.diagnostics.iter().map(|d| d.code).collect();
    let new_codes: std::collections::BTreeSet<u16> =
        to.diagnostics.iter().map(|d| d.code).collect();
    Delta {
        ais_version: AIS_VERSION.to_string(),
        from_revision: from.revision.clone().unwrap_or_default(),
        to_revision: to.revision.clone().unwrap_or_default(),
        symbols: changes,
        resolved_diagnostics: old_codes.difference(&new_codes).copied().collect(),
        new_diagnostics: new_codes.difference(&old_codes).copied().collect(),
    }
}

/// A task-focused semantic slice: the minimal sufficient context for a task
/// anchored at one target.
///
/// A slice never re-sends the whole document. It carries the target's own
/// symbol, the symbols it names (its declared dependencies, by name), and the
/// diagnostics that mention it or its dependencies, bounded by `budget`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Slice {
    /// The AIS protocol version.
    pub ais_version: String,
    /// The revision the slice was taken from.
    pub revision: String,
    /// The requested target name.
    pub target: String,
    /// The target symbol, when the document declares it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub symbol: Option<Symbol>,
    /// The directly referenced declarations, sorted by name, bounded by the
    /// budget. Depth 1 is this list; `dependency_depth` widens it.
    pub dependencies: Vec<Symbol>,
    /// Diagnostics whose span touches the slice's resolution, when known.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<Diagnostic>,
    /// How many symbols were available but omitted by the budget, so a consumer
    /// knows the slice is incomplete rather than empty.
    pub omitted: usize,
}

/// Build a task-focused slice around `target`.
///
/// `dependency_depth` is the transitive closure depth (1 = direct references);
/// `budget` caps the number of symbols carried (the target included). A target
/// that is not declared yields an empty slice rather than an error, matching
/// the protocol's "describe what is known" rule.
#[must_use]
pub fn slice(doc: &Document, target: &str, dependency_depth: usize, budget: usize) -> Slice {
    let by_name: std::collections::HashMap<&str, &Symbol> =
        doc.symbols.iter().map(|s| (s.name.as_str(), s)).collect();
    let Some(root) = by_name.get(target) else {
        return Slice {
            ais_version: AIS_VERSION.to_string(),
            revision: doc.revision.clone().unwrap_or_default(),
            target: target.to_string(),
            symbol: None,
            dependencies: Vec::new(),
            diagnostics: Vec::new(),
            omitted: 0,
        };
    };
    let mut seen: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    seen.insert(root.name.clone());
    let mut frontier: Vec<&Symbol> = vec![root];
    for _ in 0..dependency_depth {
        let mut next: Vec<&Symbol> = Vec::new();
        for sym in &frontier {
            for name in referenced_names(sym) {
                if let Some(dep) = by_name.get(name.as_str()) {
                    if seen.insert(dep.name.clone()) {
                        next.push(dep);
                    }
                }
            }
        }
        if next.is_empty() {
            break;
        }
        frontier = next;
    }
    let mut names: Vec<String> = seen.into_iter().collect();
    names.sort();
    // The target is always carried: the budget bounds the *dependencies*, so a
    // tiny budget degrades to "the target alone", never to "some unrelated
    // symbol" and never to a silent target omission.
    let mut deps: Vec<String> = names.into_iter().filter(|n| n != target).collect();
    let dep_budget = budget.saturating_sub(1);
    let omitted = deps.len().saturating_sub(dep_budget);
    deps.truncate(dep_budget);
    let dependencies: Vec<Symbol> = deps
        .iter()
        .filter_map(|n| by_name.get(n.as_str()).map(|s| (*s).clone()))
        .collect();
    Slice {
        ais_version: AIS_VERSION.to_string(),
        revision: doc.revision.clone().unwrap_or_default(),
        target: target.to_string(),
        symbol: Some((*root).clone()),
        dependencies,
        diagnostics: Vec::new(),
        omitted,
    }
}

/// The names a symbol's signature references, for slice closure.
fn referenced_names(sym: &Symbol) -> Vec<String> {
    let mut out = Vec::new();
    let mut note = |spelling: &str| {
        for token in spelling.split(|c: char| !(c.is_alphanumeric() || c == '_')) {
            if token.is_empty() || token.chars().next().is_some_and(|c| c.is_numeric()) {
                continue;
            }
            out.push(token.to_string());
        }
    };
    if let Some(t) = &sym.type_name {
        note(t);
    }
    for p in &sym.parameters {
        if let Some(t) = &p.type_name {
            note(t);
        }
    }
    for f in &sym.fields {
        note(&f.type_name);
    }
    out.sort();
    out.dedup();
    out
}

/// Build an AIS document for a parsed module.
///
/// The document describes the module's declarations and their signatures. It
/// performs no checking itself: pass `diagnostics` from the checker to attach
/// structured diagnostics. No capability is granted by producing a document.
#[must_use]
pub fn document(source_name: &str, source: &str, module: &Module) -> Document {
    let env = TypeEnv::from_module(module);
    let mut symbols: Vec<Symbol> = module
        .items
        .iter()
        .filter_map(|item| symbol_of_item(item, &env))
        .collect();
    for s in &mut symbols {
        resolve_symbol(s, source);
    }
    Document {
        ais_version: AIS_VERSION.to_string(),
        aura_version: crate::VERSION.to_string(),
        language_version: crate::LANGUAGE_VERSION.to_string(),
        capabilities: Capabilities::default(),
        source_name: source_name.to_string(),
        revision: Some(revision_of(source)),
        symbols,
        narrowings: Vec::new(),
        diagnostics: Vec::new(),
    }
}

/// Build a checked AIS document: the snapshot plus any checker diagnostic and
/// the flow narrowings the checker proved.
///
/// This is the canonical entry point for a tool that wants the compiler's
/// conclusions (AIS §13): the narrowing facts come from the checker's own
/// proof, not from a second analysis. A rejected program still yields a
/// document, with the diagnostic and without narrowings.
#[must_use]
pub fn checked_document(source_name: &str, source: &str, module: &Module) -> Document {
    let mut doc = document(source_name, source, module);
    let (outcome, facts) = crate::check::Checker::module_narrowings(module);
    if let Err(d) = outcome {
        doc.diagnostics
            .push(Diagnostic::from_diag(&d, Some(source)));
    }
    doc.narrowings = facts
        .into_iter()
        .map(|(name, ty)| FlowFact {
            name,
            narrowed_type: Some(ty.name()),
            diverges: false,
        })
        .collect();
    doc
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

    // -----------------------------------------------------------------------
    // Type / Family / ValueKind separation and the delivery model
    // -----------------------------------------------------------------------

    #[test]
    fn a_list_symbol_reports_the_sequence_family_and_list_value_kind() {
        let src = "fn rows() -> [[int]] { return [] }\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        let rows = doc.symbols.iter().find(|s| s.name == "rows").unwrap();
        // Family is `sequence`; value kind is `list`; both are present and
        // they are *different* strings for the same type.
        assert_eq!(rows.families, vec!["sequence"]);
        assert_eq!(rows.value_kind.as_deref(), Some("list"));
        assert!(rows.capabilities.contains(&"iterable".to_string()));
        assert!(rows.capabilities.contains(&"indexable".to_string()));
        assert!(rows.capabilities.contains(&"mutable".to_string()));
    }

    #[test]
    fn a_string_symbol_is_scalar_and_sequence_but_its_value_kind_is_string() {
        let src = "fn name() -> string { return \"x\" }\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        let name = doc.symbols.iter().find(|s| s.name == "name").unwrap();
        // A string is in two families; the family set is not the value kind.
        assert_eq!(name.families, vec!["scalar", "sequence"]);
        assert_eq!(name.value_kind.as_deref(), Some("string"));
        assert!(name.capabilities.contains(&"orderable".to_string()));
        assert!(name.capabilities.contains(&"iterable".to_string()));
    }

    #[test]
    fn a_struct_symbol_projects_object_not_a_map() {
        let src = "struct P { x: int }\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        let p = doc.symbols.iter().find(|s| s.name == "P").unwrap();
        assert_eq!(p.families, vec!["object"]);
        assert_eq!(p.value_kind.as_deref(), Some("struct"));
        // No map key capability is claimed merely because both are keyed.
        assert!(!p.capabilities.contains(&"iterable".to_string()));
    }

    #[test]
    fn an_alias_projects_the_target_not_the_alias_name() {
        let src = "type Names = [string]\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        let names = doc.symbols.iter().find(|s| s.name == "Names").unwrap();
        assert_eq!(names.kind, SymbolKind::TypeAlias);
        assert_eq!(names.families, vec!["sequence"]);
        assert_eq!(names.value_kind.as_deref(), Some("list"));
    }

    #[test]
    fn a_generic_parameter_claims_no_family() {
        let src = "fn id<T>(x: T) -> T { return x }\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        let id = doc.symbols.iter().find(|s| s.name == "id").unwrap();
        // `T` is not known, so no family and no value kind are invented.
        assert!(id.families.is_empty());
        assert_eq!(id.value_kind, None);
    }

    #[test]
    fn revision_is_content_addressed_and_stable() {
        let a = revision_of("fn main() { print(1) }");
        let b = revision_of("fn main() { print(1) }");
        let c = revision_of("fn main() { print(2) }");
        assert_eq!(a, b, "same bytes must share a revision");
        assert_ne!(a, c, "different bytes must differ");
        assert!(a.starts_with("rev:"));
        // The same source produces the same revision through the document.
        let module = parse("fn main() { print(1) }").expect("parses");
        let doc = document("m.aura", "fn main() { print(1) }", &module);
        assert_eq!(doc.revision.as_deref(), Some(a.as_str()));
    }

    #[test]
    fn delta_reports_added_changed_and_removed_symbols() {
        // A *declaration* change is a signature change: the delta is defined
        // over the declaration-level model AIS carries (name, kind, type,
        // fields, parameters), not over statement bodies. Changing only a body
        // is not a symbol change here — see
        // `delta_is_declaration_level_not_body_level`.
        let before_src = "fn a(x: int) -> int { return x }\nfn b() -> int { return 2 }\n";
        let after_src = "fn a(x: string) -> int { return 1 }\nfn c() -> int { return 3 }\n";
        let from = document("m.aura", before_src, &parse(before_src).unwrap());
        let to = document("m.aura", after_src, &parse(after_src).unwrap());
        let d = delta(&from, &to);
        assert!(!d.is_empty());
        assert_eq!(d.from_revision, from.revision.clone().unwrap());
        let by_name: Vec<(&str, &str)> = d
            .symbols
            .iter()
            .map(|c| (c.name.as_str(), c.change.as_str()))
            .collect();
        assert!(by_name.contains(&("a", "changed")));
        assert!(by_name.contains(&("b", "removed")));
        assert!(by_name.contains(&("c", "added")));
    }

    #[test]
    fn delta_is_declaration_level_not_body_level() {
        // Honest protocol boundary: AIS describes declarations, so a
        // body-only edit is not a symbol change. The revision still changes,
        // so a consumer always sees *that* the source changed even when no
        // declaration did.
        let src1 = "fn a() -> int { return 1 }\n";
        let src2 = "fn a() -> int { return 2 }\n";
        let from = document("m.aura", src1, &parse(src1).unwrap());
        let to = document("m.aura", src2, &parse(src2).unwrap());
        assert_ne!(from.revision, to.revision);
        assert!(delta(&from, &to).is_empty());
    }

    #[test]
    fn delta_reports_an_unrelated_insertion_as_moved_not_changed() {
        // Inserting a comment above a declaration shifts every following
        // declaration's byte range. That is a position refresh, not a
        // semantic change: the delta must not tell a consumer that the whole
        // universe changed, and `is_empty` must stay true.
        let src1 = "fn a() -> int { return 1 }\nfn b() -> int { return 2 }\n";
        let src2 = "# a note\nfn a() -> int { return 1 }\nfn b() -> int { return 2 }\n";
        let from = document("m.aura", src1, &parse(src1).unwrap());
        let to = document("m.aura", src2, &parse(src2).unwrap());
        let d = delta(&from, &to);
        assert!(d.is_empty(), "a comment insertion is not a semantic change");
        assert!(d.has_moves(), "positions moved and the delta says so");
        assert!(d.symbols.iter().all(|c| c.change == "moved"));
        // The moved metadata still carries the current position.
        let a = d.symbols.iter().find(|c| c.name == "a").unwrap();
        assert_eq!(a.after.as_ref().unwrap().range.start.line, 2);
    }

    #[test]
    fn delta_still_reports_a_real_signature_change_as_changed() {
        let src1 = "fn a(x: int) -> int { return x }\n";
        let src2 = "fn a(x: string) -> int { return 1 }\n";
        let from = document("m.aura", src1, &parse(src1).unwrap());
        let to = document("m.aura", src2, &parse(src2).unwrap());
        let d = delta(&from, &to);
        assert!(!d.is_empty());
        assert_eq!(d.symbols[0].change, "changed");
    }

    #[test]
    fn a_delta_between_identical_revisions_is_empty() {
        let src = "fn a() -> int { return 1 }\n";
        let from = document("m.aura", src, &parse(src).unwrap());
        let to = document("m.aura", src, &parse(src).unwrap());
        let d = delta(&from, &to);
        assert!(d.is_empty());
        assert_eq!(d.from_revision, d.to_revision);
    }

    #[test]
    fn a_slice_is_target_focused_and_budgeted() {
        // `use_p` depends on `P`; the slice for `use_p` carries both, but not
        // the unrelated `Unrelated`.
        let src = "struct P { x: int }\nstruct Unrelated { y: int }\nfn use_p(p: P) -> int { return p.x }\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        let s = slice(&doc, "use_p", 2, 32);
        assert_eq!(s.target, "use_p");
        assert_eq!(s.symbol.as_ref().unwrap().name, "use_p");
        let deps: Vec<&str> = s.dependencies.iter().map(|d| d.name.as_str()).collect();
        assert!(deps.contains(&"P"), "direct dependency must be included");
        assert!(
            !deps.contains(&"Unrelated"),
            "an unrelated symbol must not be dragged in"
        );
        assert_eq!(s.omitted, 0);
    }

    #[test]
    fn a_slice_budget_reports_the_omission_rather_than_lying() {
        let src =
            "struct A { x: int }\nstruct B { y: int }\nfn f(a: A, b: B) -> int { return a.x }\n";
        let doc = document("m.aura", src, &parse(src).unwrap());
        let s = slice(&doc, "f", 1, 1);
        // Target only: the dependencies are omitted and counted, never hidden.
        assert!(s.dependencies.is_empty());
        assert_eq!(s.omitted, 2);
    }

    #[test]
    fn a_slice_for_an_unknown_target_is_empty_not_an_error() {
        let src = "fn a() -> int { return 1 }\n";
        let doc = document("m.aura", src, &parse(src).unwrap());
        let s = slice(&doc, "nope", 1, 8);
        assert!(s.symbol.is_none());
        assert!(s.dependencies.is_empty());
    }

    #[test]
    fn slice_and_delta_serialize_with_the_protocol_version() {
        let src = "fn a() -> int { return 1 }\n";
        let doc = document("m.aura", src, &parse(src).unwrap());
        let s = slice(&doc, "a", 1, 8);
        let json = serde_json::to_string(&s).expect("serializes");
        assert!(json.contains("\"ais_version\":\"0.1\""));
        let d = delta(&doc, &doc);
        let json = serde_json::to_string(&d).expect("serializes");
        assert!(json.contains("\"ais_version\":\"0.1\""));
    }

    #[test]
    fn a_checked_document_reports_the_checkers_narrowing_proof() {
        // AIS must never force a consumer to redo flow analysis: after a
        // `!= none` guard the checker proves the binding is `string`, and the
        // document carries exactly that conclusion.
        let src = "fn find(ok: bool) -> string | none {\n  if ok { return \"x\" }\n  return none\n}\nfn main() {\n  let e = find(true)\n  if e != none { print(e) }\n}\n";
        let module = parse(src).expect("parses");
        let doc = checked_document("m.aura", src, &module);
        assert!(doc.diagnostics.is_empty(), "{:?}", doc.diagnostics);
        let fact = doc
            .narrowings
            .iter()
            .find(|f| f.name == "e")
            .expect("the guard proves `e` is not none");
        assert_eq!(fact.narrowed_type.as_deref(), Some("string"));
        assert!(!fact.diverges);
    }

    #[test]
    fn a_checked_document_without_a_guard_reports_no_narrowing() {
        // No guard, no proof: the document must not invent a narrowing.
        let src = "fn find(ok: bool) -> string | none {\n  if ok { return \"x\" }\n  return none\n}\nfn main() {\n  let e = find(true)\n  print(e)\n}\n";
        let module = parse(src).expect("parses");
        let (outcome, facts) = crate::check::Checker::module_narrowings(&module);
        assert!(outcome.is_ok());
        assert!(facts.iter().all(|(n, _)| n != "e"));
    }

    #[test]
    fn a_plain_document_carries_no_narrowings() {
        // The unchecked snapshot constructor never reports flow facts; only
        // `checked_document` (which runs the checker) does.
        let src = "fn main() { print(1) }\n";
        let module = parse(src).expect("parses");
        let doc = document("m.aura", src, &module);
        assert!(doc.narrowings.is_empty());
    }
}
