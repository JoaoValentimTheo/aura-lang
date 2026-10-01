//! Provider-neutral source acquisition and logical module graph assembly.
//!
//! This layer owns only source identity, logical ownership, deterministic
//! traversal, and lowering of external sources into the same recursive
//! [`crate::ast::Module`] shape produced by in-source `module` declarations.
//! Name resolution, imports, visibility, canonical names, checking, and
//! execution remain downstream responsibilities.

use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::sync::Arc;

use crate::ast::{Item, Module};
use crate::error::{codes, Diag, DiagnosticReport, SourceDiagnostic, Span};
use crate::source::{SourceId, SourceMap};

/// Provider-local identity of one source.
///
/// A `SourceKey` is opaque to Aura semantics. It identifies a record inside a
/// [`SourceProvider`]; it is neither a [`SourceId`] nor a logical module path.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceKey(Arc<str>);

impl SourceKey {
    /// Construct a provider-local key.
    #[must_use]
    pub fn new(value: impl Into<String>) -> SourceKey {
        SourceKey(Arc::<str>::from(value.into()))
    }

    /// Provider-local spelling of this key.
    ///
    /// The spelling is for provider integration and diagnostics only; it is
    /// never interpreted as an Aura module name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable metadata for a source known to a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDescriptor {
    key: SourceKey,
    name: Box<str>,
}

impl SourceDescriptor {
    /// Construct one source descriptor.
    #[must_use]
    pub fn new(key: SourceKey, name: impl Into<Box<str>>) -> SourceDescriptor {
        SourceDescriptor {
            key,
            name: name.into(),
        }
    }

    /// Provider-local source identity.
    #[must_use]
    pub const fn key(&self) -> &SourceKey {
        &self.key
    }

    /// Stable user-facing display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

/// One direct logical child candidate returned by a provider.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderChild {
    logical_name: String,
    source: SourceDescriptor,
}

impl ProviderChild {
    /// Construct a child candidate.
    #[must_use]
    pub fn new(logical_name: impl Into<String>, source: SourceDescriptor) -> ProviderChild {
        ProviderChild {
            logical_name: logical_name.into(),
            source,
        }
    }

    /// Aura logical child name supplied by the provider.
    #[must_use]
    pub fn logical_name(&self) -> &str {
        &self.logical_name
    }

    /// Provider source that claims this logical child.
    #[must_use]
    pub const fn source(&self) -> &SourceDescriptor {
        &self.source
    }
}

/// A provider-side acquisition failure.
///
/// Providers keep host-specific detail behind this boundary. The graph
/// builder maps the failure to Aura's stable module-source diagnostic class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProviderError {
    message: String,
    code: Option<u16>,
    span: Option<Span>,
    diagnostic_text: Option<Arc<str>>,
}

