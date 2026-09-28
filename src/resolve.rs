//! Module resolution and visibility (`LANGUAGE_SPEC.md` §28).
//!
//! Aura modules are **in-source**: `module Name { items }`, nestable, reached
//! by `::`-separated paths and `use` imports. They are the only module model
//! that preserves Native ↔ WASM parity, because the WebAssembly/Playground host
//! has no filesystem: a real module boundary cannot depend on files the guest
//! cannot see.
//!
//! This pass runs between parsing and checking. It flattens the module tree
//! into a single item list in which every declared name is **canonical** — the
//! item's name prefixed by its module path (`shapes::Point`) — and every
//! reference in the program is rewritten to the canonical name it denotes. The
//! checker and runtime therefore see a flat program keyed by canonical names
//! and stay module-agnostic; because both consume the same resolved tree, they
//! cannot disagree about which declaration a name reaches.
//!
//! Visibility is enforced here, at the boundary: an item declared without `pub`
//! is reachable only from within its own module or a descendant. A private
//! access is `E2018`; an unknown module or import is `E2019`.

use std::collections::{HashMap, HashSet};

use crate::ast::{
    Arm, Expr, FPart, FieldDecl, Item, Module, Param, Pattern, Stmt, TypeExpr, TypeParam,
    VariantDecl,
};
use crate::error::{codes, Diag, Result, Span};

/// The entry point: resolve `module` (the parsed file) into a flat module.
///
/// # Errors
/// Returns `E2019` for an unknown module or import target, `E2018` for a
/// private access across a module boundary, and `E2007` for a duplicate
/// declaration in one module.
pub fn resolve(module: Module) -> Result<Module> {
    let mut r = Resolver::new();
    r.collect_module(&module.items, &[])?;
    r.flatten(&module.items, &[])
}

/// A declaration retained by the REPL session. The name is canonical (a module
/// path prefix plus the local name); `module` is the owning module path; and
/// `public` records whether the declaration is visible outside that module.
///
/// The REPL's current module is always the **root**, so a later submission may
/// reach a session declaration only by its canonical path (or through `use`)
/// and only when it is `public`. Seeding the resolver with private entries
/// makes an access to them a real `E2018`, not an undefined name — no
/// REPL-only visibility semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionItem {
    /// Canonical name.
    pub name: String,
    /// Owning module path.
    pub module: Vec<String>,
    /// Whether it is exported from that module.
    pub public: bool,
}

/// A persistent import bound in the REPL's root module: `local` names
/// `canonical`. `use path as local` records the alias; a plain `use path`
/// records the target's own last segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionImport {
    /// The local name bound at the root.
    pub local: String,
    /// The canonical target the local name denotes.
    pub canonical: String,
}

/// The declarations and imports the REPL carries across submissions, in one
/// coherent session value. Nothing here is REPL-only: the resolver applies the
/// same scope and visibility rules it applies within one module.
#[derive(Debug, Clone, Default)]
pub struct Session {
    /// Retained declarations.
    pub items: Vec<SessionItem>,
    /// Retained root imports.
    pub imports: Vec<SessionImport>,
}

/// Resolve `module` against `session`, returning the resolved module and the
/// session additions it introduced (its own canonical declarations and root
/// imports). The caller applies the additions only after the submission is
/// accepted, so a failed submission cannot corrupt the session.
///
/// # Errors
/// As [`resolve`].
pub fn resolve_with_session(module: Module, session: &Session) -> Result<(Module, Session)> {
    let mut r = Resolver::new();
    r.seed_session(&session.items, &session.imports);
    r.collect_module(&module.items, &[])?;
    let imports = r.root_imports();
    let resolved = r.flatten(&module.items, &[])?;
    let additions = Session {
        items: session_items(&resolved),
        imports,
    };
    Ok((resolved, additions))
}

/// Resolve a single REPL statement (a `let`, an assignment, or an expression)
/// against the carried `session`, returning the statement with every free
/// reference rewritten to its canonical name and visibility enforced.
///
/// # Errors
/// As [`resolve`].
pub fn resolve_stmt(stmt: Stmt, session: &Session) -> Result<Stmt> {
    let module = Module {
        items: vec![Item::Fn {
            name: "@repl-stmt".to_string(),
            type_params: Vec::new(),
            params: Vec::new(),
            ret: None,
            ret_span: None,
            body: vec![stmt],
            public: false,
            span: Span::default(),
        }],
    };
    let mut r = Resolver::new();
    r.seed_session(&session.items, &session.imports);
    r.collect_module(&module.items, &[])?;
    let resolved = r.flatten(&module.items, &[])?;
    let Some(Item::Fn { body, .. }) = resolved.items.into_iter().next() else {
        return Err(Diag::new(
            codes::INTERNAL,
            "statement resolution produced no statement",
            Span::default(),
        ));
    };
    body.into_iter().next().ok_or_else(|| {
        Diag::new(
            codes::INTERNAL,
            "statement resolution produced no statement",
            Span::default(),
        )
    })
}

/// Extract the session-retained declarations from an already-resolved module.
/// Each item's name is canonical, so its owning module is every `::` segment
/// but the last.
#[must_use]
pub fn session_items(module: &Module) -> Vec<SessionItem> {
    let mut out = Vec::new();
    for item in &module.items {
        let (name, public) = match item {
            Item::Fn { name, public, .. }
            | Item::Const { name, public, .. }
            | Item::Struct { name, public, .. }
            | Item::Enum { name, public, .. }
            | Item::Alias { name, public, .. }
            | Item::Trait { name, public, .. } => (name, *public),
            _ => continue,
        };
        out.push(SessionItem {
            name: name.clone(),
            module: module_path_of(name),
            public,
        });
        // A variant's canonical name is its enum's module path plus the tag.
        if let Item::Enum { variants, .. } = item {
            let owner = module_path_of(name);
            for v in variants {
                out.push(SessionItem {
                    name: join(&owner, &v.tag),
                    module: owner.clone(),
                    public,
                });
            }
        }
    }
    out
}

/// The module path encoded in a canonical name: every `::` segment but the
/// last.
fn module_path_of(canonical: &str) -> Vec<String> {
    let mut segs: Vec<String> = canonical.split("::").map(str::to_string).collect();
    segs.pop();
    segs
}

/// What kind of item a canonical name denotes. Namespaces are separate, as in
/// Aura's flat model: a type and a value may share a name.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Namespace {
    Type,
    Value,
    Variant,
}

#[derive(Clone)]
struct ItemInfo {
    /// The module path the item was declared in (empty for the root).
    module: Vec<String>,
    /// Whether the item is exported from its module.
    public: bool,
    /// The item's namespace.
    namespace: Namespace,
}