impl ProviderError {
    /// Construct a stable provider error message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> ProviderError {
        ProviderError {
            message: message.into(),
            code: None,
            span: None,
            diagnostic_text: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn coded(code: u16, message: impl Into<String>) -> ProviderError {
        ProviderError {
            message: message.into(),
            code: Some(code),
            span: None,
            diagnostic_text: None,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn invalid_utf8(
        message: impl Into<String>,
        span: Span,
        diagnostic_text: impl Into<Arc<str>>,
    ) -> ProviderError {
        ProviderError {
            message: message.into(),
            code: Some(codes::INVALID_CHAR),
            span: Some(span),
            diagnostic_text: Some(diagnostic_text.into()),
        }
    }

    /// Stable provider-supplied detail.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    const fn code(&self) -> Option<u16> {
        self.code
    }

    const fn span(&self) -> Option<Span> {
        self.span
    }

    fn diagnostic_text(&self) -> Option<Arc<str>> {
        self.diagnostic_text.as_ref().map(Arc::clone)
    }
}

impl fmt::Display for ProviderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ProviderError {}

/// Provider-neutral source acquisition contract.
///
/// Providers expose source identity/text and direct child ownership only.
/// They do not resolve Aura names, imports, visibility, types, or execution.
pub trait SourceProvider {
    /// Identify the selected entry source.
    ///
    /// `Ok(None)` means the provider has no such entry. Provider-specific
    /// lookup/path failures use `Err`.
    fn entry(&self) -> std::result::Result<Option<SourceDescriptor>, ProviderError>;

    /// Load UTF-8 source text for one provider-local key.
    ///
    /// `Ok(None)` distinguishes a missing source from a provider failure.
    fn load(&self, key: &SourceKey) -> std::result::Result<Option<Arc<str>>, ProviderError>;

    /// Enumerate direct logical child candidates for one owned source.
    ///
    /// Enumeration order is not semantic. [`ModuleGraphBuilder`] validates and
    /// sorts these candidates before any child is parsed.
    fn children(
        &self,
        parent: &SourceKey,
    ) -> std::result::Result<Vec<ProviderChild>, ProviderError>;
}

#[derive(Debug, Clone)]
struct MemorySource {
    name: Box<str>,
    text: Arc<str>,
}

/// Deterministic in-memory source provider.
///
/// This provider has no filesystem/path semantics and is suitable for tests or
/// a future browser/VFS adapter. Duplicate source keys are rejected instead of
/// silently overwriting an existing record. Child claims are retained exactly
/// as supplied so the graph builder can adjudicate logical ownership.
#[derive(Debug)]
pub struct InMemorySourceProvider {
    entry: SourceKey,
    sources: HashMap<SourceKey, MemorySource>,
    children: HashMap<SourceKey, Vec<(String, SourceKey)>>,
}

impl InMemorySourceProvider {
    /// Create an empty provider whose selected entry is `entry`.
    #[must_use]
    pub fn new(entry: SourceKey) -> InMemorySourceProvider {
        InMemorySourceProvider {
            entry,
            sources: HashMap::new(),
            children: HashMap::new(),
        }
    }

    /// Insert one provider source.
    ///
    /// # Errors
    /// Returns an error when `key` is already present.
    pub fn insert_source(
        &mut self,
        key: SourceKey,
        name: impl Into<Box<str>>,
        text: impl Into<Arc<str>>,
    ) -> std::result::Result<(), ProviderError> {
        if self.sources.contains_key(&key) {
            return Err(ProviderError::new(format!(
                "provider source key `{}` is already registered",
                key.as_str()
            )));
        }
        self.sources.insert(
            key,
            MemorySource {
                name: name.into(),
                text: text.into(),
            },
        );
        Ok(())
    }

    /// Add one direct logical child claim.
    ///
    /// Both endpoint sources must already exist. Multiple claims are preserved
    /// rather than overwritten; ownership conflicts are diagnosed by the graph
    /// builder in deterministic logical-name order.
    ///
    /// # Errors
    /// Returns an error when either provider key is unknown.
    pub fn add_child(
        &mut self,
        parent: &SourceKey,
        logical_name: impl Into<String>,
        child: &SourceKey,
    ) -> std::result::Result<(), ProviderError> {
        if !self.sources.contains_key(parent) {
            return Err(ProviderError::new(format!(
                "unknown parent source key `{}`",
                parent.as_str()
            )));
        }
        if !self.sources.contains_key(child) {
            return Err(ProviderError::new(format!(
                "unknown child source key `{}`",
                child.as_str()
            )));
        }
        self.children
            .entry(parent.clone())
            .or_default()
            .push((logical_name.into(), child.clone()));
        Ok(())
    }

    fn descriptor(&self, key: &SourceKey) -> Option<SourceDescriptor> {
        let source = self.sources.get(key)?;
        Some(SourceDescriptor::new(key.clone(), source.name.clone()))
    }
}

impl SourceProvider for InMemorySourceProvider {
    fn entry(&self) -> std::result::Result<Option<SourceDescriptor>, ProviderError> {
        Ok(self.descriptor(&self.entry))
    }

    fn load(&self, key: &SourceKey) -> std::result::Result<Option<Arc<str>>, ProviderError> {
        Ok(self.sources.get(key).map(|source| Arc::clone(&source.text)))
    }

    fn children(
        &self,
        parent: &SourceKey,
    ) -> std::result::Result<Vec<ProviderChild>, ProviderError> {
        let mut out = Vec::new();
        if let Some(children) = self.children.get(parent) {
            out.reserve(children.len());
            for (logical_name, key) in children {
                let Some(source) = self.descriptor(key) else {
                    return Err(ProviderError::new(format!(
                        "child source key `{}` is no longer registered",
                        key.as_str()
                    )));
                };
                out.push(ProviderChild::new(logical_name.clone(), source));
            }
        }
        Ok(out)
    }
}

/// Aura logical module identity.
///
/// The root is the empty path. Segments are Aura identifiers and are entirely
/// independent of provider keys or host paths.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LogicalModulePath(Vec<String>);

impl LogicalModulePath {
    /// The logical root module (`[]`).
    #[must_use]
    pub fn root() -> LogicalModulePath {
        LogicalModulePath(Vec::new())
    }

    /// Path segments in root-to-leaf order.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.0
    }

    fn child(&self, name: &str) -> LogicalModulePath {
        let mut segments = self.0.clone();
        segments.push(name.to_string());
        LogicalModulePath(segments)
    }

    fn display(&self) -> String {
        if self.0.is_empty() {
            "<root>".to_string()
        } else {
            self.0.join("::")
        }
    }
}

/// Provider/source ownership record for one logical module.
#[derive(Debug, Clone)]
pub struct ModuleGraphNode {
    path: LogicalModulePath,
    source_key: SourceKey,
    source: SourceId,
    parent: Option<usize>,
    children: Vec<usize>,
}

impl ModuleGraphNode {
    /// Logical module path owned by this node.
    #[must_use]
    pub const fn path(&self) -> &LogicalModulePath {
        &self.path
    }