struct Resolver {
    /// Every declared item, keyed by canonical name.
    items: HashMap<String, ItemInfo>,
    /// Per module path, its direct child modules: local name → child path.
    modules: HashMap<Vec<String>, HashMap<String, Vec<String>>>,
    /// Per module path, the visible names it may refer to, by namespace:
    /// local name → canonical name. Includes the module's own items and its
    /// imports, resolved during collection.
    type_scope: HashMap<Vec<String>, HashMap<String, String>>,
    value_scope: HashMap<Vec<String>, HashMap<String, String>>,
    variant_scope: HashMap<Vec<String>, HashMap<String, String>>,
    /// Canonical enum type name → its canonical variant names. Lets
    /// `Enum::Tag` select the variant of that exact enum, unambiguously and
    /// independent of declaration order — the flat `items` table cannot hold
    /// a type and a variant that share one canonical name at once.
    enum_variants: HashMap<String, HashSet<String>>,
    /// Module paths that exist, for validation.
    module_paths: HashSet<Vec<String>>,
    /// Per module path, `use path::to::module as Alias` bindings: alias → the
    /// aliased module's canonical path (§27). Lets `Alias::item` resolve.
    module_aliases: HashMap<Vec<String>, HashMap<String, Vec<String>>>,
    /// Public re-exports (`pub use path::Item`): the re-exporting module's
    /// canonical name → the real canonical target. Lets another module reach
    /// the item through the re-exporting module without privilege escalation.
    reexports: HashMap<String, String>,
    /// Canonical names of types that have their own value constructor (structs
    /// and aliases). A variant whose canonical name matches one of these is an
    /// unrepresentable collision (§39, CONF-RESOLVE-10); an enum's own name is
    /// not constructible, so `enum E { E }` stays legal.
    /// Every import applied, as `(module, local, canonical)`. The REPL persists
    /// the ones whose module is the root.
    applied_imports: Vec<(Vec<String>, String, String)>,
    /// The generic type parameters currently in scope, innermost last. A name
    /// here is a compile-time placeholder and must NOT be canonicalized to a
    /// declared type, even if one of the same name exists. Interior-mutable so
    /// the `&self` rewriters can scope it without threading a parameter.
    type_param_scope: std::cell::RefCell<Vec<HashSet<String>>>,
}

impl Resolver {
    fn new() -> Resolver {
        Resolver {
            items: HashMap::new(),
            modules: HashMap::new(),
            type_scope: HashMap::new(),
            value_scope: HashMap::new(),
            variant_scope: HashMap::new(),
            enum_variants: HashMap::new(),
            module_paths: HashSet::new(),
            module_aliases: HashMap::new(),
            reexports: HashMap::new(),
            applied_imports: Vec::new(),
            type_param_scope: std::cell::RefCell::new(Vec::new()),
        }
    }

    /// Whether `name` is a generic type parameter currently in scope.
    fn is_type_param(&self, name: &str) -> bool {
        self.type_param_scope
            .borrow()
            .iter()
            .rev()
            .any(|f| f.contains(name))
    }

    /// Push the type parameters a declaration introduces, after rejecting a
    /// parameter that shadows a declared type visible from `prefix`
    /// (`LANGUAGE_SPEC.md` §36.2, `E2007`). Bound parameters never
    /// canonicalize to a declared type.
    ///
    /// The checker's own shadow check only sees canonical root and
    /// module-local type names, so an imported type (`use m::T`) or one from a
    /// sibling/ancestor module would slip past it; the resolver has the full
    /// visibility model and is therefore the authority for this rule. This is
    /// the same rule the checker already enforces for a root type, extended to
    /// every visible type.
    fn push_type_params(&self, params: &[TypeParam], prefix: &[String]) -> Result<()> {
        for p in params {
            if self.lookup_path(&p.name, prefix, Namespace::Type).is_some() {
                return Err(Diag::new(
                    codes::REDECLARED,
                    format!(
                        "type parameter `{}` shadows the declared type `{}`",
                        p.name, p.name
                    ),
                    p.span,
                ));
            }
        }
        let mut set: HashSet<String> = HashSet::new();
        for p in params {
            set.insert(p.name.clone());
        }
        self.type_param_scope.borrow_mut().push(set);
        Ok(())
    }

    fn pop_type_params(&self) {
        self.type_param_scope.borrow_mut().pop();
    }

    /// The root-module imports this submission introduced, so the REPL can
    /// retain them. Only imports visible at the root (the REPL's current
    /// module) are persisted.
    fn root_imports(&self) -> Vec<SessionImport> {
        self.applied_imports
            .iter()
            .filter(|(module, _, _)| module.is_empty())
            .map(|(_, local, canonical)| SessionImport {
                local: local.clone(),
                canonical: canonical.clone(),
            })
            .collect()
    }

    /// Seed the resolver with declarations and imports carried from earlier
    /// REPL submissions, so a later submission resolves and visibility-checks
    /// them exactly as within one module.
    fn seed_session(&mut self, session: &[SessionItem], imports: &[SessionImport]) {
        for item in session {
            let local = item
                .name
                .rsplit("::")
                .next()
                .unwrap_or(&item.name)
                .to_string();
            // A module item and a type are recorded so both `path::name` and a
            // bare type name resolve. The namespace is inferred: a capitalized
            // local name that was declared via `struct`/`enum`/`alias`/`trait`
            // is a type; otherwise it is a value. Both maps are populated, so
            // either use resolves the same canonical name.
            self.value_scope
                .entry(item.module.clone())
                .or_default()
                .insert(local.clone(), item.name.clone());
            self.type_scope
                .entry(item.module.clone())
                .or_default()
                .insert(local, item.name.clone());
            self.items.insert(
                item.name.clone(),
                ItemInfo {
                    module: item.module.clone(),
                    public: item.public,
                    namespace: Namespace::Value,
                },
            );
            // Record the module path itself so `path::name` resolves even when
            // the intermediate module was introduced by an earlier submission.
            for end in 1..=item.module.len() {
                self.module_paths.insert(item.module[..end].to_vec());
            }
            // Seed the module hierarchy so a qualified path can be walked:
            // level `end` maps parent `module[..end]` to child `module[..=end]`.
            for end in 0..item.module.len() {
                let parent = item.module[..end].to_vec();
                let child = item.module[..=end].to_vec();
                let child_name = item.module[end].clone();
                self.modules
                    .entry(parent)
                    .or_default()
                    .entry(child_name)
                    .or_insert(child);
            }
        }
        // Retained imports become root bindings, exactly as if the `use` that
        // created them appeared at the root of this submission.
        for imp in imports {
            self.value_scope
                .entry(Vec::new())
                .or_default()
                .insert(imp.local.clone(), imp.canonical.clone());
            self.type_scope
                .entry(Vec::new())
                .or_default()
                .insert(imp.local.clone(), imp.canonical.clone());
        }
    }

    // ------------------------------------------------------------- collection

    /// Record every declaration under `prefix`, building the per-module scope
    /// maps. Two passes are needed because a module may refer to an item
    /// declared later in the same module.
    fn collect_module(&mut self, items: &[Item], prefix: &[String]) -> Result<()> {
        self.declare_module(items, prefix)?;
        // Apply imports only after **every** module and item in the file has
        // been declared, so an import may name an item declared later or in a
        // sibling module regardless of source order.
        self.apply_imports(items, prefix)
    }