    /// Provider-local source identity.
    #[must_use]
    pub const fn source_key(&self) -> &SourceKey {
        &self.source_key
    }

    /// Compilation-local source identity allocated in the graph's SourceMap.
    #[must_use]
    pub const fn source(&self) -> SourceId {
        self.source
    }

    /// Parent node index; the logical root has no parent.
    #[must_use]
    pub const fn parent(&self) -> Option<usize> {
        self.parent
    }

    /// Direct child node indices in deterministic logical-name order.
    #[must_use]
    pub fn children(&self) -> &[usize] {
        &self.children
    }
}

/// A provider-independent logical module graph and its lowered Aura AST.
#[derive(Debug)]
pub struct ModuleGraph {
    module: Module,
    sources: SourceMap,
    entry_source: SourceId,
    nodes: Vec<ModuleGraphNode>,
    provenance: Vec<ItemProvenance>,
}

/// Source ownership parallel to one recursive AST item.
///
/// This sidecar keeps `SourceId` out of `Span`, tokens, and AST nodes while
/// allowing the resolver to preserve per-item provenance through flattening.
#[derive(Debug, Clone)]
pub(crate) struct ItemProvenance {
    pub(crate) source: SourceId,
    pub(crate) children: Vec<ItemProvenance>,
}

impl ModuleGraph {
    /// Recursive logical module tree expected by [`crate::resolve::resolve`].
    #[must_use]
    pub const fn module(&self) -> &Module {
        &self.module
    }

    /// Authoritative source map allocated in deterministic traversal order.
    #[must_use]
    pub const fn sources(&self) -> &SourceMap {
        &self.sources
    }

    /// SourceId of the selected root source.
    #[must_use]
    pub const fn entry_source(&self) -> SourceId {
        self.entry_source
    }

    /// Graph nodes in deterministic depth-first discovery order.
    #[must_use]
    pub fn nodes(&self) -> &[ModuleGraphNode] {
        &self.nodes
    }

    /// Consume the graph and return its semantic input plus provenance data.
    #[must_use]
    pub fn into_parts(self) -> (Module, SourceMap, SourceId, Vec<ModuleGraphNode>) {
        (self.module, self.sources, self.entry_source, self.nodes)
    }

    pub(crate) fn into_compilation_parts(
        self,
    ) -> (
        Module,
        SourceMap,
        SourceId,
        Vec<ModuleGraphNode>,
        Vec<ItemProvenance>,
    ) {
        (
            self.module,
            self.sources,
            self.entry_source,
            self.nodes,
            self.provenance,
        )
    }
}

#[derive(Debug)]
struct BuildNode {
    path: LogicalModulePath,
    source_key: SourceKey,
    source: SourceId,
    parent: Option<usize>,
    children: Vec<usize>,
    parsed: Option<Module>,
    provenance: Option<Vec<ItemProvenance>>,
}

#[derive(Debug)]
struct BuildFrame {
    node: usize,
    children: Vec<ProviderChild>,
    next_child: usize,
}

/// Deterministic provider-neutral graph builder.
#[derive(Debug, Default)]
pub struct ModuleGraphBuilder;

impl ModuleGraphBuilder {
    /// Construct a graph builder.
    #[must_use]
    pub const fn new() -> ModuleGraphBuilder {
        ModuleGraphBuilder
    }