    /// Pass A: declare this module's direct items and recurse into children.
    fn declare_module(&mut self, items: &[Item], prefix: &[String]) -> Result<()> {
        self.module_paths.insert(prefix.to_vec());
        for item in items {
            match item {
                Item::Fn { name, public, .. } => {
                    self.declare(prefix, name, *public, Namespace::Value, item_span(item));
                }
                Item::Const {
                    name, public, span, ..
                } => {
                    self.declare(prefix, name, *public, Namespace::Value, *span);
                }
                Item::Struct {
                    name, public, span, ..
                } => {
                    self.declare(prefix, name, *public, Namespace::Type, *span);
                }
                Item::Enum {
                    name,
                    variants,
                    public,
                    span,
                    ..
                } => {
                    self.declare(prefix, name, *public, Namespace::Type, *span);
                    // Variant tags live in a per-module namespace; the checker
                    // remains the single authority for duplicate detection.
                    let enum_canonical = join(prefix, name);
                    for v in variants {
                        let tag = join(prefix, &v.tag);
                        self.variant_scope
                            .entry(prefix.to_vec())
                            .or_default()
                            .insert(v.tag.clone(), tag.clone());
                        self.enum_variants
                            .entry(enum_canonical.clone())
                            .or_default()
                            .insert(tag.clone());
                        self.items
                            .entry(tag)
                            .and_modify(|e| e.public |= *public)
                            .or_insert(ItemInfo {
                                module: prefix.to_vec(),
                                public: *public,
                                namespace: Namespace::Variant,
                            });
                    }
                }
                Item::Alias {
                    name, public, span, ..
                } => {
                    self.declare(prefix, name, *public, Namespace::Type, *span);
                }
                Item::Trait {
                    name, public, span, ..
                } => {
                    self.declare(prefix, name, *public, Namespace::Type, *span);
                }
                Item::Impl { .. } | Item::Expr(..) => {}
                Item::Use { .. } => {}
                Item::Module {
                    name,
                    items: inner,
                    span,
                    public,
                } => {
                    let mut child = prefix.to_vec();
                    child.push(name.clone());
                    // Record the module itself so `pub module` is a real
                    // visibility boundary (§27, CONF-RESOLVE-7): reaching a
                    // nested module's items from outside its parent requires
                    // the crossed module to be `pub`.
                    self.items
                        .entry(child.join("::"))
                        .and_modify(|e| e.public |= *public)
                        .or_insert(ItemInfo {
                            module: prefix.to_vec(),
                            public: *public,
                            namespace: Namespace::Type,
                        });
                    if self
                        .modules
                        .entry(prefix.to_vec())
                        .or_default()
                        .insert(name.clone(), child.clone())
                        .is_some()
                    {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("module `{name}` is declared more than once"),
                            *span,
                        ));
                    }
                    self.module_paths.insert(child.clone());
                    self.declare_module(inner, &child)?;
                }
            }
        }
        Ok(())
    }

    /// Pass B: apply `use` imports from every module, now that the whole
    /// program's declarations are known.
    fn apply_imports(&mut self, items: &[Item], prefix: &[String]) -> Result<()> {
        for item in items {
            match item {
                Item::Use {
                    path,
                    alias,
                    public,
                    span,
                } => self.apply_use(path, alias.as_deref(), *public, prefix, *span)?,
                Item::Module {
                    items: inner, name, ..
                } => {
                    let mut child = prefix.to_vec();
                    child.push(name.clone());
                    self.apply_imports(inner, &child)?;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn apply_use(
        &mut self,
        path: &[String],
        alias: Option<&str>,
        public: bool,
        prefix: &[String],
        span: Span,
    ) -> Result<()> {
        // `use path::to::module` (optionally `as Alias`) binds a module alias
        // (§27, CONF-RESOLVE-9). The alias is lexical and resolution-only: it
        // is not a runtime value, and it is collision-checked like any import.
        let module_target = self.resolve_module_path(prefix, path);
        if let Some(target) = module_target {
            let local =
                alias.map_or_else(|| path.last().cloned().unwrap_or_default(), str::to_string);
            self.check_module_visible(&target, prefix, span)?;
            if self.name_visible_locally(prefix, &local) {
                return Err(Diag::new(
                    codes::REDECLARED,
                    format!(
                        "`{local}` is already declared in this module; rename the import with `as`"
                    ),
                    span,
                ));
            }
            self.module_aliases
                .entry(prefix.to_vec())
                .or_default()
                .insert(local, target);
            return Ok(());
        }
        let canonical = self.resolve_use_path(prefix, path, span)?;
        if !self.items.contains_key(&canonical) {
            return Err(Diag::new(
                codes::UNKNOWN_MODULE,
                format!(
                    "`{}` is not a declared item or module; `use` must name an item or a module",
                    path.join("::")
                ),
                span,
            ));
        }
        let namespace = self.items.get(&canonical).map(|i| i.namespace);
        let local = alias.map_or_else(|| path.last().cloned().unwrap_or_default(), str::to_string);
        // Importing across a module boundary respects visibility: a private
        // target is `E2018`, exactly as a direct path would be.
        self.check_visible(&canonical, prefix, span)?;
        // An import must not silently shadow a name already visible in this
        // module (a local declaration or an earlier import): that is `E2007`.
        if self.name_visible_locally(prefix, &local) {
            return Err(Diag::new(
                codes::REDECLARED,
                format!(
                    "`{local}` is already declared in this module; rename the import with `as`"
                ),
                span,
            ));
        }
        // An imported type is also usable as a value (a struct name in a
        // constructor or a bare variant reference), so bind it in both
        // namespaces when it is a type.
        if namespace == Some(Namespace::Type) {
            self.type_scope
                .entry(prefix.to_vec())
                .or_default()
                .insert(local.clone(), canonical.clone());
            self.value_scope
                .entry(prefix.to_vec())
                .or_default()
                .insert(local.clone(), canonical.clone());
        } else {
            let scope = match namespace {
                Some(Namespace::Variant) => self.variant_scope.entry(prefix.to_vec()).or_default(),
                _ => self.value_scope.entry(prefix.to_vec()).or_default(),
            };
            scope.insert(local.clone(), canonical.clone());
        }
        self.applied_imports
            .push((prefix.to_vec(), local.clone(), canonical.clone()));
        // `pub use path::Item` re-exports: the re-exporting module gains an
        // exported name `prefix::local` that denotes `canonical` (§27,
        // CONF-RESOLVE-8). No privilege escalation: the target must itself be
        // visible here (checked above), and a private declaration cannot be
        // published through a public re-export.
        if public {
            if !self.is_publicly_visible(&canonical) {
                return Err(Diag::new(
                    codes::PRIVATE_ACCESS,
                    format!(
                        "`{}` is private where it is declared; a `pub use` cannot re-export it",
                        path.join("::")
                    ),
                    span,
                ));
            }
            self.reexports
                .insert(join(prefix, &local), canonical.clone());
            // Mark the re-exported public name as an exported item.
            if let Some(info) = self.items.get(&canonical).cloned() {
                self.items.insert(
                    join(prefix, &local),
                    ItemInfo {
                        module: prefix.to_vec(),
                        public: true,
                        namespace: info.namespace,
                    },
                );
            }
        }
        Ok(())
    }

    /// Whether `canonical` is exported (`pub`) from its own declaring module.
    fn is_publicly_visible(&self, canonical: &str) -> bool {
        self.items.get(canonical).is_some_and(|i| i.public)
    }

    /// Resolve a `use` path that names a *module*, returning its canonical path
    /// segments. Returns `None` when the path names an item instead.
    fn resolve_module_path(&self, prefix: &[String], path: &[String]) -> Option<Vec<String>> {
        let (first, rest) = path.split_first()?;
        // A module alias in scope may head the path.
        let mut base = self
            .lookup_module_alias(prefix, first)
            .or_else(|| self.lookup_child_module(prefix, first))?;
        for seg in rest {
            base = self.modules.get(&base)?.get(seg).cloned()?;
        }
        Some(base)
    }

    /// The child module named `first` visible from `prefix`, from the innermost
    /// scope outward.
    fn lookup_child_module(&self, prefix: &[String], first: &str) -> Option<Vec<String>> {
        for end in (0..=prefix.len()).rev() {
            if let Some(child) = self
                .modules
                .get(&prefix[..end])
                .and_then(|m| m.get(first))
                .cloned()
            {
                return Some(child);
            }
        }
        None
    }

    /// The module path an alias in scope denotes, from the innermost scope out.
    fn lookup_module_alias(&self, prefix: &[String], alias: &str) -> Option<Vec<String>> {
        for end in (0..=prefix.len()).rev() {
            if let Some(target) = self
                .module_aliases
                .get(&prefix[..end])
                .and_then(|m| m.get(alias))
                .cloned()
            {
                return Some(target);
            }
        }
        None
    }

    /// A module is visible from `prefix` when it is a descendant of the
    /// referring module, when its parent is visible, or when it was explicitly
    /// reachable. Aura has only `private` and `pub` (§27), so a module reached
    /// across a boundary must be `pub`.
    fn check_module_visible(&self, target: &[String], prefix: &[String], span: Span) -> Result<()> {
        let Some(info) = self.items.get(&target.join("::")) else {
            // A module with no recorded item: fall back to the item walk.
            return Ok(());
        };
        // The parent that declares the module, and any descendant of that
        // parent, may name it without `pub`; a more distant ancestor must not,
        // so `pub module` is meaningful (CONF-RESOLVE-7).
        if info.public || is_descendant(prefix, &info.module) {
            return Ok(());
        }
        Err(Diag::new(
            codes::PRIVATE_ACCESS,
            format!(
                "module `{}` is private; declare it `pub` to reach it from here",
                target.join("::")
            ),
            span,
        ))
    }

    /// Whether `local` is already declared (or imported) in this module,
    /// excluding any name a parent module exposes.
    fn name_visible_locally(&self, prefix: &[String], local: &str) -> bool {
        let key = prefix.to_vec();
        [
            self.type_scope.get(&key),
            self.value_scope.get(&key),
            self.variant_scope.get(&key),
        ]
        .into_iter()
        .flatten()
        .any(|m| m.contains_key(local))
            || self
                .module_aliases
                .get(&key)
                .is_some_and(|m| m.contains_key(local))
    }

    /// Record a declaration in a module's scope. Duplicate detection is
    /// deliberately **not** done here: the checker is the single authority for
    /// `E2007`/`E2012`/`E2013`, and it must see overloaded functions and
    /// methods as one name. The resolver only maps local names to canonical
    /// names, so a repeated declaration overwrites the same canonical entry.
    fn declare(
        &mut self,
        prefix: &[String],
        name: &str,
        public: bool,
        namespace: Namespace,
        _span: Span,
    ) {
        let canonical = join(prefix, name);
        let scope = match namespace {
            Namespace::Type => self.type_scope.entry(prefix.to_vec()).or_default(),
            Namespace::Value => self.value_scope.entry(prefix.to_vec()).or_default(),
            Namespace::Variant => self.variant_scope.entry(prefix.to_vec()).or_default(),
        };
        scope.insert(name.to_string(), canonical.clone());
        // A name declared `pub` anywhere in a module is exported; keep the
        // `public` flag true if any declaration of that name is public.
        self.items
            .entry(canonical)
            .and_modify(|e| e.public |= public)
            .or_insert(ItemInfo {
                module: prefix.to_vec(),
                public,
                namespace,
            });
    }

    /// Resolve a `use` path from `prefix`, then reject a canonical name that
    /// denotes both a declared type and an enum variant. Such a name cannot be
    /// represented by the flat item table, so whichever spelling reaches it
    /// (`use m::A`, `use m::E::A`), an import could otherwise silently bind the
    /// struct where the source named the variant.
    fn resolve_use_path(&self, prefix: &[String], path: &[String], span: Span) -> Result<String> {
        let canonical = self.resolve_use_path_inner(prefix, path, span)?;
        if self.is_declared_type(&canonical) && self.is_variant(&canonical) {
            return Err(Diag::new(
                codes::UNKNOWN_TYPE,
                format!(
                    "`{}` is ambiguous: the variant `{canonical}` shares its name with a declared type; rename one of them",
                    path.join("::")
                ),
                span,
            ));
        }
        Ok(canonical)
    }

    /// Follow a public re-export chain to the real canonical target, with a
    /// bounded walk so a cycle is a deterministic failure rather than
    /// unbounded recursion (§27, CONF-RESOLVE-8).
    fn follow_reexport(&self, canonical: &str) -> String {
        let mut current = canonical.to_string();
        for _ in 0..self.reexports.len().saturating_add(1) {
            match self.reexports.get(&current) {
                Some(next) if *next != current => current = next.clone(),
                _ => break,
            }
        }
        current
    }

    /// Whether `canonical` is a declared enum-variant tag.
    fn is_variant(&self, canonical: &str) -> bool {
        self.variant_scope
            .values()
            .any(|m| m.values().any(|c| c == canonical))
    }

    /// Resolve a `use` path from `prefix`. The first segment is looked up in the
    /// enclosing module scopes from the innermost outward; the remaining
    /// segments navigate nested modules. Returns the canonical name of the
    /// imported item.
    fn resolve_use_path_inner(
        &self,
        prefix: &[String],
        path: &[String],
        span: Span,
    ) -> Result<String> {
        let Some((first, rest)) = path.split_first() else {
            return Err(Diag::new(codes::UNKNOWN_MODULE, "empty `use` path", span));
        };
        // `use Enum::Variant` / `use module::Enum::Variant` — the documented
        // enum-path spelling (§27). The segment before the tag names an enum
        // type, not a module, so the module walk below cannot reach it; resolve
        // it against the enum's own variant set, exactly like a construct
        // reference.
        if path.len() >= 2 {
            let enum_path = path[..path.len() - 1].join("::");
            let tag = path[path.len() - 1].as_str();
            if let Some(enum_canonical) = self.lookup_path(&enum_path, prefix, Namespace::Type) {
                let canonical = match enum_canonical.rsplit_once("::") {
                    Some((module, _)) => format!("{module}::{tag}"),
                    None => tag.to_string(),
                };
                if self
                    .enum_variants
                    .get(&enum_canonical)
                    .is_some_and(|tags| tags.contains(&canonical))
                {
                    self.check_visible(&enum_canonical, prefix, span)?;
                    return Ok(canonical);
                }
            }
        }
        // Walk outward from the current module to the root, looking for the
        // first segment as a module alias, a child module, or a visible item.
        let mut base: Option<Vec<String>> = self.lookup_module_alias(prefix, first);
        for end in (0..=prefix.len()).rev() {
            let scope = prefix[..end].to_vec();
            if base.is_none() {
                if let Some(child) = self.modules.get(&scope).and_then(|m| m.get(first)).cloned() {
                    base = Some(child);
                    break;
                }
            }
            if rest.is_empty() {
                for map in [
                    self.type_scope.get(&scope),
                    self.value_scope.get(&scope),
                    self.variant_scope.get(&scope),
                ]
                .into_iter()
                .flatten()
                {
                    if let Some(c) = map.get(first) {
                        return Ok(self.follow_reexport(c));
                    }
                }
            }
        }
        let Some(mut base) = base else {
            return Err(Diag::new(
                codes::UNKNOWN_MODULE,
                format!("unknown module `{first}` in `use`"),
                span,
            ));
        };
        if rest.is_empty() {
            return Ok(base.join("::"));
        }
        // Navigate the remaining segments as modules, then the final item.
        for (i, seg) in rest.iter().enumerate() {
            let last = i == rest.len() - 1;
            if last {
                let canonical = join(&base, seg);
                if self.items.contains_key(&canonical)
                    || self.module_paths.contains(&canonical_path(&base, seg))
                {
                    return Ok(canonical);
                }
                return Err(Diag::new(
                    codes::UNKNOWN_MODULE,
                    format!("`{seg}` is not declared in module `{}`", base.join("::")),
                    span,
                ));
            }
            let Some(child) = self.modules.get(&base).and_then(|m| m.get(seg)).cloned() else {
                return Err(Diag::new(
                    codes::UNKNOWN_MODULE,
                    format!("`{seg}` is not a module in `{}`", base.join("::")),
                    span,
                ));
            };
            base = child;
        }
        Ok(base.join("::"))
    }

    // ---------------------------------------------------------------- rewrite

    /// Flatten the module tree: emit each item with its canonical target names,
    /// dropping module wrappers and `use` imports (already applied).
    fn flatten(&mut self, items: &[Item], prefix: &[String]) -> Result<Module> {
        let mut out = Vec::new();
        for item in items {
            match item {
                Item::Module { items: inner, .. } => {
                    // The child module's canonical path must be reconstructed
                    // from the item, since `prefix` here is the parent.
                    let name = match item {
                        Item::Module { name, .. } => name,
                        _ => unreachable!(),
                    };
                    let mut child = prefix.to_vec();
                    child.push(name.clone());
                    let mut sub = self.flatten(inner, &child)?;
                    out.append(&mut sub.items);
                }
                Item::Use { .. } => {}
                other => out.push(self.rewrite_item(other, prefix)?),
            }
        }
        Ok(Module { items: out })
    }

    fn rewrite_item(&self, item: &Item, prefix: &[String]) -> Result<Item> {
        Ok(match item {
            Item::Fn {
                name,
                type_params,
                params,
                ret,
                ret_span,
                body,
                public,
                span,
            } => {
                self.push_type_params(type_params, prefix)?;
                let mut locals = Locals::default();
                let params = self.rewrite_params(params, &mut locals, prefix)?;
                let ret = ret
                    .as_ref()
                    .map(|t| self.rewrite_type(t, prefix, ret_span.unwrap_or(*span)))
                    .transpose()?;
                let mut body = body.clone();
                self.rewrite_block(&mut body, &mut locals, prefix)?;
                self.pop_type_params();
                Item::Fn {
                    name: join(prefix, name),
                    type_params: type_params.clone(),
                    params,
                    ret,
                    ret_span: *ret_span,
                    body,
                    public: *public,
                    span: *span,
                }
            }
            Item::Const {
                name,
                ann,
                value,
                public,
                span,
            } => {
                let ann = ann
                    .as_ref()
                    .map(|t| self.rewrite_type(t, prefix, *span))
                    .transpose()?;
                let value = self.rewrite_expr(value, &mut Locals::default(), prefix)?;
                Item::Const {
                    name: join(prefix, name),
                    ann,
                    value,
                    public: *public,
                    span: *span,
                }
            }
            Item::Expr(e, span) => {
                Item::Expr(self.rewrite_expr(e, &mut Locals::default(), prefix)?, *span)
            }
            Item::Struct {
                name,
                type_params,
                fields,
                public,
                span,
            } => {
                self.push_type_params(type_params, prefix)?;
                let fields = fields
                    .iter()
                    .map(|f| self.rewrite_field(f, prefix))
                    .collect::<Result<Vec<_>>>()?;
                self.pop_type_params();
                Item::Struct {
                    name: join(prefix, name),
                    type_params: type_params.clone(),
                    fields,
                    public: *public,
                    span: *span,
                }
            }
            Item::Enum {
                name,
                type_params,
                variants,
                public,
                span,
            } => {
                self.push_type_params(type_params, prefix)?;
                let variants = variants
                    .iter()
                    .map(|v| self.rewrite_variant(v, prefix))
                    .collect::<Result<Vec<_>>>()?;
                self.pop_type_params();
                Item::Enum {
                    name: join(prefix, name),
                    type_params: type_params.clone(),
                    variants,
                    public: *public,
                    span: *span,
                }
            }
            Item::Alias {
                name,
                type_params,
                target,
                public,
                span,
            } => {
                self.push_type_params(type_params, prefix)?;
                let target = self.rewrite_type(target, prefix, *span)?;
                self.pop_type_params();
                Item::Alias {
                    name: join(prefix, name),
                    type_params: type_params.clone(),
                    target,
                    public: *public,
                    span: *span,
                }
            }
            Item::Trait {
                name,
                type_params,
                methods,
                public,
                span,
                ..
            } => {
                self.push_type_params(type_params, prefix)?;
                let methods = methods
                    .iter()
                    .map(|m| self.rewrite_member(m, prefix))
                    .collect::<Result<Vec<_>>>()?;
                self.pop_type_params();
                Item::Trait {
                    name: join(prefix, name),
                    type_params: type_params.clone(),
                    methods,
                    public: *public,
                    owner: prefix.to_vec(),
                    span: *span,
                }
            }
            Item::Impl {
                target,
                target_args,
                trait_name,
                trait_args,
                type_params,
                methods,
                span,
                ..
            } => {
                self.push_type_params(type_params, prefix)?;
                let target = self.canonical_type(target, prefix, *span)?;
                let target_args = target_args
                    .iter()
                    .map(|t| self.rewrite_type(t, prefix, *span))
                    .collect::<Result<Vec<_>>>()?;
                let trait_name = match trait_name {
                    Some(t) => Some(self.canonical_type(t, prefix, *span)?),
                    None => None,
                };
                let trait_args = trait_args
                    .iter()
                    .map(|t| self.rewrite_type(t, prefix, *span))
                    .collect::<Result<Vec<_>>>()?;
                let methods = methods
                    .iter()
                    .map(|m| self.rewrite_member(m, prefix))
                    .collect::<Result<Vec<_>>>()?;
                self.pop_type_params();
                Item::Impl {
                    target,
                    target_args,
                    trait_name,
                    trait_args,
                    type_params: type_params.clone(),
                    methods,
                    owner: prefix.to_vec(),
                    span: *span,
                }
            }
            Item::Module { .. } | Item::Use { .. } => {
                unreachable!("modules and imports are handled by flatten")
            }
        })
    }

    /// Rewrite a method inside an `impl` or `trait`. A method name is **not**
    /// path-qualified: methods are keyed by their nominal struct and share no
    /// global value namespace (`LANGUAGE_SPEC.md` §17.6). Only the parameter
    /// and return type annotations and the body are rewritten.
    fn rewrite_member(&self, item: &Item, prefix: &[String]) -> Result<Item> {
        let Item::Fn {
            name,
            type_params,
            params,
            ret,
            ret_span,
            body,
            public,
            span,
        } = item
        else {
            return self.rewrite_item(item, prefix);
        };
        self.push_type_params(type_params, prefix)?;
        let mut locals = Locals::default();
        let params = self.rewrite_params(params, &mut locals, prefix)?;
        let ret = ret
            .as_ref()
            .map(|t| self.rewrite_type(t, prefix, ret_span.unwrap_or(*span)))
            .transpose()?;
        let mut body = body.clone();
        self.rewrite_block(&mut body, &mut locals, prefix)?;
        self.pop_type_params();
        Ok(Item::Fn {
            name: name.clone(),
            type_params: type_params.clone(),
            params,
            ret,
            ret_span: *ret_span,
            body,
            public: *public,
            span: *span,
        })
    }

    fn rewrite_field(&self, f: &FieldDecl, prefix: &[String]) -> Result<FieldDecl> {
        Ok(FieldDecl {
            name: f.name.clone(),
            ty: self.rewrite_type(&f.ty, prefix, f.span)?,
            public: f.public,
            span: f.span,
        })
    }

    fn rewrite_variant(&self, v: &VariantDecl, prefix: &[String]) -> Result<VariantDecl> {
        // Variant tags are canonicalized so two modules may each declare a
        // variant with the same local name without collision (`E2013`).
        Ok(VariantDecl {
            tag: join(prefix, &v.tag),
            payload: v
                .payload
                .iter()
                .map(|t| self.rewrite_type(t, prefix, v.span))
                .collect::<Result<Vec<_>>>()?,
            span: v.span,
        })
    }

    /// Rewrite a written type. `span` is the span of the enclosing
    /// declaration syntax (a field, parameter, return annotation, alias target,
    /// …); `TypeExpr` carries no spans of its own, so a diagnostic about a name
    /// inside the type is reported at that enclosing span rather than at the
    /// file start.
    fn rewrite_type(&self, t: &TypeExpr, prefix: &[String], span: Span) -> Result<TypeExpr> {
        Ok(match t {
            TypeExpr::Named(n) => {
                // A bound type parameter is a placeholder, never a declared
                // type: leave it exactly as written.
                if self.is_type_param(n) {
                    TypeExpr::Named(n.clone())
                } else {
                    TypeExpr::Named(self.canonical_type(n, prefix, span)?)
                }
            }
            TypeExpr::App(n, args) => {
                let head = if self.is_type_param(n) {
                    n.clone()
                } else {
                    self.canonical_type(n, prefix, span)?
                };
                TypeExpr::App(
                    head,
                    args.iter()
                        .map(|a| self.rewrite_type(a, prefix, span))
                        .collect::<Result<Vec<_>>>()?,
                )
            }
            TypeExpr::List(i) => TypeExpr::List(Box::new(self.rewrite_type(i, prefix, span)?)),
            TypeExpr::Map(k, v) => TypeExpr::Map(
                Box::new(self.rewrite_type(k, prefix, span)?),
                Box::new(self.rewrite_type(v, prefix, span)?),
            ),
            TypeExpr::Union(ms) => TypeExpr::Union(
                ms.iter()
                    .map(|m| self.rewrite_type(m, prefix, span))
                    .collect::<Result<Vec<_>>>()?,
            ),
            other => other.clone(),
        })
    }

    /// Canonicalize a type-like name (a struct, enum, alias, or trait) and
    /// enforce visibility. A name that is not a known type is left as written
    /// so the checker reports the unknown type with its own diagnostic.
    fn canonical_type(&self, name: &str, prefix: &[String], span: Span) -> Result<String> {
        if let Some(c) = self.lookup_path(name, prefix, Namespace::Type) {
            self.check_visible(&c, prefix, span)?;
            return Ok(c);
        }
        Ok(name.to_string())
    }

    /// Canonicalize a construct/variant name (a struct, or an enum variant).
    ///
    /// A struct is named by its path (`shapes::Point`). An enum variant is
    /// named by its enum's path plus the tag (`shapes::Color::Red`), or by the
    /// module path plus the tag (`shapes::Red`), both of which name the same
    /// canonical variant `shapes::<tag>`.
    fn canonical_construct(&self, name: &str, prefix: &[String], span: Span) -> Result<String> {
        // `Enum::Tag` or `a::b::Enum::Tag` — the segments before the tag name
        // the enum type, bare, module-qualified, or imported (`use m::E`). A
        // variant's canonical name is the enum's *module* path plus the tag
        // (`LANGUAGE_SPEC.md` §27), so `E::A` at the root is `A` and
        // `m::E::A` is `m::A`. Membership is tested against the enum's own
        // variant set, so the answer never depends on declaration order.
        if let Some((enum_path, tag)) = name.rsplit_once("::") {
            if let Some(enum_canonical) = self.lookup_path(enum_path, prefix, Namespace::Type) {
                let canonical = match enum_canonical.rsplit_once("::") {
                    Some((module, _)) => format!("{module}::{tag}"),
                    None => tag.to_string(),
                };
                if self
                    .enum_variants
                    .get(&enum_canonical)
                    .is_some_and(|tags| tags.contains(&canonical))
                {
                    self.check_visible(&enum_canonical, prefix, span)?;
                    // A variant and a struct may share a canonical name
                    // (`module::tag`). The flat item table and the runtime both
                    // key by that name, so a construction cannot be
                    // disambiguated downstream — it would silently build the
                    // struct. Reject rather than resolve to the wrong value.
                    // (The genuinely ambiguous namespace question is recorded
                    // as a SPEC GAP in `docs/CONFORMANCE_PHASE2.md`.)
                    if self.is_declared_type(&canonical) {
                        return Err(Diag::new(
                            codes::UNKNOWN_TYPE,
                            format!(
                                "`{name}` is ambiguous: the variant `{canonical}` shares its name with a declared type; rename one of them"
                            ),
                            span,
                        ));
                    }
                    return Ok(canonical);
                }
            }
        }
        if let Some(c) = self.lookup_path(name, prefix, Namespace::Type) {
            self.check_visible(&c, prefix, span)?;
            return Ok(c);
        }
        if let Some(c) = self.lookup_path(name, prefix, Namespace::Variant) {
            self.check_visible(&c, prefix, span)?;
            return Ok(c);
        }
        Ok(name.to_string())
    }

    /// Whether `canonical` names a declared type (struct, enum, alias, or
    /// trait) in any module scope. Used to detect a type/variant canonical-name
    /// collision that the flat item table cannot represent.
    fn is_declared_type(&self, canonical: &str) -> bool {
        self.type_scope
            .values()
            .any(|m| m.values().any(|c| c == canonical))
    }

    /// Look up `name`, which may be a `::`-qualified path or a bare local name,
    /// in the module scopes from the innermost outward.
    fn lookup_path(&self, name: &str, prefix: &[String], ns: Namespace) -> Option<String> {
        if name.contains("::") {
            return self.lookup_qualified(name, prefix);
        }
        for end in (0..=prefix.len()).rev() {
            let scope = &prefix[..end];
            let map = match ns {
                Namespace::Type => self.type_scope.get(scope),
                Namespace::Value => self.value_scope.get(scope),
                Namespace::Variant => self.variant_scope.get(scope),
            };
            if let Some(c) = map.and_then(|m| m.get(name)) {
                return Some(c.clone());
            }
        }
        None
    }

    /// Resolve a `::`-qualified path: the first segment names a child module,
    /// the rest navigate to the item.
    fn lookup_qualified(&self, name: &str, prefix: &[String]) -> Option<String> {
        let segments: Vec<&str> = name.split("::").collect();
        let (first, rest) = segments.split_first()?;
        // The first segment may name a module alias in scope (§27), a child
        // module, or a re-exported type whose own members are then navigated.
        let mut base = self
            .lookup_module_alias(prefix, first)
            .or_else(|| self.lookup_child_module(prefix, first))?;
        for (i, seg) in rest.iter().enumerate() {
            if i == rest.len() - 1 {
                // Require the item to exist: a path to nothing must fall
                // through to the checker's `E2003`, never be rewritten to an
                // unchecked name. Follow a public re-export to its target.
                let canonical = join(&base, seg);
                if self.items.contains_key(&canonical) {
                    return Some(self.follow_reexport(&canonical));
                }
                return None;
            }
            base = self.modules.get(&base).and_then(|m| m.get(*seg)).cloned()?;
        }
        None
    }

    /// A canonical item is visible from `prefix` when it is exported (`pub`) or
    /// when the referring module is the defining module or a descendant.
    fn check_visible(&self, canonical: &str, prefix: &[String], span: Span) -> Result<()> {
        // Every module boundary crossed by the path must permit traversal
        // (§27, CONF-RESOLVE-7). A crossing is allowed when the referring
        // module is inside the module's declaring scope, or when the module is
        // `pub`.
        let segments: Vec<&str> = canonical.split("::").collect();
        for end in 1..segments.len() {
            let module = &segments[..end];
            let module_name = module.join("::");
            let Some(minfo) = self.items.get(&module_name) else {
                continue;
            };
            // Only module entries carry a nested path; a same-named type has no
            // children, so skip it (it is not a boundary here).
            let module_path: Vec<String> = module.iter().map(ToString::to_string).collect();
            if !self.module_paths.contains(&module_path) {
                continue;
            }
            // A module `M` declared directly in parent `P` is nameable by `P`
            // and by any descendant of `P` (so siblings inside the same parent
            // may traverse, exactly as in the existing model), while `M`'s own
            // items still obey their own `pub` rule. A more distant ancestor
            // (`P`'s ancestor) gets no such access, so `pub module` is
            // meaningful and an ancestor gains no access to a descendant's
            // private items (§27, CONF-RESOLVE-7).
            if minfo.public || is_descendant(prefix, &minfo.module) {
                continue;
            }
            return Err(Diag::new(
                codes::PRIVATE_ACCESS,
                format!(
                    "module `{module_name}` is private; declare it `pub` to reach it from here"
                ),
                span,
            ));
        }
        let Some(info) = self.items.get(canonical) else {
            return Ok(());
        };
        if info.public || is_descendant(prefix, &info.module) {
            return Ok(());
        }
        Err(Diag::new(
            codes::PRIVATE_ACCESS,
            format!(
                "`{}` is private to module `{}`; mark it `pub` to use it here",
                canonical.rsplit("::").next().unwrap_or(canonical),
                if info.module.is_empty() {
                    "the file root".to_string()
                } else {
                    info.module.join("::")
                }
            ),
            span,
        ))
    }

    // ------------------------------------------------------------ expressions

    fn rewrite_block(
        &self,
        body: &mut [Stmt],
        locals: &mut Locals,
        prefix: &[String],
    ) -> Result<()> {
        locals.push();
        for s in body.iter_mut() {
            self.rewrite_stmt(s, locals, prefix)?;
        }
        locals.pop();
        Ok(())
    }

    fn rewrite_stmt(&self, s: &mut Stmt, locals: &mut Locals, prefix: &[String]) -> Result<()> {
        match s {
            Stmt::Let {
                name,
                ann,
                value,
                span,
                ..
            } => {
                *value = self.rewrite_expr(value, locals, prefix)?;
                if let Some(a) = ann {
                    *a = self.rewrite_type(a, prefix, *span)?;
                }
                locals.declare(name);
            }
            Stmt::LetPattern { pattern, value, .. } => {
                *value = self.rewrite_expr(value, locals, prefix)?;
                self.collect_pattern_bindings(pattern, locals);
            }
            Stmt::Assign { target, value, .. } => {
                *target = self.rewrite_expr(target, locals, prefix)?;
                *value = self.rewrite_expr(value, locals, prefix)?;
            }
            Stmt::Expr(e, _) => *e = self.rewrite_expr(e, locals, prefix)?,
            Stmt::Return(Some(e), _) | Stmt::Throw(e, _) => {
                *e = self.rewrite_expr(e, locals, prefix)?;
            }
            Stmt::Return(None, _) | Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::While(c, body, _) => {
                *c = self.rewrite_expr(c, locals, prefix)?;
                self.rewrite_block(body, locals, prefix)?;
            }
            Stmt::Loop(body, _) => self.rewrite_block(body, locals, prefix)?,
            Stmt::For(pat, iter, body, _) => {
                *iter = self.rewrite_expr(iter, locals, prefix)?;
                locals.push();
                self.collect_pattern_bindings(pat, locals);
                for st in body.iter_mut() {
                    self.rewrite_stmt(st, locals, prefix)?;
                }
                locals.pop();
            }
            Stmt::Try {
                body,
                catch,
                catch_body,
                finally,
                ..
            } => {
                self.rewrite_block(body, locals, prefix)?;
                locals.push();
                locals.declare(catch);
                for st in catch_body.iter_mut() {
                    self.rewrite_stmt(st, locals, prefix)?;
                }
                locals.pop();
                if let Some(f) = finally {
                    self.rewrite_block(f, locals, prefix)?;
                }
            }
        }
        Ok(())
    }

    fn collect_pattern_bindings(&self, p: &Pattern, locals: &mut Locals) {
        match p {
            Pattern::Bind(n, _) => locals.declare(n),
            Pattern::List(ps, _) => {
                for x in ps {
                    self.collect_pattern_bindings(x, locals);
                }
            }
            Pattern::Variant(_, ps, _) => {
                for x in ps {
                    self.collect_pattern_bindings(x, locals);
                }
            }
            _ => {}
        }
    }

    fn rewrite_type_args(
        &self,
        args: &[TypeExpr],
        prefix: &[String],
        span: Span,
    ) -> Result<Vec<TypeExpr>> {
        args.iter()
            .map(|t| self.rewrite_type(t, prefix, span))
            .collect::<Result<Vec<_>>>()
    }

    fn rewrite_params(
        &self,
        params: &[Param],
        locals: &mut Locals,
        prefix: &[String],
    ) -> Result<Vec<Param>> {
        let mut out = Vec::with_capacity(params.len());
        for p in params {
            let ty =
                p.ty.as_ref()
                    .map(|t| self.rewrite_type(t, prefix, p.span))
                    .transpose()?;
            locals.declare(&p.name);
            out.push(Param {
                name: p.name.clone(),
                ty,
                mutable: p.mutable,
                span: p.span,
            });
        }
        Ok(out)
    }

    fn rewrite_expr(&self, e: &Expr, locals: &mut Locals, prefix: &[String]) -> Result<Expr> {
        Ok(match e {
            Expr::Name(n, span) => {
                if locals.contains(n) {
                    Expr::Name(n.clone(), *span)
                } else if let Some(c) = self.lookup_path(n, prefix, Namespace::Value) {
                    self.check_visible(&c, prefix, *span)?;
                    Expr::Name(c, *span)
                } else {
                    Expr::Name(n.clone(), *span)
                }
            }
            Expr::Construct(n, args, ty_args, span) => {
                let canonical = self.canonical_construct(n, prefix, *span)?;
                let args = args
                    .iter()
                    .map(|a| {
                        Ok(crate::ast::Arg {
                            name: a.name.clone(),
                            value: self.rewrite_expr(&a.value, locals, prefix)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?;
                let ty_args = self.rewrite_type_args(ty_args, prefix, *span)?;
                Expr::Construct(canonical, args, ty_args, *span)
            }
            Expr::Unary(op, o, span) => {
                Expr::Unary(*op, Box::new(self.rewrite_expr(o, locals, prefix)?), *span)
            }
            Expr::Binary(op, l, r, span) => Expr::Binary(
                *op,
                Box::new(self.rewrite_expr(l, locals, prefix)?),
                Box::new(self.rewrite_expr(r, locals, prefix)?),
                *span,
            ),
            Expr::Call(f, args, ty_args, span) => {
                let ty_args = self.rewrite_type_args(ty_args, prefix, *span)?;
                Expr::Call(
                    Box::new(self.rewrite_expr(f, locals, prefix)?),
                    self.rewrite_args(args, locals, prefix)?,
                    ty_args,
                    *span,
                )
            }
            Expr::Method(r, name, args, ty_args, span) => {
                let ty_args = self.rewrite_type_args(ty_args, prefix, *span)?;
                Expr::Method(
                    Box::new(self.rewrite_expr(r, locals, prefix)?),
                    name.clone(),
                    self.rewrite_args(args, locals, prefix)?,
                    ty_args,
                    *span,
                )
            }
            Expr::Field(r, name, span) => Expr::Field(
                Box::new(self.rewrite_expr(r, locals, prefix)?),
                name.clone(),
                *span,
            ),
            Expr::Index(b, i, span) => Expr::Index(
                Box::new(self.rewrite_expr(b, locals, prefix)?),
                Box::new(self.rewrite_expr(i, locals, prefix)?),
                *span,
            ),
            Expr::List(items, span) => Expr::List(
                items
                    .iter()
                    .map(|x| self.rewrite_expr(x, locals, prefix))
                    .collect::<Result<Vec<_>>>()?,
                *span,
            ),
            Expr::Tuple(items, span) => Expr::Tuple(
                items
                    .iter()
                    .map(|x| self.rewrite_expr(x, locals, prefix))
                    .collect::<Result<Vec<_>>>()?,
                *span,
            ),
            Expr::Map(entries, span) => Expr::Map(
                entries
                    .iter()
                    .map(|(k, v)| {
                        Ok((
                            self.rewrite_expr(k, locals, prefix)?,
                            self.rewrite_expr(v, locals, prefix)?,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
                *span,
            ),
            Expr::Lambda(params, body, span) => {
                let mut inner = locals.clone();
                inner.push();
                let params = self.rewrite_params(params, &mut inner, prefix)?;
                let body = self.rewrite_expr(body, &mut inner, prefix)?;
                Expr::Lambda(params, Box::new(body), *span)
            }
            Expr::Pipe(l, r, span) => Expr::Pipe(
                Box::new(self.rewrite_expr(l, locals, prefix)?),
                Box::new(self.rewrite_expr(r, locals, prefix)?),
                *span,
            ),
            Expr::Range(a, b, span) => Expr::Range(
                Box::new(self.rewrite_expr(a, locals, prefix)?),
                Box::new(self.rewrite_expr(b, locals, prefix)?),
                *span,
            ),
            Expr::If(c, then, els, span) => {
                let c = self.rewrite_expr(c, locals, prefix)?;
                let mut t = then.clone();
                self.rewrite_block(&mut t, locals, prefix)?;
                let e = match els {
                    Some(e) => Some(Box::new(self.rewrite_expr(e, locals, prefix)?)),
                    None => None,
                };
                Expr::If(Box::new(c), t, e, *span)
            }
            Expr::Match(value, arms, span) => {
                let value = self.rewrite_expr(value, locals, prefix)?;
                let arms = arms
                    .iter()
                    .map(|a| self.rewrite_arm(a, locals, prefix))
                    .collect::<Result<Vec<_>>>()?;
                Expr::Match(Box::new(value), arms, *span)
            }
            Expr::ListComp {
                value,
                pattern,
                iterable,
                filter,
                span,
            } => {
                // The iterable is outside the pattern's scope; the value and
                // filter see the pattern bindings (§24).
                let iterable = self.rewrite_expr(iterable, locals, prefix)?;
                let pattern = self.rewrite_pattern(pattern, prefix)?;
                let mut inner = locals.clone();
                inner.push();
                for b in pattern.bindings() {
                    inner.declare(&b);
                }
                let value = self.rewrite_expr(value, &mut inner, prefix)?;
                let filter = match filter {
                    Some(f) => Some(Box::new(self.rewrite_expr(f, &mut inner, prefix)?)),
                    None => None,
                };
                Expr::ListComp {
                    value: Box::new(value),
                    pattern,
                    iterable: Box::new(iterable),
                    filter,
                    span: *span,
                }
            }
            Expr::MapComp {
                key,
                value,
                pattern,
                iterable,
                filter,
                span,
            } => {
                let iterable = self.rewrite_expr(iterable, locals, prefix)?;
                let pattern = self.rewrite_pattern(pattern, prefix)?;
                let mut inner = locals.clone();
                inner.push();
                for b in pattern.bindings() {
                    inner.declare(&b);
                }
                let key = self.rewrite_expr(key, &mut inner, prefix)?;
                let value = self.rewrite_expr(value, &mut inner, prefix)?;
                let filter = match filter {
                    Some(f) => Some(Box::new(self.rewrite_expr(f, &mut inner, prefix)?)),
                    None => None,
                };
                Expr::MapComp {
                    key: Box::new(key),
                    value: Box::new(value),
                    pattern,
                    iterable: Box::new(iterable),
                    filter,
                    span: *span,
                }
            }
            Expr::Block(body, span) => {
                let mut b = body.clone();
                self.rewrite_block(&mut b, locals, prefix)?;
                Expr::Block(b, *span)
            }
            Expr::FStr(parts, span) => {
                let parts = parts
                    .iter()
                    .map(|p| match p {
                        FPart::Lit(_) => Ok(p.clone()),
                        FPart::Expr(e, spec) => Ok(FPart::Expr(
                            self.rewrite_expr(e, locals, prefix)?,
                            spec.clone(),
                        )),
                    })
                    .collect::<Result<Vec<_>>>()?;
                Expr::FStr(parts, *span)
            }
            // Literals and other leaves carry no names to rewrite.
            other @ Expr::Lit(..) => other.clone(),
        })
    }

    fn rewrite_args(
        &self,
        args: &[crate::ast::Arg],
        locals: &mut Locals,
        prefix: &[String],
    ) -> Result<Vec<crate::ast::Arg>> {
        args.iter()
            .map(|a| {
                Ok(crate::ast::Arg {
                    name: a.name.clone(),
                    value: self.rewrite_expr(&a.value, locals, prefix)?,
                })
            })
            .collect()
    }

    fn rewrite_arm(&self, a: &Arm, locals: &mut Locals, prefix: &[String]) -> Result<Arm> {
        locals.push();
        self.collect_pattern_bindings(&a.pattern, locals);
        let guard = a
            .guard
            .as_ref()
            .map(|g| self.rewrite_expr(g, locals, prefix))
            .transpose()?;
        let pattern = self.rewrite_pattern(&a.pattern, prefix)?;
        let mut body = a.body.clone();
        for s in &mut body {
            self.rewrite_stmt(s, locals, prefix)?;
        }
        locals.pop();
        Ok(Arm {
            pattern,
            guard,
            body,
        })
    }

    fn rewrite_pattern(&self, p: &Pattern, prefix: &[String]) -> Result<Pattern> {
        Ok(match p {
            Pattern::Variant(tag, ps, span) => {
                let canonical = self.canonical_construct(tag, prefix, *span)?;
                let ps = ps
                    .iter()
                    .map(|x| self.rewrite_pattern(x, prefix))
                    .collect::<Result<Vec<_>>>()?;
                Pattern::Variant(canonical, ps, *span)
            }
            Pattern::List(ps, span) => Pattern::List(
                ps.iter()
                    .map(|x| self.rewrite_pattern(x, prefix))
                    .collect::<Result<Vec<_>>>()?,
                *span,
            ),
            other => other.clone(),
        })
    }
}

/// Lexical binding set used to keep a local name from being rewritten to a
/// module item of the same name. A stack of frames, innermost last.
#[derive(Default, Clone)]
struct Locals {
    frames: Vec<HashSet<String>>,
}

impl Locals {
    fn push(&mut self) {
        self.frames.push(HashSet::new());
    }
    fn pop(&mut self) {
        self.frames.pop();
    }
    fn declare(&mut self, name: &str) {
        if let Some(f) = self.frames.last_mut() {
            f.insert(name.to_string());
        }
    }
    fn contains(&self, name: &str) -> bool {
        self.frames.iter().rev().any(|f| f.contains(name))
    }
}

fn join(prefix: &[String], name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{}::{name}", prefix.join("::"))
    }
}

fn canonical_path(base: &[String], seg: &str) -> Vec<String> {
    let mut v = base.to_vec();
    v.push(seg.to_string());
    v
}

/// Whether `prefix` is `module` or a descendant of it.
fn is_descendant(prefix: &[String], module: &[String]) -> bool {
    prefix.len() >= module.len() && prefix[..module.len()] == module[..]
}

fn item_span(item: &Item) -> Span {
    match item {
        Item::Fn { span, .. }
        | Item::Struct { span, .. }
        | Item::Enum { span, .. }
        | Item::Alias { span, .. }
        | Item::Const { span, .. }
        | Item::Trait { span, .. }
        | Item::Impl { span, .. }
        | Item::Use { span, .. }
        | Item::Module { span, .. }
        | Item::Expr(_, span) => *span,
    }
}