    /// Acquire, parse, validate, and lower the provider's reachable ownership
    /// tree into Aura's existing recursive module AST.
    ///
    /// Children are adjudicated and sorted before parsing, then processed
    /// depth-first. Physical wrappers are always public and are appended after
    /// all items written in the parent source.
    ///
    /// # Errors
    /// Returns the first deterministic provider, ownership, depth, or parser
    /// diagnostic together with the SourceMap accumulated to that point.
    pub fn build<P: SourceProvider>(
        &self,
        provider: &P,
    ) -> std::result::Result<ModuleGraph, DiagnosticReport> {
        let mut sources = SourceMap::new();

        let entry = match provider.entry() {
            Ok(Some(entry)) => entry,
            Ok(None) => {
                return Err(locationless_report(
                    codes::MODULE_SOURCE_PATH,
                    "source provider has no entry source",
                    sources,
                ));
            }
            Err(error) => {
                return Err(provider_report(
                    "cannot identify entry source",
                    error,
                    sources,
                ));
            }
        };
        let entry_text = match provider.load(entry.key()) {
            Ok(Some(text)) => text,
            Ok(None) => {
                return Err(locationless_report(
                    codes::MODULE_SOURCE_PATH,
                    format!("entry source `{}` is missing", entry.name()),
                    sources,
                ));
            }
            Err(error) => {
                return Err(load_report(
                    format!("cannot read entry source `{}`", entry.name()),
                    &entry,
                    error,
                    sources,
                ));
            }
        };

        let entry_source = sources.add(entry.name(), Arc::clone(&entry_text));
        let root_module = match crate::parse::parse_with_initial_depth(&entry_text, 0) {
            Ok(module) => module,
            Err(diagnostic) => {
                return Err(DiagnosticReport::new(
                    SourceDiagnostic::new(diagnostic, entry_source),
                    sources,
                ));
            }
        };

        let root_path = LogicalModulePath::root();
        let mut nodes = vec![BuildNode {
            path: root_path.clone(),
            source_key: entry.key().clone(),
            source: entry_source,
            parent: None,
            children: Vec::new(),
            provenance: Some(provenance_for_items(&root_module.items, entry_source)),
            parsed: Some(root_module),
        }];

        let mut source_owners: HashMap<SourceKey, LogicalModulePath> = HashMap::new();
        source_owners.insert(entry.key().clone(), root_path);

        let root_children = match prepare_children(provider, &nodes[0]) {
            Ok(children) => children,
            Err(diagnostic) => return Err(DiagnosticReport::new(diagnostic, sources)),
        };
        let mut stack = vec![BuildFrame {
            node: 0,
            children: root_children,
            next_child: 0,
        }];

        while let Some(frame) = stack.last_mut() {
            if frame.next_child == frame.children.len() {
                stack.pop();
                continue;
            }

            let child = frame.children[frame.next_child].clone();
            frame.next_child += 1;

            let parent_index = frame.node;
            let path = nodes[parent_index].path.child(child.logical_name());
            if let Some(existing_path) = source_owners.get(child.source().key()) {
                return Err(locationless_report(
                    codes::DUPLICATE_LOGICAL_SOURCE,
                    format!(
                        "provider source `{}` cannot own both logical modules `{}` and `{}`",
                        child.source().name(),
                        existing_path.display(),
                        path.display()
                    ),
                    sources,
                ));
            }

            let text = match provider.load(child.source().key()) {
                Ok(Some(text)) => text,
                Ok(None) => {
                    return Err(locationless_report(
                        codes::MODULE_SOURCE_PATH,
                        format!(
                            "provider child source `{}` for logical module `{}` is missing",
                            child.source().name(),
                            path.display()
                        ),
                        sources,
                    ));
                }
                Err(error) => {
                    return Err(load_report(
                        format!(
                            "cannot read source `{}` for logical module `{}`",
                            child.source().name(),
                            path.display()
                        ),
                        child.source(),
                        error,
                        sources,
                    ));
                }
            };

            let source = sources.add(child.source().name(), Arc::clone(&text));
            // A physical source path is equivalent to an in-source `module`
            // chain: one wrapper per logical segment. Module nesting counts
            // toward the semantic `MAX_AST_DEPTH` on every substrate
            // (ADR-0004), so a *pure* physical directory chain must be bounded
            // by the same limit — otherwise it bypasses the semantic ceiling
            // that in-source nesting respects (and would reintroduce the
            // native/WASM acceptance split for deep physical trees).
            let depth = path.segments().len();
            if depth > crate::parse::MAX_AST_DEPTH {
                return Err(DiagnosticReport::new(
                    SourceDiagnostic::new(
                        Diag::new(codes::NESTING, "module nests too deeply", Span::default()),
                        source,
                    ),
                    sources,
                ));
            }
            debug_assert!(depth <= crate::parse::parse_recursion_budget());

            let parsed = match crate::parse::parse_with_initial_depth(&text, depth) {
                Ok(module) => module,
                Err(diagnostic) => {
                    return Err(DiagnosticReport::new(
                        SourceDiagnostic::new(diagnostic, source),
                        sources,
                    ));
                }
            };

            let child_index = nodes.len();
            nodes[parent_index].children.push(child_index);
            nodes.push(BuildNode {
                path: path.clone(),
                source_key: child.source().key().clone(),
                source,
                parent: Some(parent_index),
                children: Vec::new(),
                provenance: Some(provenance_for_items(&parsed.items, source)),
                parsed: Some(parsed),
            });
            source_owners.insert(child.source().key().clone(), path);

            let children = match prepare_children(provider, &nodes[child_index]) {
                Ok(children) => children,
                Err(diagnostic) => return Err(DiagnosticReport::new(diagnostic, sources)),
            };
            stack.push(BuildFrame {
                node: child_index,
                children,
                next_child: 0,
            });
        }

        let mut lowered: Vec<Option<Item>> = (0..nodes.len()).map(|_| None).collect();
        let mut lowered_provenance: Vec<Option<ItemProvenance>> =
            (0..nodes.len()).map(|_| None).collect();
        let mut root = None;
        let mut root_provenance = None;
        for index in (0..nodes.len()).rev() {
            let Some(mut module) = nodes[index].parsed.take() else {
                return Err(locationless_report(
                    codes::INTERNAL,
                    "module graph node lost its parsed module during lowering",
                    sources,
                ));
            };
            let Some(mut provenance) = nodes[index].provenance.take() else {
                return Err(locationless_report(
                    codes::INTERNAL,
                    "module graph node lost its source provenance during lowering",
                    sources,
                ));
            };
            for &child_index in &nodes[index].children {
                let Some(child) = lowered[child_index].take() else {
                    return Err(locationless_report(
                        codes::INTERNAL,
                        "module graph child did not lower before its parent",
                        sources,
                    ));
                };
                module.items.push(child);
                let Some(child_provenance) = lowered_provenance[child_index].take() else {
                    return Err(locationless_report(
                        codes::INTERNAL,
                        "module graph child provenance did not lower before its parent",
                        sources,
                    ));
                };
                provenance.push(child_provenance);
            }
            if index == 0 {
                root = Some(module);
                root_provenance = Some(provenance);
            } else {
                let Some(name) = nodes[index].path.segments().last().cloned() else {
                    return Err(locationless_report(
                        codes::INTERNAL,
                        "non-root module graph node has an empty logical path",
                        sources,
                    ));
                };
                lowered[index] = Some(Item::Module {
                    name,
                    items: module.items,
                    public: true,
                    span: Span::default(),
                });
                lowered_provenance[index] = Some(ItemProvenance {
                    source: nodes[index].source,
                    children: provenance,
                });
            }
        }

        let nodes = nodes
            .into_iter()
            .map(|node| ModuleGraphNode {
                path: node.path,
                source_key: node.source_key,
                source: node.source,
                parent: node.parent,
                children: node.children,
            })
            .collect();

        let Some(module) = root else {
            return Err(locationless_report(
                codes::INTERNAL,
                "module graph lowering produced no root module",
                sources,
            ));
        };
        let Some(provenance) = root_provenance else {
            return Err(locationless_report(
                codes::INTERNAL,
                "module graph lowering produced no root provenance",
                sources,
            ));
        };

        Ok(ModuleGraph {
            module,
            sources,
            entry_source,
            nodes,
            provenance,
        })
    }
}

fn provenance_for_items(items: &[Item], source: SourceId) -> Vec<ItemProvenance> {
    items
        .iter()
        .map(|item| ItemProvenance {
            source,
            children: match item {
                Item::Module { items, .. } => provenance_for_items(items, source),
                _ => Vec::new(),
            },
        })
        .collect()
}

fn prepare_children<P: SourceProvider>(
    provider: &P,
    parent: &BuildNode,
) -> std::result::Result<Vec<ProviderChild>, SourceDiagnostic> {
    let mut children = provider.children(&parent.source_key).map_err(|error| {
        SourceDiagnostic::locationless(Diag::locationless(
            error.code().unwrap_or(codes::MODULE_SOURCE_PATH),
            format!(
                "cannot enumerate children of logical module `{}`: {}",
                parent.path.display(),
                error.message()
            ),
        ))
    })?;

    children.sort_by(|left, right| {
        left.logical_name()
            .as_bytes()
            .cmp(right.logical_name().as_bytes())
            .then_with(|| {
                left.source()
                    .name()
                    .as_bytes()
                    .cmp(right.source().name().as_bytes())
            })
            .then_with(|| {
                left.source()
                    .key()
                    .as_str()
                    .as_bytes()
                    .cmp(right.source().key().as_str().as_bytes())
            })
    });

    for child in &children {
        if !is_aura_module_name(child.logical_name()) {
            return Err(SourceDiagnostic::locationless(Diag::locationless(
                codes::MODULE_SOURCE_PATH,
                format!(
                    "provider child name `{}` is not a valid Aura module identifier",
                    child.logical_name()
                ),
            )));
        }
    }

    let mut first_by_folded_name = BTreeMap::<String, usize>::new();
    let mut first_case_collision = None;
    for (index, child) in children.iter().enumerate() {
        let folded_name = child.logical_name().to_ascii_lowercase();
        if let Some(&first_index) = first_by_folded_name.get(&folded_name) {
            if children[first_index].logical_name() != child.logical_name()
                && first_case_collision.is_none_or(|(best_first, _)| first_index < best_first)
            {
                first_case_collision = Some((first_index, index));
            }
        } else {
            first_by_folded_name.insert(folded_name, index);
        }
    }
    if let Some((left_index, right_index)) = first_case_collision {
        let left = &children[left_index];
        let right = &children[right_index];
        return Err(SourceDiagnostic::locationless(Diag::locationless(
            codes::MODULE_SOURCE_PATH,
            format!(
                "case-only module collision under logical module `{}`: {} and {}",
                parent.path.display(),
                left.logical_name(),
                right.logical_name()
            ),
        )));
    }

    let mut index = 0;
    while index < children.len() {
        let mut end = index + 1;
        while end < children.len() && children[end].logical_name() == children[index].logical_name()
        {
            end += 1;
        }
        if end - index > 1 {
            let logical_path = parent.path.child(children[index].logical_name());
            let first_key = children[index].source().key();
            let same_source = children[index + 1..end]
                .iter()
                .all(|child| child.source().key() == first_key);
            if same_source {
                return Err(SourceDiagnostic::locationless(Diag::locationless(
                    codes::DUPLICATE_LOGICAL_SOURCE,
                    format!(
                        "provider source `{}` is supplied more than once for logical module `{}`",
                        children[index].source().name(),
                        logical_path.display()
                    ),
                )));
            }
            let owners = children[index..end]
                .iter()
                .map(|child| child.source().name())
                .collect::<Vec<_>>()
                .join("`, `");
            return Err(SourceDiagnostic::locationless(Diag::locationless(
                codes::MODULE_SOURCE_OWNERSHIP,
                format!(
                    "logical module `{}` has multiple external owners: `{owners}`",
                    logical_path.display()
                ),
            )));
        }
        index = end;
    }

    let Some(parsed) = parent.parsed.as_ref() else {
        return Err(SourceDiagnostic::locationless(Diag::locationless(
            codes::INTERNAL,
            "child adjudication requires a parsed parent module",
        )));
    };
    for child in &children {
        if let Some(span) = parsed.items.iter().find_map(|item| match item {
            Item::Module { name, span, .. } if name == child.logical_name() => Some(*span),
            _ => None,
        }) {
            return Err(SourceDiagnostic::new(
                Diag::new(
                    codes::MODULE_SOURCE_OWNERSHIP,
                    format!(
                        "logical module `{}` is owned both in source and by external source `{}`",
                        parent.path.child(child.logical_name()).display(),
                        child.source().name()
                    ),
                    span,
                ),
                parent.source,
            ));
        }
    }

    Ok(children)
}

fn is_aura_module_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    if !bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_') {
        return false;
    }
    !crate::lex::KEYWORDS.contains(&name)
}

fn locationless_report(
    code: u16,
    message: impl Into<String>,
    sources: SourceMap,
) -> DiagnosticReport {
    DiagnosticReport::new(
        SourceDiagnostic::locationless(Diag::locationless(code, message)),
        sources,
    )
}

fn provider_report(
    context: impl AsRef<str>,
    error: ProviderError,
    sources: SourceMap,
) -> DiagnosticReport {
    locationless_report(
        error.code().unwrap_or(codes::MODULE_SOURCE_PATH),
        format!("{}: {}", context.as_ref(), error.message()),
        sources,
    )
}

fn load_report(
    context: impl AsRef<str>,
    source: &SourceDescriptor,
    error: ProviderError,
    mut sources: SourceMap,
) -> DiagnosticReport {
    if error.code() == Some(codes::INVALID_CHAR) {
        if let (Some(span), Some(text)) = (error.span(), error.diagnostic_text()) {
            let source_id = sources.add(source.name(), text);
            return DiagnosticReport::new(
                SourceDiagnostic::new(
                    Diag::new(codes::INVALID_CHAR, error.message(), span),
                    source_id,
                ),
                sources,
            );
        }
    }
    locationless_report(
        error.code().unwrap_or(codes::IO),
        format!("{}: {}", context.as_ref(), error.message()),
        sources,
    )
}
