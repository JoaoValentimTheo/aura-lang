//! Static checks: reserved names, mutability, and name resolution.
//!
//! Aura checks are conservative but complete: every name used must be
//! declared, every assignment must target a `let mut` binding, and every
//! forbidden construct (e.g. `break` outside a loop) is rejected by the
//! parser or reported here.

use std::cell::RefCell;
use std::collections::HashMap;

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use crate::types::Ty;

/// Re-export of the shared type representation, for callers that expect
/// `check::types::Ty`.
pub use crate::types;

/// Normalize a resolved union's members at the `TypeExpr` level.
///
/// Alias resolution can duplicate a union member subtree (`type T = A | A`),
/// which would otherwise grow the resolved type exponentially with each
/// additional alias level. Resolved unions therefore flatten nested unions and
/// Return-position compatibility. A declared return type may mention a generic
/// parameter (`fn f<T>(x: T) -> T`, `-> Box<T>`, `-> T | int`). The body must
/// be sound for **every** substitution the caller will apply, so a parameter
/// is not universally permissive here the way [`Ty::compatible_with`] treats it
/// when binding an argument:
///
/// * the declared type matches an actual that is identical, or an actual that
///   matches one member of a declared union (`T | int` accepts a `T`);
/// * a declared parameter is not satisfied by a concrete incompatible value
///   (`fn f<T>(x: T) -> T { return 5 }` is `E3005`);
/// * a concrete declared type is not satisfied by a universally-quantified
///   actual (`fn f<T>(x: T) -> int { return x }` is `E3005`, because `T` may be
///   any type);
/// * `Unknown` remains permissive (`LANGUAGE_SPEC.md` §2.3).
fn return_compatible(expected: &Ty, actual: &Ty) -> bool {
    if matches!(expected, Ty::Unknown) || matches!(actual, Ty::Unknown) {
        return true;
    }
    return_assignable(expected, actual)
}

/// The recursive core of [`return_compatible`]. Handles `Unknown` as
/// permissive at **every** position (`LANGUAGE_SPEC.md` §2.3: a rule is not
/// applied to a type the checker cannot determine — so `[T]` accepts `[]`,
/// which infers `[unknown]`); a declared union accepts a member; lists, maps,
/// and applications compare element-wise, so a parameter nested inside one is
/// still checked. A remaining unresolved parameter against a concrete type is
/// not soundly assignable in either direction.
fn return_assignable(expected: &Ty, actual: &Ty) -> bool {
    if matches!(expected, Ty::Unknown) || matches!(actual, Ty::Unknown) {
        return true;
    }
    if expected == actual {
        return true;
    }
    match (expected, actual) {
        (Ty::Union(members), _) => members.iter().any(|m| return_assignable(m, actual)),
        (_, Ty::Union(members)) => members.iter().all(|m| return_assignable(expected, m)),
        (Ty::List(a), Ty::List(b)) => return_assignable(a, b),
        (Ty::Map(ek, ev), Ty::Map(ak, av)) => {
            return_assignable(ek, ak) && return_assignable(ev, av)
        }
        (Ty::App(a, xa), Ty::App(b, xb)) => {
            a == b
                && xa.len() == xb.len()
                && xa.iter().zip(xb).all(|(x, y)| return_assignable(x, y))
        }
        (Ty::Named(a), Ty::App(b, xb)) => a == b && xb.is_empty(),
        (Ty::App(a, xa), Ty::Named(b)) => a == b && xa.is_empty(),
        _ if expected.has_param() || actual.has_param() => false,
        _ => expected == actual,
    }
}

/// remove structurally-equal duplicate members, mirroring the normalization
/// [`Ty::union`] already performs. Member order is not canonicalized here
/// because it is not observable: the only consumer is `Ty::from_expr`, which
/// applies its own canonical `Ty::union`. A union with one remaining member is
/// that member; `union` never yields an empty list.
fn normalize_resolved_union(members: Vec<TypeExpr>) -> TypeExpr {
    fn flatten(m: TypeExpr, out: &mut Vec<TypeExpr>) {
        match m {
            TypeExpr::Union(inner) => {
                for x in inner {
                    flatten(x, out);
                }
            }
            other => out.push(other),
        }
    }
    let mut flat = Vec::with_capacity(members.len());
    for m in members {
        flatten(m, &mut flat);
    }
    // Deduplicate by canonical spelling, which is an injective serialization
    // for the type grammar, in one linear pass that preserves first-seen
    // order. This is what collapses `T = A | A` after alias substitution.
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::with_capacity(flat.len());
    for m in flat {
        if seen.insert(m.name()) {
            unique.push(m);
        }
    }
    match unique.len() {
        0 => TypeExpr::None,
        1 => unique.pop().unwrap_or(TypeExpr::None),
        _ => TypeExpr::Union(unique),
    }
}

/// Maximum AST nesting the checker will descend before reporting a limit.
/// Prevents a flat but deeply nested program from exhausting the host stack.
const MAX_AST_DEPTH: usize = 256;

/// A lexical scope of bindings.
#[derive(Debug, Default)]
struct Scope {
    /// name -> mutable
    vars: HashMap<String, bool>,
    /// names declared in this exact scope, for redeclaration checks
    declares: HashMap<String, Span>,
}

/// A declaration carried from an earlier session into a new checker.
///
/// The REPL builds one of these per declaration a submission introduces; the
/// list is the session's semantic state, so the checker sees every binding,
/// function, and type the interpreter session already holds.
#[derive(Debug, Clone)]
pub enum GlobalDecl {
    /// `let [mut] name = ...`
    Binding {
        /// Name.
        name: String,
        /// Whether it is mutable.
        mutable: bool,
        /// The binding's declared type annotation, if any. Carried across REPL
        /// submissions so a field read on the binding can infer its struct type
        /// (`LANGUAGE_SPEC.md` §17.5). Unannotated bindings carry `None`.
        ty: Option<TypeExpr>,
    },
    /// `fn name(...) -> ret`
    Function {
        /// Name.
        name: String,
        /// Declared return type, if any.
        ret: Option<TypeExpr>,
        /// Parameters in declaration order.
        params: Vec<ParamDecl>,
        /// Generic type parameters `(name, bounds)` in declaration order.
        type_params: Vec<(String, Vec<String>)>,
    },
    /// `struct Name { fields... }`
    Struct {
        /// Name.
        name: String,
        /// Generic type parameters `(name, bounds)` in declaration order.
        type_params: Vec<(String, Vec<String>)>,
        /// Fields in declaration order, as `(name, type, public)`. `public`
        /// records whether the field was declared `pub` (`LANGUAGE_SPEC.md`
        /// §28).
        fields: Vec<(String, TypeExpr, bool)>,
    },
    /// `enum Name { variants... }`
    Enum {
        /// Name.
        name: String,
        /// Variants, each with its payload type annotations.
        variants: Vec<(String, Vec<TypeExpr>)>,
    },
    /// `type Name = T`
    Alias {
        /// Name.
        name: String,
        /// Generic type parameters `(name, bounds)` in declaration order.
        type_params: Vec<(String, Vec<String>)>,
        /// The aliased type expression.
        target: TypeExpr,
    },
    /// Methods introduced by an `impl` block, grouped by their target struct.
    /// Carried across REPL submissions so methods declared in one submission
    /// remain callable in later ones. `trait_name` is `Some` for
    /// `impl Trait for Struct`.
    Impl {
        /// The nominal struct the methods belong to.
        target: String,
        /// Generic type parameters `(name, bounds)` in declaration order.
        type_params: Vec<(String, Vec<String>)>,
        /// The trait being implemented, when this is `impl Trait for Struct`.
        trait_name: Option<String>,
        /// The canonical module path the block was declared in.
        owner: Vec<String>,
        /// Each method as `(name, return annotation, params)`.
        methods: Vec<MethodDecl>,
    },
    /// A trait declared with `trait Name { ... }`. Carried across REPL
    /// submissions so a trait declared in one submission can be implemented,
    /// and its contract checked, in later ones.
    Trait {
        /// The trait name.
        name: String,
        /// Generic type parameters `(name, bounds)` in declaration order.
        type_params: Vec<(String, Vec<String>)>,
        /// Declared methods as `(name, return annotation, params)`.
        methods: Vec<MethodDecl>,
    },
}

/// A parameter declaration carried across REPL submissions.
#[derive(Debug, Clone, PartialEq)]
pub struct ParamDecl {
    /// Parameter name.
    pub name: String,
    /// Declared type annotation, if any.
    pub ty: Option<TypeExpr>,
    /// Whether the parameter was declared `mut`.
    pub mutable: bool,
}

impl ParamDecl {
    /// Build from an AST parameter.
    #[must_use]
    pub fn from_param(p: &Param) -> ParamDecl {
        ParamDecl {
            name: p.name.clone(),
            ty: p.ty.clone(),
            mutable: p.mutable,
        }
    }
}

/// A method's declaration carried across REPL submissions.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    /// Method name.
    pub name: String,
    /// Declared return type, if any.
    pub ret: Option<TypeExpr>,
    /// Parameters in declaration order (the first is the receiver).
    pub params: Vec<ParamDecl>,
    /// Whether the method is exported (`pub`), for cross-module visibility.
    pub public: bool,
}

/// A top-level function's signature as known to the checker.
///
/// The return type drives expression inference; `params` drives the static
/// argument check at directly resolved calls (FEATURE_001, `LANGUAGE_SPEC.md`
/// §6.5) and the named-argument mapping (FEATURE_002, §15.7). A parameter's
/// type is `None` when it has no annotation.
#[derive(Debug, Clone)]
struct FnSig {
    /// The declared return type, if annotated.
    ret: Option<Ty>,
    /// One entry per parameter, in declaration order.
    params: Vec<ParamSig>,
    /// For a method, whether its receiver is declared `mut self`, which grants
    /// the body mutable capability over the caller's value. Always `false`
    /// for a plain function.
    mut_receiver: bool,
    /// The declaration's source span, for overload diagnostics.
    span: Span,
    /// For a method, the module path it was declared in (`[]` for the root)
    /// and whether it is exported (`pub`). A method is reachable across a
    /// module boundary only when it is `pub` or the caller is the owning
    /// module or a descendant (`LANGUAGE_SPEC.md` §28).
    owner: Vec<String>,
    /// Whether the method is visible outside its module (`pub`).
    ///
    /// The flag is `true` for a plain function: a top-level function is
    /// governed by the same rule, but its reachability is already resolved by
    /// the resolver (a private one is never rewritten into a caller's scope).
    public: bool,
    /// Generic type parameters in scope for this signature, in declaration
    /// order, each with its trait bounds. A method carries the enclosing
    /// `impl`'s parameters followed by its own. Identity is computed up to
    /// alpha-renaming of these names.
    type_params: Vec<(String, Vec<String>)>,
}

/// A parameter's checker signature (name and declared type). Whether the
/// parameter is `mut` is a property of the callee's body, not of a call, so it
/// is not part of the call-checking signature.
#[derive(Debug, Clone)]
struct ParamSig {
    /// Parameter name.
    name: String,
    /// Declared type, if annotated.
    ty: Option<Ty>,
}

/// The call-site context of a method invocation needed to instantiate a
/// generic method: the receiver type (which binds the enclosing `impl`'s
/// parameters) and any explicit type arguments written on the call.
struct MethodSite<'a> {
    /// The statically known receiver type.
    recv: &'a Ty,
    /// Explicit type arguments written at the call site.
    ty_args: &'a [TypeExpr],
}

/// Resolve a carried-across parameter declaration list into checker signatures,
/// expanding type aliases leniently (the same policy as hoisting).
/// The module path encoded in a canonical name: every `::` segment but the
/// last. A bare name (the root module) yields `[]`.
fn canonical_module_of(canonical: &str) -> Vec<String> {
    let mut segs: Vec<String> = canonical.split("::").map(str::to_string).collect();
    segs.pop();
    segs
}

/// Whether `current` is `owner` or a descendant of it. A caller inside a
/// module (or a nested one) may reach that module's private items.
fn is_descendant_module(current: &[String], owner: &[String]) -> bool {
    current.len() >= owner.len() && current[..owner.len()] == owner[..]
}

fn params_of(params: &[ParamDecl], c: &Checker, pnames: &[String]) -> Vec<ParamSig> {
    params
        .iter()
        .map(|p| ParamSig {
            name: p.name.clone(),
            ty: p.ty.as_ref().map(|t| c.session_type(t, pnames)),
        })
        .collect()
}

/// The checker. Reports the first error, matching the CLI contract.
pub struct Checker {
    scopes: Vec<Scope>,
    /// Whether a `main` function was seen.
    pub has_main: bool,
    /// Every top-level function (hoisted), by name, as an ordered **overload
    /// set** (`LANGUAGE_SPEC.md` §15.7). Overload identity is the name plus the
    /// ordered parameter types; the return type is not part of identity, so two
    /// declarations with the same input types are a duplicate.
    functions: HashMap<String, Vec<FnSig>>,
    /// Names of every top-level type (struct, enum, alias).
    types: HashMap<String, Span>,
    /// The kind of each user type: `struct`, `enum`, or `alias`.
    type_kinds: HashMap<String, String>,
    /// Field types of each struct, by struct name.
    struct_fields: HashMap<String, HashMap<String, Ty>>,
    /// Struct field names in declaration order, for positional construction.
    struct_field_order: HashMap<String, Vec<String>>,
    /// Whether each struct field is exported (`pub`), by struct name and field.
    /// A field with no `pub` is private to the struct's module: it cannot be
    /// named in a construction, read, or written from outside that module
    /// (`LANGUAGE_SPEC.md` §28).
    struct_public_fields: HashMap<String, HashMap<String, bool>>,
    /// Methods of each struct, by struct name, then by method name as an
    /// ordered overload set. The method table is keyed by the nominal struct
    /// type, so two structs may declare a method with the same name without
    /// collision (`LANGUAGE_SPEC.md` §17.6). Overload identity is the method
    /// name plus the ordered parameter types (the receiver included).
    struct_methods: HashMap<String, HashMap<String, Vec<FnSig>>>,
    /// Declared traits, by name, each holding its method signatures in
    /// declaration order. A trait is a behavioral contract with no value
    /// representation (`LANGUAGE_SPEC.md` §17.7). Order is preserved so
    /// completeness and signature diagnostics deterministically name the first
    /// affected method.
    traits: HashMap<String, Vec<(String, FnSig)>>,
    /// Which trait each struct implements, keyed by `(struct, trait)`, so a
    /// duplicate `impl Trait for Struct` can be rejected.
    trait_impls: HashMap<(String, String), Span>,
    /// Enum variant tags declared anywhere, with defining enum name.
    variants: HashMap<String, String>,
    /// Payload types of each enum variant, by tag.
    variant_payloads: HashMap<String, Vec<Ty>>,
    /// The declared return type of the function currently being checked.
    return_type: Option<Ty>,
    /// The declared type of annotated bindings in scope, innermost last.
    value_types: Vec<HashMap<String, Ty>>,
    /// `_`-prefixed parameters of the function currently being checked, with
    /// their spans. By contract, each must remain unused.
    underscore_params: Vec<(String, Span)>,
    /// Names referenced while checking the current function body.
    used_names: HashMap<String, Span>,
    /// Current AST descent depth, guarding against host stack exhaustion.
    depth: usize,
    /// Number of enclosing loops, so `break`/`continue` can be validated.
    loop_depth: usize,
    /// Top-level constant names mapped to their source order.
    const_order: HashMap<String, usize>,
    /// While checking a constant initializer, the order index of that
    /// constant; constants at or after it are not yet initialized.
    active_const: Option<usize>,
    /// Whether each trait is exported (`pub`), by canonical name. A trait
    /// method's reachability follows its trait: a `pub` trait's methods are
    /// usable wherever the trait is, a private trait's only within its module.
    trait_public: HashMap<String, bool>,
    /// The declaring module of each trait, by canonical name.
    trait_owner: HashMap<String, Vec<String>>,
    /// The canonical module path of the item whose body is being checked.
    /// Method-call visibility is relative to this (`LANGUAGE_SPEC.md` §28).
    current_module: Vec<String>,
    /// Targets of `type Name = T` aliases, so a transparent alias resolves to
    /// the type it names.
    alias_targets: HashMap<String, TypeExpr>,
    /// Memoized successful alias resolutions, keyed by alias name. An alias is
    /// a global, context-free definition, so a resolution that succeeded (no
    /// cycle was involved) is reusable everywhere. This keeps resolution
    /// linear in the number of aliases even when an alias names another alias
    /// more than once (for example `type T = A | A`), which would otherwise
    /// fan out exponentially. Failures are never cached, so cycle diagnostics
    /// keep their precise span.
    resolved_aliases: RefCell<HashMap<String, TypeExpr>>,
    /// Value-namespace names carried from earlier REPL submissions (bindings,
    /// functions, constants). Used so a new submission cannot redeclare one
    /// (`E2007`). Types live in their own namespace (`types`) and are checked
    /// there, so `struct S` and `fn S` may coexist exactly as in a module.
    session_value_names: HashMap<String, Span>,
    /// Declared generic type parameters of each user type (struct, enum, or
    /// alias), by canonical name, in declaration order. Empty for a
    /// non-generic type. Enforces arity on every application.
    type_type_params: HashMap<String, Vec<String>>,
    /// Trait bounds declared on each type parameter of a user type, aligned
    /// with [`Checker::type_type_params`]. Used when an `impl` binds the
    /// parameter.
    type_type_param_bounds: HashMap<String, Vec<Vec<String>>>,
    /// Declared generic type parameters of each trait, by canonical name.
    trait_type_params: HashMap<String, Vec<String>>,
    /// The generic type parameters currently in scope while checking a body,
    /// with trait bounds, innermost last. A name here is a `Ty::Param`.
    current_type_params: Vec<Vec<(String, Vec<String>)>>,
    /// Trait names seen in this module, collected in the first hoist pass so a
    /// bound (`T: Trait`) can be validated before the trait table itself is
    /// built (which happens after function signatures are annotated).
    declared_traits: std::collections::HashSet<String>,
}

impl Checker {
    /// Build a checker.
    fn new() -> Checker {
        Checker {
            scopes: vec![Scope::default()],
            has_main: false,
            functions: HashMap::new(),
            types: HashMap::new(),
            type_kinds: HashMap::new(),
            struct_fields: HashMap::new(),
            struct_field_order: HashMap::new(),
            struct_public_fields: HashMap::new(),
            struct_methods: HashMap::new(),
            traits: HashMap::new(),
            trait_impls: HashMap::new(),
            variants: HashMap::new(),
            variant_payloads: HashMap::new(),
            return_type: None,
            value_types: vec![HashMap::new()],
            underscore_params: Vec::new(),
            used_names: HashMap::new(),
            depth: 0,
            const_order: HashMap::new(),
            active_const: None,
            loop_depth: 0,
            trait_public: HashMap::new(),
            trait_owner: HashMap::new(),
            current_module: Vec::new(),
            alias_targets: HashMap::new(),
            resolved_aliases: RefCell::new(HashMap::new()),
            session_value_names: HashMap::new(),
            type_type_params: HashMap::new(),
            type_type_param_bounds: HashMap::new(),
            trait_type_params: HashMap::new(),
            current_type_params: Vec::new(),
            declared_traits: std::collections::HashSet::new(),
        }
    }

    /// Build a checker that already knows a set of global names (used by the
    /// REPL to carry declarations across submissions). Each entry is
    /// `(name, mutable, is_function)`.
    #[must_use]
    pub fn with_globals(globals: &[(String, bool, bool)]) -> Checker {
        let decls: Vec<GlobalDecl> = globals
            .iter()
            .map(|(name, mutable, is_fn)| {
                if *is_fn {
                    GlobalDecl::Function {
                        name: name.clone(),
                        ret: None,
                        params: Vec::new(),
                        type_params: Vec::new(),
                    }
                } else {
                    GlobalDecl::Binding {
                        name: name.clone(),
                        mutable: *mutable,
                        ty: None,
                    }
                }
            })
            .collect();
        Checker::with_declarations(&decls)
    }

    /// Build a checker that knows the declarations carried across REPL
    /// submissions. This is the coherent session model: one declaration list
    /// feeds both bindings and the type tables, so the checker sees exactly
    /// what the interpreter session retains.
    #[must_use]
    pub fn with_declarations(decls: &[GlobalDecl]) -> Checker {
        let mut c = Checker::new();
        // Register type names first, so field/payload annotations that refer
        // to types declared in any order resolve.
        for d in decls {
            match d {
                GlobalDecl::Binding {
                    name,
                    mutable,
                    ty: _,
                } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), *mutable);
                    c.session_value_names.insert(name.clone(), Span::default());
                }
                GlobalDecl::Function {
                    name,
                    ret,
                    params,
                    type_params,
                } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.session_value_names.insert(name.clone(), Span::default());
                    let pnames: Vec<String> = type_params.iter().map(|(n, _)| n.clone()).collect();
                    let ret_ty = ret.as_ref().map(|t| c.session_type(t, &pnames));
                    let param_tys = params
                        .iter()
                        .map(|p| ParamSig {
                            name: p.name.clone(),
                            ty: p.ty.as_ref().map(|t| c.session_type(t, &pnames)),
                        })
                        .collect();
                    c.functions.entry(name.clone()).or_default().push(FnSig {
                        ret: ret_ty,
                        params: param_tys,
                        mut_receiver: false,
                        owner: Vec::new(),
                        public: true,
                        span: Span::default(),
                        type_params: type_params.clone(),
                    });
                }
                GlobalDecl::Struct {
                    name, type_params, ..
                } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.types.insert(name.clone(), Span::default());
                    c.type_kinds.insert(name.clone(), "struct".to_string());
                    c.type_type_params.insert(
                        name.clone(),
                        type_params.iter().map(|(n, _)| n.clone()).collect(),
                    );
                    c.type_type_param_bounds.insert(
                        name.clone(),
                        type_params.iter().map(|(_, b)| b.clone()).collect(),
                    );
                }
                GlobalDecl::Enum { name, .. } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.types.insert(name.clone(), Span::default());
                    c.type_kinds.insert(name.clone(), "enum".to_string());
                }
                GlobalDecl::Alias {
                    name,
                    type_params,
                    target,
                } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.types.insert(name.clone(), Span::default());
                    c.type_kinds.insert(name.clone(), "alias".to_string());
                    c.alias_targets.insert(name.clone(), target.clone());
                    c.type_type_params.insert(
                        name.clone(),
                        type_params.iter().map(|(n, _)| n.clone()).collect(),
                    );
                    c.type_type_param_bounds.insert(
                        name.clone(),
                        type_params.iter().map(|(_, b)| b.clone()).collect(),
                    );
                }
                GlobalDecl::Impl { .. } => {}
                GlobalDecl::Trait { .. } => {}
            }
        }
        // Then record fields, payloads, and variant tags, resolving aliases
        // now that every alias target is known.
        for d in decls {
            match d {
                GlobalDecl::Struct {
                    name,
                    type_params,
                    fields,
                } => {
                    let pnames: Vec<String> = type_params.iter().map(|(n, _)| n.clone()).collect();
                    let mut map = HashMap::new();
                    let mut order = Vec::new();
                    let mut public = HashMap::new();
                    for (f, t, is_pub) in fields {
                        map.insert(f.clone(), c.session_type(t, &pnames));
                        order.push(f.clone());
                        public.insert(f.clone(), *is_pub);
                    }
                    c.struct_fields.insert(name.clone(), map);
                    c.struct_field_order.insert(name.clone(), order);
                    c.struct_public_fields.insert(name.clone(), public);
                }
                GlobalDecl::Enum { variants, .. } => {
                    for (tag, payload) in variants {
                        let mut tys = Vec::with_capacity(payload.len());
                        for t in payload {
                            tys.push(Ty::from_expr_lenient(&c.resolve_type_expr_lenient(t)));
                        }
                        c.variants.insert(tag.clone(), String::new());
                        c.variant_payloads.insert(tag.clone(), tys);
                    }
                }
                // A persisted binding's declared type, restored into the same
                // `value_types` state `infer` consults. Unannotated bindings
                // stay `Unknown`, exactly as within a single submission.
                GlobalDecl::Binding {
                    name, ty: Some(t), ..
                } => {
                    let restored = Ty::from_expr_lenient(&c.resolve_type_expr_lenient(t));
                    c.value_types[0].insert(name.clone(), restored);
                }
                GlobalDecl::Impl {
                    target,
                    type_params,
                    trait_name,
                    owner,
                    methods,
                } => {
                    // Restore the persisted method table so a method declared
                    // in an earlier submission resolves in later ones. Methods
                    // are merged into the struct's single surface, since
                    // inherent and trait-provided methods share one namespace.
                    // Resolve signatures first (which borrows `c`) and merge
                    // afterwards so the table borrow does not overlap.
                    let pnames: Vec<String> = type_params.iter().map(|(n, _)| n.clone()).collect();
                    let mut restored: Vec<(String, FnSig)> = Vec::new();
                    for m in methods {
                        let ret_ty = m.ret.as_ref().map(|t| c.session_type(t, &pnames));
                        let param_tys = params_of(&m.params, &c, &pnames);
                        restored.push((
                            m.name.clone(),
                            FnSig {
                                ret: ret_ty,
                                params: param_tys,
                                mut_receiver: m.params.first().is_some_and(|p| p.mutable),
                                owner: owner.clone(),
                                public: m.public,
                                span: Span::default(),
                                type_params: type_params.clone(),
                            },
                        ));
                    }
                    let table = c.struct_methods.entry(target.clone()).or_default();
                    for (name, sig) in restored {
                        table.entry(name).or_default().push(sig);
                    }
                    if let Some(tname) = trait_name {
                        c.trait_impls
                            .insert((target.clone(), tname.clone()), Span::default());
                    }
                }
                GlobalDecl::Trait {
                    name,
                    type_params,
                    methods,
                } => {
                    // Restore the persisted trait contract so a later
                    // submission can implement it. Method order is preserved
                    // so diagnostics name the first declared method.
                    let pnames: Vec<String> = type_params.iter().map(|(n, _)| n.clone()).collect();
                    let mut table: Vec<(String, FnSig)> = Vec::with_capacity(methods.len());
                    for m in methods {
                        let ret_ty = m.ret.as_ref().map(|t| c.session_type(t, &pnames));
                        let param_tys = params_of(&m.params, &c, &pnames);
                        table.push((
                            m.name.clone(),
                            FnSig {
                                ret: ret_ty,
                                params: param_tys,
                                mut_receiver: m.params.first().is_some_and(|p| p.mutable),
                                owner: Vec::new(),
                                public: true,
                                span: Span::default(),
                                type_params: type_params.clone(),
                            },
                        ));
                    }
                    c.trait_type_params.insert(
                        name.clone(),
                        type_params.iter().map(|(n, _)| n.clone()).collect(),
                    );
                    c.traits.insert(name.clone(), table);
                }
                _ => {}
            }
        }
        c
    }

    /// Check a module against this checker.
    ///
    /// # Errors
    /// Returns the first diagnostic found.
    pub fn check(&mut self, m: &Module) -> Result<()> {
        self.hoist(m)?;
        for item in &m.items {
            self.item(item)?;
        }
        Ok(())
    }

    /// Check a module in an explicit [`crate::CompileMode`].
    ///
    /// This is the one place the "does a module need a `main`?" decision is
    /// made; every entry point funnels through it.
    ///
    /// # Errors
    /// Returns the first checker diagnostic, plus `E4027` in program mode.
    pub fn check_mode(&mut self, m: &Module, mode: crate::CompileMode) -> Result<()> {
        self.check(m)?;
        match mode {
            crate::CompileMode::Module => Ok(()),
            crate::CompileMode::Program => self.require_main(),
        }
    }

    /// Pre-pass: collect every top-level name before checking bodies, so that
    /// forward references between functions and constants resolve. This makes
    /// the checker agree with the runtime's declaration-then-initialize order.
    fn hoist(&mut self, m: &Module) -> Result<()> {
        // Collect trait names first, so a generic bound (`T: Trait`) written on
        // any declaration is validated against a known trait even though the
        // full trait table is built later in this pass.
        for item in &m.items {
            if let Item::Trait { name, .. } = item {
                self.declared_traits.insert(name.clone());
            }
        }
        let mut declared: HashMap<String, Span> = HashMap::new();
        // Whether the value name first declared here is a function, so a later
        // item may overload only another function, never a constant or binding
        // (`LANGUAGE_SPEC.md` §15.7).
        let mut declared_is_fn: HashMap<String, bool> = HashMap::new();
        // For each name, the index of the first overload pushed by *this*
        // module. `with_declarations` may already have restored overloads from
        // earlier REPL submissions, and the annotation pass below must fill
        // this module's signatures, not overwrite the restored ones.
        let mut new_fn_base: HashMap<String, usize> = HashMap::new();
        for item in &m.items {
            match item {
                Item::Fn {
                    name,
                    span,
                    ret,
                    params,
                    public,
                    ..
                } => {
                    // Overloading: several top-level functions may share a name
                    // when their ordered parameter types differ. A name already
                    // declared by a non-function (a constant or top-level
                    // `let`), or carried from an earlier REPL submission as a
                    // non-function, is `E2007`. Types are a separate namespace,
                    // so `struct S` and `fn S` may coexist as in a module.
                    let is_session = self.session_value_names.contains_key(name);
                    let clash_non_fn = is_session && !self.functions.contains_key(name);
                    if clash_non_fn || declared_is_fn.get(name).is_some_and(|&is_fn| !is_fn) {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{name}` is already declared in this scope"),
                            *span,
                        ));
                    }
                    declared.entry(name.clone()).or_insert(*span);
                    declared_is_fn.insert(name.clone(), true);
                    let ret_ty = ret.as_ref().map(Ty::from_expr_lenient);
                    // Parameter names are recorded now; parameter types are
                    // filled in by the annotation pass below, once every type
                    // name is known.
                    let param_sigs = params
                        .iter()
                        .map(|p| ParamSig {
                            name: p.name.clone(),
                            ty: None,
                        })
                        .collect();
                    // Record each overload's owning module and visibility. A
                    // private function is unreachable from another module by
                    // name (the resolver never rewrites it there), but an
                    // **overload set** can mix visibilities: the public
                    // overload is reachable, so resolution must consider only
                    // the visible ones (`LANGUAGE_SPEC.md` §28).
                    let owner = canonical_module_of(name);
                    self.functions.entry(name.clone()).or_default().push(FnSig {
                        ret: ret_ty,
                        params: param_sigs,
                        mut_receiver: false,
                        owner,
                        public: *public,
                        span: *span,
                        type_params: Vec::new(),
                    });
                    let base = self.functions.get(name).map_or(0, Vec::len) - 1;
                    new_fn_base.entry(name.clone()).or_insert(base);
                    self.scopes[0].declares.entry(name.clone()).or_insert(*span);
                    self.scopes[0].vars.insert(name.clone(), false);
                }
                Item::Const { name, span, .. } => {
                    // Constants share the value-namespace scope rule: a prior
                    // declaration (module or session) makes this `E2007`.
                    // `const NAME` and a top-level `let NAME` are the same
                    // declaration.
                    if declared.contains_key(name) || self.session_value_names.contains_key(name) {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{name}` is already declared in this scope"),
                            *span,
                        ));
                    }
                    declared.insert(name.clone(), *span);
                    declared_is_fn.insert(name.clone(), false);
                    self.scopes[0].declares.entry(name.clone()).or_insert(*span);
                    self.scopes[0].vars.insert(name.clone(), false);
                    let ord = self.const_order.len();
                    self.const_order.insert(name.clone(), ord);
                }
                Item::Struct {
                    name,
                    type_params,
                    span,
                    ..
                } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "struct".to_string());
                    self.type_type_params.insert(
                        name.clone(),
                        type_params.iter().map(|p| p.name.clone()).collect(),
                    );
                    self.type_type_param_bounds.insert(
                        name.clone(),
                        type_params.iter().map(|p| p.bounds.clone()).collect(),
                    );
                }
                Item::Enum {
                    name,
                    type_params,
                    variants,
                    span,
                    ..
                } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "enum".to_string());
                    self.type_type_params.insert(
                        name.clone(),
                        type_params.iter().map(|p| p.name.clone()).collect(),
                    );
                    self.type_type_param_bounds.insert(
                        name.clone(),
                        type_params.iter().map(|p| p.bounds.clone()).collect(),
                    );
                    for variant in variants {
                        if let Some(prev) = self.variants.get(&variant.tag) {
                            return Err(Diag::new(
                                codes::DUPLICATE_VARIANT,
                                if prev == name {
                                    format!("variant `{}` is declared more than once", variant.tag)
                                } else {
                                    format!(
                                        "variant `{}` is already declared by enum `{prev}`; variant names must be unique across the program",
                                        variant.tag
                                    )
                                },
                                variant.span,
                            ));
                        }
                        self.variants.insert(variant.tag.clone(), name.clone());
                    }
                }
                Item::Alias {
                    name,
                    type_params,
                    target,
                    span,
                    ..
                } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "alias".to_string());
                    self.alias_targets.insert(name.clone(), target.clone());
                    self.type_type_params.insert(
                        name.clone(),
                        type_params.iter().map(|p| p.name.clone()).collect(),
                    );
                    self.type_type_param_bounds.insert(
                        name.clone(),
                        type_params.iter().map(|p| p.bounds.clone()).collect(),
                    );
                }
                Item::Use { .. }
                | Item::Expr(..)
                | Item::Impl { .. }
                | Item::Trait { .. }
                | Item::Module { .. } => {}
            }
        }
        // Validate every written type annotation now that all names are known.
        // `filled` tracks, per function name, how many of its hoisted overload
        // signatures have been annotated, so the Nth declaration fills the Nth
        // signature.
        let mut filled: HashMap<String, usize> = HashMap::new();
        for item in &m.items {
            match item {
                Item::Struct {
                    name,
                    type_params,
                    fields,
                    ..
                } => {
                    self.check_type_params(type_params, Span::default())?;
                    self.check_type_param_bounds(type_params)?;
                    self.push_type_params(type_params);
                    let mut map = HashMap::new();
                    let mut order = Vec::new();
                    let mut public = HashMap::new();
                    for field in fields {
                        if map.contains_key(&field.name) {
                            return Err(Diag::new(
                                codes::DUPLICATE_FIELD,
                                format!(
                                    "field `{}` is declared more than once in struct `{name}`",
                                    field.name
                                ),
                                field.span,
                            ));
                        }
                        let ty = self.annotation(&field.ty, field.span)?;
                        map.insert(field.name.clone(), ty);
                        order.push(field.name.clone());
                        public.insert(field.name.clone(), field.public);
                    }
                    self.struct_fields.insert(name.clone(), map);
                    self.struct_field_order.insert(name.clone(), order);
                    self.struct_public_fields.insert(name.clone(), public);
                    self.pop_type_params();
                }
                Item::Enum {
                    type_params,
                    variants,
                    ..
                } => {
                    self.check_type_params(type_params, Span::default())?;
                    self.check_type_param_bounds(type_params)?;
                    self.push_type_params(type_params);
                    for variant in variants {
                        let mut tys = Vec::with_capacity(variant.payload.len());
                        for pty in &variant.payload {
                            tys.push(self.annotation(pty, variant.span)?);
                        }
                        self.variant_payloads.insert(variant.tag.clone(), tys);
                    }
                    self.pop_type_params();
                }
                Item::Alias {
                    type_params,
                    target,
                    span,
                    ..
                } => {
                    self.check_type_params(type_params, Span::default())?;
                    self.check_type_param_bounds(type_params)?;
                    self.push_type_params(type_params);
                    self.annotation(target, *span)?;
                    self.pop_type_params();
                }
                Item::Fn {
                    name,
                    type_params,
                    params,
                    ret,
                    ret_span,
                    ..
                } => {
                    self.check_type_params(type_params, Span::default())?;
                    self.check_type_param_bounds(type_params)?;
                    self.push_type_params(type_params);
                    let mut param_tys = Vec::with_capacity(params.len());
                    for p in params {
                        let ty = match &p.ty {
                            Some(pty) => Some(self.annotation(pty, p.span)?),
                            None => None,
                        };
                        param_tys.push(ParamSig {
                            name: p.name.clone(),
                            ty,
                        });
                    }
                    let ret_ty = match ret {
                        Some(rt) => Some(self.annotation(rt, ret_span.unwrap_or_default())?),
                        None => None,
                    };
                    // `hoist` pushed one signature per declaration in source
                    // order; fill this declaration's signature in the same
                    // order so overloads are annotated positionally.
                    if let Some(set) = self.functions.get_mut(name) {
                        let base = *new_fn_base.get(name).unwrap_or(&0);
                        let idx = *filled.entry(name.clone()).or_insert(base);
                        if let Some(sig) = set.get_mut(idx) {
                            sig.ret = ret_ty;
                            sig.params = param_tys;
                            sig.type_params = type_params
                                .iter()
                                .map(|p| (p.name.clone(), p.bounds.clone()))
                                .collect();
                        }
                        filled.insert(name.clone(), idx + 1);
                    }
                    self.pop_type_params();
                }
                _ => {}
            }
        }
        // Overload identity is the name plus the ordered parameter types: two
        // top-level functions that share a name AND a parameter-type list are a
        // duplicate declaration (`E2007`), never two overloads
        // (`LANGUAGE_SPEC.md` §15.7). The return type and `mut` are not part of
        // identity.
        // Iterate in a deterministic order (by the earlier declaration's source
        // position) so that when several names carry duplicates, the diagnostic
        // is always the same. Iterating `self.functions` directly would depend
        // on `HashMap` iteration order and make the reported error — and so the
        // CLI exit code — nondeterministic.
        let mut names: Vec<&String> = self.functions.keys().collect();
        names.sort_by_key(|n| {
            self.functions
                .get(*n)
                .and_then(|set| set.first())
                .map_or((usize::MAX, usize::MAX), |s| (s.span.start, s.span.end))
        });
        for name in names {
            let set = &self.functions[name];
            for i in 0..set.len() {
                for j in (i + 1)..set.len() {
                    if Self::sig_identical(&set[i], &set[j]) {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!(
                                "function `{name}` is already defined with the same parameter types"
                            ),
                            set[j].span,
                        ));
                    }
                }
            }
        }
        // Behavior blocks last: field tables, trait tables, and method tables must
        // all be populated before method/field collision and method signatures
        // are resolved, regardless of source order.
        // 1. Traits: register each declared trait's method signatures.
        for item in &m.items {
            if let Item::Trait {
                name,
                type_params,
                methods,
                public,
                owner,
                span,
            } = item
            {
                self.trait_public.insert(name.clone(), *public);
                self.trait_owner.insert(name.clone(), owner.clone());
                if self.traits.contains_key(name) {
                    return Err(Diag::new(
                        codes::REDECLARED,
                        format!("trait `{name}` is already declared"),
                        *span,
                    ));
                }
                self.check_type_params(type_params, *span)?;
                self.check_type_param_bounds(type_params)?;
                self.trait_type_params.insert(
                    name.clone(),
                    type_params.iter().map(|p| p.name.clone()).collect(),
                );
                self.push_type_params(type_params);
                let mut table: Vec<(String, FnSig)> = Vec::with_capacity(methods.len());
                for m_item in methods {
                    let Item::Fn {
                        name: mname,
                        type_params: mtype_params,
                        params,
                        ret,
                        ret_span,
                        span: mspan,
                        ..
                    } = m_item
                    else {
                        continue;
                    };
                    if table.iter().any(|(n, _)| n == mname) {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!(
                                "trait method `{mname}` is declared more than once in `{name}`"
                            ),
                            *mspan,
                        ));
                    }
                    self.push_type_params(mtype_params);
                    let mut param_tys: Vec<ParamSig> = Vec::with_capacity(params.len());
                    for p in params {
                        // A parameter name declared twice is a same-scope
                        // redeclaration, exactly like a function or inherent
                        // method (`LANGUAGE_SPEC.md` §16.3). Without this a
                        // trait could declare a contract no `impl` could
                        // satisfy, since the matching method rejects it.
                        if param_tys.iter().any(|q| q.name == p.name) {
                            return Err(Diag::new(
                                codes::REDECLARED,
                                format!("`{}` is already declared in this scope", p.name),
                                p.span,
                            ));
                        }
                        let ty = match &p.ty {
                            Some(pty) => Some(self.annotation(pty, p.span)?),
                            None => None,
                        };
                        param_tys.push(ParamSig {
                            name: p.name.clone(),
                            ty,
                        });
                    }
                    let ret_ty = match ret {
                        Some(rt) => Some(self.annotation(rt, ret_span.unwrap_or_default())?),
                        None => None,
                    };
                    self.pop_type_params();
                    let mut method_p = type_params.clone();
                    method_p.extend(mtype_params.clone());
                    table.push((
                        mname.clone(),
                        FnSig {
                            ret: ret_ty,
                            params: param_tys,
                            mut_receiver: params.first().is_some_and(|p| p.mutable),
                            type_params: method_p
                                .iter()
                                .map(|p| (p.name.clone(), p.bounds.clone()))
                                .collect(),
                            owner: owner.clone(),
                            // A trait method's reachability follows its trait:
                            // a `pub` trait's methods are usable wherever the
                            // trait is, a private trait's only within its module
                            // (`LANGUAGE_SPEC.md` §28).
                            public: *public,
                            span: *mspan,
                        },
                    ));
                }
                self.traits.insert(name.clone(), table);
                self.pop_type_params();
            }
        }
        // 2. Inherent and trait implementations, merged into one method surface.
        for item in &m.items {
            if let Item::Impl {
                target,
                target_args,
                trait_name,
                trait_args,
                type_params,
                methods,
                owner,
                span,
            } = item
            {
                self.check_type_params(type_params, *span)?;
                self.check_type_param_bounds(type_params)?;
                self.push_type_params(type_params);
                // The target MUST be an already-declared nominal struct.
                match self.type_kinds.get(target) {
                    Some(kind) if kind == "struct" => {}
                    Some(_) => {
                        return Err(Diag::new(
                            codes::UNDEFINED,
                            format!("`{target}` is not a struct; `impl` requires a struct"),
                            *span,
                        ));
                    }
                    None => {
                        return Err(Diag::new(
                            codes::UNDEFINED,
                            format!("unknown struct `{target}` in `impl`"),
                            *span,
                        ));
                    }
                }
                // A generic `impl<T>` on a generic struct must apply the
                // struct's parameters; its target arguments must have the
                // struct's arity.
                self.check_type_args_arity(target, target_args, *span)?;
                if let Some(tname) = trait_name {
                    self.check_type_args_arity(tname, trait_args, *span)?;
                }
                // Multiple `impl` blocks for one struct are allowed: their
                // methods merge into the struct's single method surface, and
                // method overloading (`LANGUAGE_SPEC.md` §15.7) lets a later
                // block add an overload. A method with an identity already
                // present is rejected below as a duplicate.
                // Validate the trait, when this is a trait implementation.
                if let Some(tname) = trait_name {
                    let Some(trait_sig) = self.traits.get(tname).cloned() else {
                        return Err(Diag::new(
                            codes::UNDEFINED,
                            format!("unknown trait `{tname}`"),
                            *span,
                        ));
                    };
                    if self
                        .trait_impls
                        .insert((target.clone(), tname.clone()), *span)
                        .is_some()
                    {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{target}` already implements trait `{tname}`"),
                            *span,
                        ));
                    }
                    // Every declared trait method must be implemented exactly
                    // once with a compatible signature; a trait implementation
                    // may not add methods beyond the contract, and may not add
                    // a *second* method with a trait method's name of a
                    // different signature (`LANGUAGE_SPEC.md` §17.7).
                    for m_item in methods {
                        let Item::Fn {
                            name: mname,
                            params,
                            ret,
                            ret_span,
                            span: mspan,
                            ..
                        } = m_item
                        else {
                            continue;
                        };
                        let Some((_, expected)) = trait_sig.iter().find(|(n, _)| n == mname) else {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!(
                                    "trait `{tname}` has no method `{mname}`; a trait implementation may only implement the trait's methods"
                                ),
                                *mspan,
                            ));
                        };
                        // A trait method may be implemented only once.
                        if methods
                            .iter()
                            .filter(|mi| matches!(mi, Item::Fn { name, .. } if name == mname))
                            .count()
                            > 1
                        {
                            return Err(Diag::new(
                                codes::REDECLARED,
                                format!(
                                    "trait `{tname}` method `{mname}` is implemented more than once in `{target}`"
                                ),
                                *mspan,
                            ));
                        }
                        let mut param_tys = Vec::with_capacity(params.len());
                        for p in params {
                            let ty = match &p.ty {
                                Some(pty) => Some(self.annotation(pty, p.span)?),
                                None => None,
                            };
                            param_tys.push(ParamSig {
                                name: p.name.clone(),
                                ty,
                            });
                        }
                        let ret_ty = match ret {
                            Some(rt) => Some(self.annotation(rt, ret_span.unwrap_or_default())?),
                            None => None,
                        };
                        let actual = FnSig {
                            ret: ret_ty,
                            params: param_tys,
                            mut_receiver: params.first().is_some_and(|p| p.mutable),
                            owner: owner.clone(),
                            public: false,
                            span: *mspan,
                            type_params: Vec::new(),
                        };
                        if !self.method_sigs_compatible(expected, &actual) {
                            return Err(Diag::new(
                                codes::TYPE_MISMATCH,
                                format!(
                                    "trait `{tname}` method `{mname}` has an incompatible signature in `{target}`"
                                ),
                                *mspan,
                            ));
                        }
                    }
                    // Every declared trait method must be present.
                    for (mname, _) in &trait_sig {
                        if !methods
                            .iter()
                            .any(|mi| matches!(mi, Item::Fn { name, .. } if name == mname))
                        {
                            return Err(Diag::new(
                                codes::TRAIT_INCOMPLETE,
                                format!(
                                    "trait `{tname}` requires method `{mname}`, but `{target}` does not implement it"
                                ),
                                *span,
                            ));
                        }
                    }
                }
                // Merge the methods into the struct's single method surface.
                // A trait method's reachability follows its trait: if this is a
                // trait implementation, the method is visible exactly when the
                // trait is, relative to the trait's module. An inherent method
                // follows its own `pub` and the struct's module.
                let trait_publicity = trait_name
                    .as_ref()
                    .and_then(|t| self.trait_public.get(t).copied());
                let trait_module = trait_name
                    .as_ref()
                    .and_then(|t| self.trait_owner.get(t).cloned());
                for m_item in methods {
                    let Item::Fn {
                        name,
                        type_params: method_type_params,
                        params,
                        ret,
                        ret_span,
                        public: public_flag,
                        span: mspan,
                        ..
                    } = m_item
                    else {
                        continue;
                    };
                    // A method name may not collide with a field of the same
                    // struct (§17.6): member lookup must stay unambiguous.
                    if self
                        .struct_fields
                        .get(target)
                        .is_some_and(|fs| fs.contains_key(name))
                    {
                        return Err(Diag::new(
                            codes::DUPLICATE_FIELD,
                            format!("`{name}` is both a field and a method of struct `{target}`"),
                            *mspan,
                        ));
                    }
                    // A method's own type parameters extend the impl's for its
                    // signature (`impl<T> Box<T> { fn map<U>... }`).
                    self.push_type_params(method_type_params);
                    let mut param_tys = Vec::with_capacity(params.len());
                    for p in params {
                        let ty = match &p.ty {
                            Some(pty) => Some(self.annotation(pty, p.span)?),
                            None => None,
                        };
                        param_tys.push(ParamSig {
                            name: p.name.clone(),
                            ty,
                        });
                    }
                    let ret_ty = match ret {
                        Some(rt) => Some(self.annotation(rt, ret_span.unwrap_or_default())?),
                        None => None,
                    };
                    self.pop_type_params();
                    let mut_receiver = params.first().is_some_and(|p| p.mutable);
                    let mut method_p = type_params.clone();
                    method_p.extend(method_type_params.clone());
                    let sig = FnSig {
                        ret: ret_ty,
                        params: param_tys,
                        mut_receiver,
                        // A trait-provided method's owner is the trait's module,
                        // so its visibility is judged from there.
                        owner: trait_module.clone().unwrap_or_else(|| owner.clone()),
                        public: trait_publicity.unwrap_or(*public_flag),
                        span: *mspan,
                        type_params: method_p
                            .iter()
                            .map(|p| (p.name.clone(), p.bounds.clone()))
                            .collect(),
                    };
                    // One member namespace: inherent and trait methods, and two
                    // traits, share it. Overloading is allowed when the ordered
                    // parameter types differ; the same identity is a duplicate
                    // (`LANGUAGE_SPEC.md` §17.7).
                    let duplicate = self.struct_methods.get(target).is_some_and(|t| {
                        t.get(name)
                            .is_some_and(|set| set.iter().any(|s| Self::sig_identical(s, &sig)))
                    });
                    if duplicate {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!(
                                "method `{name}` is already defined for `{target}` with the same parameter types"
                            ),
                            *mspan,
                        ));
                    }
                    self.struct_methods
                        .entry(target.clone())
                        .or_default()
                        .entry(name.clone())
                        .or_default()
                        .push(sig);
                }
                self.pop_type_params();
            }
        }
        Ok(())
    }

    /// Check a single statement against the current global scope (used by
    /// the REPL, where `let mut` is valid at the top level).
    ///
    /// # Errors
    /// Returns the first diagnostic found.
    pub fn check_stmt(&mut self, s: &Stmt) -> Result<()> {
        self.stmt(s)
    }

    /// The statically inferred type of a `let` binding's initializer, as a
    /// written type expression, given the session declarations. Returns `None`
    /// when the type is `Unknown` (no source spelling) or the statement is not
    /// a `let`. The REPL uses this to persist an unannotated binding's nominal
    /// type across submissions (`LANGUAGE_SPEC.md` §17.6).
    #[must_use]
    pub fn let_binding_type(&self, s: &Stmt) -> Option<TypeExpr> {
        let Stmt::Let { value, .. } = s else {
            return None;
        };
        self.infer(value).to_type_expr()
    }

    /// Record a REPL global so later submissions can see it. Functions are
    /// also made callable.
    pub fn add_global(&mut self, name: &str, is_fn: bool) {
        self.scopes[0]
            .declares
            .insert(name.to_string(), Span::default());
        self.scopes[0].vars.insert(name.to_string(), false);
        if is_fn {
            let set = self.functions.entry(name.to_string()).or_default();
            if set.is_empty() {
                set.push(FnSig {
                    ret: None,
                    params: Vec::new(),
                    mut_receiver: false,
                    owner: Vec::new(),
                    public: true,
                    span: Span::default(),
                    type_params: Vec::new(),
                });
            }
        }
    }

    /// Check a module, returning the first diagnostic.
    ///
    /// # Errors
    /// Returns the first diagnostic found.
    pub fn module(m: &Module) -> Result<()> {
        let mut c = Checker::new();
        c.check(m)
    }

    /// Check a module in an explicit mode (this is the canonical entry point).
    ///
    /// # Errors
    /// Returns the first checker diagnostic; `E4027` in program mode without a
    /// `main`.
    pub fn module_in_mode(m: &Module, mode: crate::CompileMode) -> Result<()> {
        let mut c = Checker::new();
        c.check_mode(m, mode)
    }

    /// Check a module and require an entry point.
    ///
    /// # Errors
    /// Returns `E4027` when the module declares no `fn main`.
    pub fn module_with_main(m: &Module) -> Result<()> {
        let mut c = Checker::new();
        c.check(m)?;
        c.require_main()
    }

    /// Require that an entry point was declared.
    ///
    /// # Errors
    /// Returns `E4027` when no `fn main` was seen.
    pub fn require_main(&self) -> Result<()> {
        if self.has_main {
            Ok(())
        } else {
            Err(Diag::new(
                codes::NO_MAIN,
                "this program has no `fn main()`, so there is nothing to run",
                Span::default(),
            ))
        }
    }

    fn push(&mut self) {
        self.scopes.push(Scope::default());
        self.value_types.push(HashMap::new());
    }

    fn pop(&mut self) {
        self.scopes.pop();
        self.value_types.pop();
    }

    fn declare(&mut self, name: &str, mutable: bool, span: Span) -> Result<()> {
        self.declare_inner(name, mutable, span, false)
    }

    /// Declare an ordinary `let`/`let mut` variable binding, which may shadow
    /// an existing binding in the same scope (`LANGUAGE_SPEC.md` §16.3).
    ///
    /// Shadowing installs a *new* binding: this overwrites the current scope's
    /// record so later name resolution sees the new mutability and type, while
    /// the runtime gives it a new binding identity (existing closures keep the
    /// old one). Parameters, pattern bindings, and constants use the strict
    /// [`Checker::declare`], so a duplicate parameter or a duplicate constant
    /// stays `E2007`.
    fn declare_shadowing(&mut self, name: &str, mutable: bool, span: Span) -> Result<()> {
        self.declare_inner(name, mutable, span, true)
    }

    fn declare_inner(
        &mut self,
        name: &str,
        mutable: bool,
        span: Span,
        allow_shadow: bool,
    ) -> Result<()> {
        if crate::lex::KEYWORDS.contains(&name) {
            return Err(Diag::new(
                codes::RESERVED_NAME,
                format!("`{name}` is a reserved word and cannot be used as a name"),
                span,
            ));
        }
        let Some(scope) = self.scopes.last_mut() else {
            return Ok(());
        };
        if !allow_shadow && scope.declares.contains_key(name) {
            return Err(Diag::new(
                codes::REDECLARED,
                format!("`{name}` is already declared in this scope"),
                span,
            ));
        }
        // A shadow records the new declaration's span (the binding identity is
        // the latest declaration); a fresh declaration records its own.
        scope.declares.insert(name.to_string(), span);
        scope.vars.insert(name.to_string(), mutable);
        Ok(())
    }

    /// The annotated type of `name`, if any, innermost scope first.
    fn lookup_type(&self, name: &str) -> Option<Ty> {
        for scope in self.value_types.iter().rev() {
            if let Some(t) = scope.get(name) {
                return Some(t.clone());
            }
        }
        None
    }

    fn lookup(&self, name: &str) -> Option<bool> {
        if crate::stdlib::builtin_names().contains(&name) {
            return Some(false);
        }
        // While checking a constant initializer, only constants declared
        // before it are initialized (source-order semantics).
        if let (Some(active), Some(ord)) = (self.active_const, self.const_order.get(name)) {
            if *ord >= active {
                return None;
            }
        }
        for scope in self.scopes.iter().rev() {
            if let Some(m) = scope.vars.get(name) {
                return Some(*m);
            }
        }
        None
    }

    /// Whether `name` resolves to a specific top-level `fn` declaration.
    ///
    /// Resolution follows lexical scope: the global scope (`scopes[0]`) holds
    /// the hoisted declarations, so `name` denotes a declaration only when no
    /// inner scope shadows it. A user declaration takes precedence over a
    /// builtin of the same name, matching the runtime's call dispatch. This is
    /// the gate for FEATURE_001's static argument check (`LANGUAGE_SPEC.md`
    /// §6.5): the check applies to exactly the calls this predicate accepts.
    fn resolves_to_user_function(&self, name: &str) -> bool {
        self.functions.contains_key(name)
            && !self
                .scopes
                .iter()
                .skip(1)
                .any(|s| s.declares.contains_key(name))
    }

    /// Statically validate a call to a directly resolved top-level function.
    ///
    /// Performs parameter satisfaction (`LANGUAGE_SPEC.md` §15.7): positional
    /// arguments fill the next unfilled parameter, named arguments fill the
    /// parameter with that exact name, and every parameter must be satisfied
    /// exactly once. Then the existing annotated-type check runs against the
    /// resulting mapping. Only provable mismatches are rejected: an `Unknown`
    /// argument is accepted, and an unannotated parameter imposes no type
    /// constraint.
    ///
    /// Evaluation order is not a concern here: the checker inspects types, not
    /// runtime values. The runtime binds values after evaluating them in
    /// source order (see `run::Interp::eval_call`).
    ///
    /// Resolve a call to a function overload set, returning the index of the
    /// selected overload. Uses the shared selector
    /// (`crate::types::resolve_overload`) so the checker and runtime agree.
    ///
    /// Arguments are mapped to each candidate's parameters: a positional
    /// argument fills the next unfilled parameter; a named argument fills its
    /// parameter by name. A candidate whose parameter names cannot accept the
    /// argument names is not viable. Returns `None` when no candidate matches
    /// or the call is ambiguous (the caller reports the diagnostic).
    fn resolve_call_sig(&self, set: &[FnSig], args: &[Arg]) -> Option<usize> {
        if args.iter().all(|a| a.name.is_none()) {
            let candidates: Vec<crate::types::OverloadParams> = set
                .iter()
                .map(|s| s.params.iter().map(|p| p.ty.clone()).collect())
                .collect();
            let actual: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
            return match crate::types::resolve_overload(&candidates, &actual) {
                crate::types::OverloadResolution::Selected(i) => Some(i),
                _ => None,
            };
        }
        // Named arguments: map per candidate, then pick the most specific.
        let mut best: Option<(usize, u32)> = None;
        let mut tie = false;
        for (index, sig) in set.iter().enumerate() {
            let Some(actual) = self.map_args_to_params(&sig.params, args) else {
                continue;
            };
            let score: u32 = sig
                .params
                .iter()
                .zip(&actual)
                .map(|(p, t)| match &p.ty {
                    Some(e) if e == t => 2,
                    Some(_) => 1,
                    None => 0,
                })
                .sum();
            match best {
                Some((_, s)) if score < s => {}
                Some((_, s)) if score == s => tie = true,
                _ => {
                    best = Some((index, score));
                    tie = false;
                }
            }
        }
        match best {
            Some((i, _)) if !tie => Some(i),
            _ => None,
        }
    }

    /// Build the substitution that binds a generic signature's parameters by
    /// structurally matching each actual argument type against the parameter's
    /// declared type. Returns `None` when a concrete constraint is violated (a
    /// mismatch), and binds what it can otherwise. A parameter with no binding
    /// is left unbound; the caller maps it to `Unknown` for value positions.
    ///
    /// Matching is one-way and deterministic: a bare parameter binds to the
    /// actual type, a compound pattern matches the same compound shape, and a
    /// concrete type requires compatibility.
    fn infer_substitution(
        &self,
        params: &[ParamSig],
        actual: &[Ty],
        explicit: &HashMap<String, Ty>,
    ) -> Option<HashMap<String, Ty>> {
        let mut sigma: HashMap<String, Ty> = explicit.clone();
        for (p, a) in params.iter().zip(actual) {
            let Some(expected) = &p.ty else { continue };
            if !self.unify_into(expected, a, &mut sigma) {
                return None;
            }
        }
        Some(sigma)
    }

    /// Structurally match `pattern` against `actual`, recording parameter
    /// bindings in `sigma`. Returns `false` on a provable mismatch. Two
    /// occurrences of one parameter must agree.
    fn unify_into(&self, pattern: &Ty, actual: &Ty, sigma: &mut HashMap<String, Ty>) -> bool {
        // `Unknown` never forces a binding and never fails.
        if matches!(actual, Ty::Unknown) {
            return true;
        }
        match pattern {
            Ty::Param(name) => match sigma.get(name) {
                Some(bound) => bound.compatible_with(actual) && actual.compatible_with(bound),
                None => {
                    sigma.insert(name.clone(), actual.clone());
                    true
                }
            },
            Ty::List(p) => match actual {
                Ty::List(a) => self.unify_into(p, a, sigma),
                // A runtime container carries `[Unknown]`; keep the parameter
                // unbound rather than guessing (it stays `Unknown`).
                _ => pattern.compatible_with(actual),
            },
            Ty::Map(pk, pv) => match actual {
                Ty::Map(ak, av) => self.unify_into(pk, ak, sigma) && self.unify_into(pv, av, sigma),
                _ => pattern.compatible_with(actual),
            },
            Ty::Union(members) => {
                // A union pattern like `T | none` collapses to `Unknown` before
                // it reaches here. For a genuine union, match when some member
                // accepts the actual type.
                members.iter().any(|m| self.unify_into(m, actual, sigma))
            }
            Ty::App(name, args) => match actual {
                Ty::App(an, aargs) if an == name && aargs.len() == args.len() => args
                    .iter()
                    .zip(aargs)
                    .all(|(p, a)| self.unify_into(p, a, sigma)),
                _ => pattern.compatible_with(actual),
            },
            // A concrete pattern requires compatibility; a parameter nested in
            // a union or container has already been handled above.
            _ => pattern.compatible_with(actual),
        }
    }

    /// Seed a substitution from explicit type arguments written at a call site.
    /// Returns `E3001` for a wrong arity or an unknown parameter name.
    fn explicit_substitution(
        &self,
        name: &str,
        sig: &FnSig,
        ty_args: &[TypeExpr],
        span: Span,
    ) -> Result<HashMap<String, Ty>> {
        if ty_args.is_empty() {
            return Ok(HashMap::new());
        }
        if ty_args.len() != sig.type_params.len() {
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!(
                    "`{name}` expects {} type argument(s), found {}",
                    sig.type_params.len(),
                    ty_args.len()
                ),
                span,
            ));
        }
        let mut sigma = HashMap::new();
        for ((pname, pbounds), t) in sig.type_params.iter().zip(ty_args) {
            let ty = self.annotation(t, span)?;
            if ty.has_param() {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "type argument for `{pname}` must be a concrete type, found `{}`",
                        ty.name()
                    ),
                    span,
                ));
            }
            self.satisfies_bounds(&ty, pbounds, span)?;
            sigma.insert(pname.clone(), ty);
        }
        Ok(sigma)
    }

    /// Map arguments to a candidate's parameters, returning the actual types in
    /// parameter order, or `None` when the names/arity do not fit.
    fn map_args_to_params(&self, params: &[ParamSig], args: &[Arg]) -> Option<Vec<Ty>> {
        if args.len() != params.len() {
            return None;
        }
        let mut slots: Vec<Option<Ty>> = vec![None; params.len()];
        let mut next = 0usize;
        for arg in args {
            let idx = match &arg.name {
                None => {
                    while next < params.len() && slots[next].is_some() {
                        next += 1;
                    }
                    if next >= params.len() {
                        return None;
                    }
                    let i = next;
                    next += 1;
                    i
                }
                Some(n) => params.iter().position(|p| &p.name == n)?,
            };
            if slots[idx].is_some() {
                return None;
            }
            slots[idx] = Some(self.infer(&arg.value));
        }
        slots.into_iter().collect()
    }

    /// A deterministic diagnostic for a call that no overload accepts, or that
    /// is ambiguous. The argument types are listed so the report is
    /// reproducible regardless of hash order.
    fn no_overload_diag(&self, name: &str, set: &[FnSig], args: &[Arg], span: Span) -> Diag {
        let actual = args
            .iter()
            .map(|a| match &a.name {
                Some(n) => format!("{n}: {}", self.infer(&a.value).name()),
                None => self.infer(&a.value).name().clone(),
            })
            .collect::<Vec<_>>()
            .join(", ");
        // Re-run the shared selector to distinguish ambiguity from no-match.
        let candidates: Vec<crate::types::OverloadParams> = set
            .iter()
            .map(|s| s.params.iter().map(|p| p.ty.clone()).collect())
            .collect();
        let actual_tys: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
        let message = match crate::types::resolve_overload(&candidates, &actual_tys) {
            crate::types::OverloadResolution::Ambiguous(tied) => format!(
                "call to `{name}` is ambiguous: {} overloads accept ({actual})",
                tied.len()
            ),
            _ => format!(
                "no overload of `{name}` accepts ({actual}); {} candidate(s) declared",
                set.len()
            ),
        };
        Diag::new(codes::TYPE_MISMATCH, message, span)
    }

    /// The method analogue of [`Checker::no_overload_diag`]: a deterministic
    /// diagnostic when no method overload accepts the arguments.
    fn no_method_overload_diag(
        &self,
        struct_name: &str,
        name: &str,
        set: &[FnSig],
        args: &[Arg],
        span: Span,
    ) -> Diag {
        let actual = args
            .iter()
            .map(|a| self.infer(&a.value).name().clone())
            .collect::<Vec<_>>()
            .join(", ");
        let candidates: Vec<crate::types::OverloadParams> = set
            .iter()
            .map(|s| s.params[1..].iter().map(|p| p.ty.clone()).collect())
            .collect();
        let actual_tys: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
        let message = match crate::types::resolve_overload(&candidates, &actual_tys) {
            crate::types::OverloadResolution::Ambiguous(tied) => format!(
                "call to `{struct_name}.{name}` is ambiguous: {} overloads accept ({actual})",
                tied.len()
            ),
            _ => format!(
                "no overload of `{struct_name}.{name}` accepts ({actual}); {} candidate(s) declared",
                set.len()
            ),
        };
        Diag::new(codes::TYPE_MISMATCH, message, span)
    }

    fn check_user_call(
        &self,
        name: &str,
        sig: &FnSig,
        args: &[Arg],
        ty_args: &[TypeExpr],
        span: Span,
    ) -> Result<()> {
        // Map each declared parameter index to the argument index that fills
        // it, or `None` if it is missing.
        let mut filled: Vec<Option<usize>> = vec![None; sig.params.len()];
        let mut next_positional = 0usize;
        for (arg_index, arg) in args.iter().enumerate() {
            match &arg.name {
                None => {
                    // Positional: next unfilled parameter in declaration order.
                    if next_positional >= sig.params.len() {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "`{name}` expects {} argument(s), got {}",
                                sig.params.len(),
                                args.len()
                            ),
                            span,
                        ));
                    }
                    filled[next_positional] = Some(arg_index);
                    next_positional += 1;
                }
                Some(param_name) => {
                    let Some(param_index) = sig.params.iter().position(|p| &p.name == param_name)
                    else {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!("`{name}` has no parameter named `{param_name}`"),
                            span,
                        ));
                    };
                    if filled[param_index].is_some() {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "parameter `{param_name}` is given more than once for `{name}`"
                            ),
                            span,
                        ));
                    }
                    filled[param_index] = Some(arg_index);
                }
            }
        }

        // Every parameter must be satisfied.
        for (param_index, param) in sig.params.iter().enumerate() {
            if filled[param_index].is_none() {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "missing argument for parameter `{}` of `{name}`",
                        param.name
                    ),
                    span,
                ));
            }
        }

        // Type-check each parameter against the argument mapped to it.
        for (param_index, param) in sig.params.iter().enumerate() {
            let Some(expected) = &param.ty else { continue };
            let Some(arg_index) = filled[param_index] else {
                continue;
            };
            let actual = self.infer(&args[arg_index].value);
            if !expected.compatible_with(&actual) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "`{name}` parameter `{}` expects `{}`, found `{}`",
                        sig.params[param_index].name,
                        expected.name(),
                        actual.name()
                    ),
                    span,
                ));
            }
        }
        // Generic signature: build the substitution from the explicit type
        // arguments (when written) and the argument types, then check every
        // bound the parameters declare.
        if !sig.type_params.is_empty() {
            let explicit = self.explicit_substitution(name, sig, ty_args, span)?;
            let actual: Vec<Ty> = sig
                .params
                .iter()
                .zip(&filled)
                .map(|(_, slot)| slot.map_or(Ty::Unknown, |ai| self.infer(&args[ai].value)))
                .collect();
            let Some(sigma) = self.infer_substitution(&sig.params, &actual, &explicit) else {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!("`{name}` cannot be instantiated with these argument types"),
                    span,
                ));
            };
            for (pname, pbounds) in &sig.type_params {
                if let Some(ty) = sigma.get(pname) {
                    self.satisfies_bounds(ty, pbounds, span)?;
                }
            }
        } else if !ty_args.is_empty() {
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!("`{name}` is not generic and takes no type arguments"),
                span,
            ));
        }
        Ok(())
    }

    /// Reject named arguments on a callable category that does not support
    /// them (built-ins, methods, and dynamic/unknown callees).
    ///
    /// Named arguments require a statically known parameter set
    /// (`LANGUAGE_SPEC.md` §15.7); only directly resolved user functions have
    /// one.
    fn reject_named_args(&self, what: &str, args: &[Arg], span: Span) -> Result<()> {
        if let Some(arg) = args.iter().find(|a| a.name.is_some()) {
            let name = arg.name.as_deref().unwrap_or_default();
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!("`{what}` does not accept named arguments (`{name}: ...`)"),
                span,
            ));
        }
        Ok(())
    }

    fn item(&mut self, item: &Item) -> Result<()> {
        match item {
            Item::Fn {
                name,
                type_params,
                params,
                ret,
                ret_span,
                body,
                ..
            } => {
                // The canonical name encodes the declaring module, so a method
                // call inside this body is visibility-checked relative to it.
                let saved_module =
                    std::mem::replace(&mut self.current_module, canonical_module_of(name));
                // Bind the declaration's generic parameters for the signature
                // and body, so `T` is a placeholder throughout.
                self.push_type_params(type_params);
                if name == "main" {
                    if !params.is_empty() {
                        return Err(Diag::new(
                            codes::INVALID_MAIN,
                            format!(
                                "`main` is the program entry point and takes no arguments, but {} were declared",
                                params.len()
                            ),
                            params[0].span,
                        ));
                    }
                    self.has_main = true;
                }
                // The name is already declared by `hoist`; only the body scope
                // and parameter bindings are introduced here.
                self.push();
                for p in params {
                    // `mut` on a parameter grants mutable capability over the
                    // bound name inside the body (§16.6).
                    self.declare(&p.name, p.mutable, p.span)?;
                    if let Some(pty) = &p.ty {
                        let t = self.annotation(pty, p.span)?;
                        self.value_types
                            .last_mut()
                            .map(|m| m.insert(p.name.clone(), t));
                    }
                }
                let saved_return = self.return_type.clone();
                let saved_underscore = std::mem::take(&mut self.underscore_params);
                let saved_used = std::mem::take(&mut self.used_names);
                self.return_type = match ret {
                    Some(rt) => Some(self.annotation(rt, ret_span.unwrap_or_default())?),
                    None => None,
                };
                for p in params {
                    if p.name.starts_with('_') {
                        self.underscore_params.push((p.name.clone(), p.span));
                    }
                }
                self.block(body)?;
                // Contract: a parameter whose name starts with `_` must be
                // unused. Using it is E2009.
                for (pname, _pspan) in std::mem::take(&mut self.underscore_params) {
                    let use_span = self.used_names.get(&pname).copied();
                    if let Some(use_span) = use_span {
                        self.return_type = saved_return;
                        self.underscore_params = saved_underscore;
                        self.used_names = saved_used;
                        self.current_module = saved_module;
                        self.pop();
                        self.pop_type_params();
                        return Err(Diag::new(
                            codes::UNUSED_PARAM,
                            format!(
                                "parameter `{pname}` starts with `_`, which means it must stay unused; remove the `_` to use it"
                            ),
                            use_span,
                        ));
                    }
                }
                self.return_type = saved_return;
                self.underscore_params = saved_underscore;
                self.used_names = saved_used;
                self.current_module = saved_module;
                self.pop();
                self.pop_type_params();
            }
            Item::Const {
                name,
                ann,
                value,
                span,
                ..
            } => {
                let saved = self.active_const;
                self.active_const = self.const_order.get(name).copied();
                let r = self.expr(value);
                self.active_const = saved;
                r?;
                // Record the constant's type so a member access on it (e.g.
                // `XS.push(...)`, `p.field`) resolves instead of staying
                // `Unknown`. This mirrors a `let` binding's inference and is
                // what lets the mutation-capability check see the receiver.
                if let Some(ann) = ann {
                    let expected = self.annotation(ann, *span)?;
                    let actual = self.infer(value);
                    if !expected.compatible_with(&actual) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "`{name}` is annotated as `{}` but its value is `{}`",
                                expected.name(),
                                actual.name()
                            ),
                            *span,
                        ));
                    }
                    self.value_types
                        .last_mut()
                        .map(|m| m.insert(name.clone(), expected));
                } else {
                    let inferred = self.infer(value);
                    if !matches!(inferred, Ty::Unknown) {
                        self.value_types
                            .last_mut()
                            .map(|m| m.insert(name.clone(), inferred));
                    }
                }
            }
            Item::Expr(e, _) => self.expr(e)?,
            Item::Struct { .. }
            | Item::Enum { .. }
            | Item::Alias { .. }
            | Item::Use { .. }
            | Item::Trait { .. }
            | Item::Module { .. } => {}
            Item::Impl {
                target,
                target_args,
                type_params,
                methods,
                ..
            } => {
                // The method table was built in the hoist pass; here each
                // method body is checked with `self` typed as the receiver's
                // nominal struct, under the ordinary function rules. A generic
                // `impl<T> Struct<T>` binds `T` for every method body, and the
                // receiver type carries the parameters so `self.field` on a
                // parameterised field substitutes correctly.
                self.push_type_params(type_params);
                let recv_ty = if target_args.is_empty() {
                    Ty::Named(target.clone())
                } else {
                    let mut tys = Vec::with_capacity(target_args.len());
                    for a in target_args {
                        // An argument that names an impl parameter is a
                        // `Ty::Param`; a concrete one resolves normally.
                        tys.push(match a {
                            TypeExpr::Named(n) if self.is_type_param(n) => Ty::Param(n.clone()),
                            other => self.annotation(other, Span::default())?,
                        });
                    }
                    Ty::App(target.clone(), tys)
                };
                for m_item in methods {
                    let Item::Fn {
                        name,
                        type_params: method_type_params,
                        params,
                        ret,
                        ret_span,
                        body,
                        ..
                    } = m_item
                    else {
                        continue;
                    };
                    // A method may add its own parameters on top of the impl's.
                    self.push_type_params(method_type_params);
                    self.check_method_body(
                        target,
                        name,
                        &recv_ty,
                        params,
                        ret.as_ref().map(|rt| (rt, ret_span.unwrap_or_default())),
                        body,
                    )?;
                    self.pop_type_params();
                }
                self.pop_type_params();
            }
        }
        Ok(())
    }

    /// Check one method body. Identical to a top-level function except that
    /// the receiver parameter's type is the nominal struct `target` (so
    /// `self.field` and `self.other_method(...)` resolve) and the method has
    /// no declaration in the global function namespace.
    fn check_method_body(
        &mut self,
        target: &str,
        _name: &str,
        recv_ty: &Ty,
        params: &[Param],
        ret: Option<(&TypeExpr, Span)>,
        body: &[Stmt],
    ) -> Result<()> {
        // A method body is checked relative to the module the struct is
        // declared in (its canonical name carries the path), so calling a
        // private method of the same module is allowed and calling a foreign
        // private one is not.
        let saved_module = std::mem::replace(&mut self.current_module, canonical_module_of(target));
        self.push();
        for (i, p) in params.iter().enumerate() {
            // The receiver's mutability comes from `mut self`; an ordinary
            // parameter's from `mut name`. This grants the body mutable
            // capability over the bound name (§16.6).
            self.declare(&p.name, p.mutable, p.span)?;
            // The receiver `self` is typed as the nominal struct. An explicit
            // receiver annotation, if written, must still name that struct.
            if i == 0 {
                self.value_types
                    .last_mut()
                    .map(|m| m.insert(p.name.clone(), recv_ty.clone()));
            } else if let Some(pty) = &p.ty {
                let t = self.annotation(pty, p.span)?;
                self.value_types
                    .last_mut()
                    .map(|m| m.insert(p.name.clone(), t));
            }
        }
        let saved_return = self.return_type.clone();
        let saved_underscore = std::mem::take(&mut self.underscore_params);
        let saved_used = std::mem::take(&mut self.used_names);
        self.return_type = match ret {
            Some((rt, span)) => Some(self.annotation(rt, span)?),
            None => None,
        };
        for p in params {
            if p.name.starts_with('_') {
                self.underscore_params.push((p.name.clone(), p.span));
            }
        }
        let r = self.block(body);
        r?;
        // The receiver is a parameter; using `self` is expected, so it is
        // never treated as an unused `_` parameter. The remaining `_`-prefixed
        // parameters keep the ordinary contract.
        for (pname, _pspan) in std::mem::take(&mut self.underscore_params) {
            if pname == "self" {
                continue;
            }
            if let Some(use_span) = self.used_names.get(&pname).copied() {
                self.return_type = saved_return;
                self.underscore_params = saved_underscore;
                self.used_names = saved_used;
                self.current_module = saved_module;
                self.pop();
                return Err(Diag::new(
                    codes::UNUSED_PARAM,
                    format!(
                        "parameter `{pname}` starts with `_`, which means it must stay unused; remove the `_` to use it"
                    ),
                    use_span,
                ));
            }
        }
        self.return_type = saved_return;
        self.underscore_params = saved_underscore;
        self.used_names = saved_used;
        self.current_module = saved_module;
        self.pop();
        Ok(())
    }

    /// Validate a pattern: every variant pattern names a declared variant,
    /// and a pattern never binds the same name twice.
    fn check_pattern(&self, pat: &Pattern) -> Result<()> {
        match pat {
            Pattern::Variant(tag, ps, span) => {
                // A variant pattern MUST name a declared runtime variant tag.
                // A struct name, enum type name, or alias name is not a
                // variant and must not be accepted: the runtime matches only
                // `Value::Variant` by tag, so accepting a type name would let
                // `match` silently fall through (`LANGUAGE_SPEC.md` §19.3).
                if !self.variants.contains_key(tag) {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("`{tag}` is not a declared enum variant"),
                        *span,
                    ));
                }
                for p in ps {
                    self.check_pattern(p)?;
                }
            }
            Pattern::List(ps, _) => {
                for p in ps {
                    self.check_pattern(p)?;
                }
            }
            _ => {}
        }
        let mut seen: Vec<String> = Vec::new();
        for b in pat.bindings() {
            if seen.contains(&b) {
                return Err(Diag::new(
                    codes::DUPLICATE_BINDING,
                    format!("`{b}` is bound more than once in the same pattern"),
                    pat.span(),
                ));
            }
            seen.push(b);
        }
        Ok(())
    }

    /// Infer the type of an expression, conservatively. Returns
    /// [`Ty::Unknown`] whenever the checker cannot be certain.
    fn infer(&self, e: &Expr) -> Ty {
        match e {
            Expr::Lit(l, _) => match l {
                Lit::Int(_) => Ty::Int,
                Lit::Float(_) => Ty::Float,
                Lit::Str(_) => Ty::String,
                Lit::Bool(_) => Ty::Bool,
                Lit::None => Ty::Unknown,
            },
            Expr::FStr(..) => Ty::String,
            // A parenthesized comma-list is list sugar (§21): it is
            // indistinguishable from a list literal, so it infers the same
            // list type. (It previously inferred `Unknown`, which let a
            // `(1, 2)` pass an annotation that `[1, 2]` would be rejected
            // for, contradicting the "indistinguishable" rule.)
            Expr::List(items, _) | Expr::Tuple(items, _) => {
                // Infer the element type as the union of every statically known
                // element, using the language's existing union rule (§5.2). A
                // first-known-element rule hid provable mismatches: `[1, "x"]`
                // inferred `[int]`, so `let xs: [int] = [1, "x"]` was accepted.
                // `Unknown` elements impose no constraint, so a list of only
                // `Unknown` stays `[Unknown]`. A parenthesized comma-list is
                // list sugar and infers identically.
                let elems: Vec<Ty> = items
                    .iter()
                    .map(|item| self.infer(item))
                    .filter(|t| !matches!(t, Ty::Unknown))
                    .collect();
                let elem = if elems.is_empty() {
                    Ty::Unknown
                } else {
                    Ty::union(elems)
                };
                Ty::List(Box::new(elem))
            }
            Expr::Map(entries, _) => {
                // Infer each dimension as the union of the entries' static
                // types, using the language's existing union rule (§5.2). A
                // first-entry-wins rule mis-typed a heterogeneous literal: it
                // reported `{int: string}` for `{1: "a", "1": "b"}` while the
                // value actually held a string key, and that wrong type then
                // flowed into `keys()`/`values()`. `Unknown` members are
                // skipped (they impose no constraint), so an empty map stays
                // `{Unknown: Unknown}` and remains compatible with any map
                // annotation. A non-key-capable key is rejected in `expr`, not
                // here, where the span is available.
                let mut keys = Vec::new();
                let mut vals = Vec::new();
                for (k, v) in entries {
                    let kt = self.infer(k);
                    if kt.is_key_capable() && !matches!(kt, Ty::Unknown) {
                        keys.push(kt);
                    }
                    let vt = self.infer(v);
                    if !matches!(vt, Ty::Unknown) {
                        vals.push(vt);
                    }
                }
                let key = if keys.is_empty() {
                    Ty::Unknown
                } else {
                    Ty::union(keys)
                };
                let val = if vals.is_empty() {
                    Ty::Unknown
                } else {
                    Ty::union(vals)
                };
                Ty::Map(Box::new(key), Box::new(val))
            }
            Expr::Construct(name, args, ty_args, _) => {
                if self.variants.contains_key(name) {
                    Ty::Unknown
                } else if self
                    .type_type_params
                    .get(name)
                    .is_some_and(|p| !p.is_empty())
                {
                    // A generic struct: build the substitution from the explicit
                    // arguments or the field values, and apply it to the type.
                    self.construct_generic_ty(name, args, ty_args)
                } else {
                    Ty::Named(name.clone())
                }
            }
            Expr::Unary(UnOp::Not, _, _) => Ty::Bool,
            Expr::Unary(UnOp::Neg, inner, _) => self.infer(inner),
            Expr::Unary(UnOp::BitNot, inner, _) => self.infer(inner),
            Expr::Binary(op, l, r, _) => match op {
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => Ty::Bool,
                // Arithmetic and bitwise results have their own result type,
                // not the left operand's (LANGUAGE_SPEC.md §9.1). Returning
                // `infer(l)` made `1 + 1.5` infer `int`, so
                // `let x: int = 1 + 1.5` was accepted while `f(1 + 1.5)` for
                // `f(float)` was wrongly rejected.
                BinOp::Add => self.infer_add(l, r),
                BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Rem | BinOp::Pow => {
                    self.infer_numeric(l, r)
                }
                BinOp::BitAnd | BinOp::BitOr | BinOp::Shl | BinOp::Shr => {
                    let lt = self.infer(l);
                    let rt = self.infer(r);
                    match (&lt, &rt) {
                        (Ty::Int, Ty::Int) => Ty::Int,
                        // Unknown is permissive; a provably non-int operand
                        // stays Unknown here and is a runtime `E3001`.
                        _ => Ty::Unknown,
                    }
                }
            },
            Expr::If(_, _, _, _) | Expr::Match(_, _, _) | Expr::Block(_, _) => Ty::Unknown,
            Expr::Name(name, _) => {
                for scope in self.value_types.iter().rev() {
                    if let Some(t) = scope.get(name) {
                        return t.clone();
                    }
                }
                Ty::Unknown
            }
            Expr::Call(f, args, ty_args, _) => match f.as_ref() {
                Expr::Name(name, _) => {
                    if let Some(set) = self.functions.get(name) {
                        let visible: Vec<FnSig> =
                            set.iter().filter(|s| self.fn_visible(s)).cloned().collect();
                        self.resolve_call_sig(&visible, args)
                            .map_or(Ty::Unknown, |i| {
                                self.instantiate_return_ty(&visible[i], args, ty_args)
                            })
                    } else if let Some(sig) = crate::stdlib::signatures::builtin(name) {
                        sig.returns.ty()
                    } else {
                        Ty::Unknown
                    }
                }
                _ => Ty::Unknown,
            },
            Expr::Method(recv, name, args, ty_args, _) => {
                let recv_ty = self.infer(recv);
                if let Some(sname) = self.struct_name_of(&recv_ty) {
                    if let Some(set) = self.method_set(&sname, name) {
                        let visible: Vec<FnSig> = set
                            .iter()
                            .filter(|s| self.method_visible(s))
                            .cloned()
                            .collect();
                        if let Some(i) = self.resolve_method_sig(&visible, args) {
                            return self.instantiate_return_ty(&visible[i], args, ty_args);
                        }
                    }
                    return Ty::Unknown;
                }
                if let Some(class) = recv_ty.type_class() {
                    if let Some(sig) = crate::stdlib::signatures::method(class, name) {
                        // For a statically known map receiver, `keys`/`values`
                        // carry the map's key and value types rather than the
                        // registry's coarse `[string]`. `get`/`remove` stay
                        // dynamic: they return `none` for a missing key, and
                        // Aura has no static `none` type, so claiming `V` would
                        // be a lie (LANGUAGE_SPEC §18).
                        if let Ty::Map(k, v) = &recv_ty {
                            match name.as_str() {
                                "keys" => return Ty::List(k.clone()),
                                "values" => return Ty::List(v.clone()),
                                _ => {}
                            }
                        }
                        // `sort`/`reverse` on a known list return a list of the
                        // same element type; the registry cannot express that.
                        if let Ty::List(elem) = &recv_ty {
                            if matches!(name.as_str(), "sort" | "reverse") {
                                return Ty::List(elem.clone());
                            }
                        }
                        return sig.returns.ty();
                    }
                }
                Ty::Unknown
            }
            Expr::Field(recv, name, _) => {
                // FEATURE_003: a field read on a statically known struct has
                // that struct's declared field type, with the struct's generic
                // substitution applied. The field table stores alias-resolved
                // types, so the propagated `Ty` is the semantically resolved
                // one. Anything else (an `Unknown` receiver, a primitive, a
                // list, a map, an enum, `range`, or a struct without that
                // field) stays `Unknown` — the checker never speculates
                // (`LANGUAGE_SPEC.md` §17.5).
                let recv_ty = self.infer(recv);
                if let Some(sname) = self.struct_name_of(&recv_ty) {
                    if let Some(fty) = self.struct_fields.get(&sname).and_then(|m| m.get(name)) {
                        let sigma = self.receiver_substitution(&sname, &recv_ty);
                        return fty.substitute(&sigma);
                    }
                }
                Ty::Unknown
            }
            Expr::Range(_, _, _) => Ty::Named("range".to_string()),
            Expr::Index(base, _, _) => {
                // The result of a *successful* indexing expression. A missing
                // map key is the runtime error E2003 (not a `none` value), and
                // an out-of-range list index is E4019, so returning the element
                // type is honest for the value the expression yields when it
                // does not raise.
                match self.infer(base) {
                    Ty::List(elem) => elem.as_ref().clone(),
                    Ty::Map(_, v) => v.as_ref().clone(),
                    Ty::String => Ty::String,
                    _ => Ty::Unknown,
                }
            }
            Expr::Pipe(_, _, _) | Expr::Lambda(_, _, _) => Ty::Unknown,
        }
    }

    /// Infer the result type of a numeric binary operator (`-`, `*`, `/`, `%`,
    /// `^`), which §9.1 defines only for numbers. `int` with `int` is `int`;
    /// any `float` operand makes the result `float`; anything else is
    /// `Unknown` (the runtime reports the type error).
    fn infer_numeric(&self, l: &Expr, r: &Expr) -> Ty {
        match (self.infer(l), self.infer(r)) {
            (Ty::Int, Ty::Int) => Ty::Int,
            (Ty::Float, Ty::Float) | (Ty::Float, Ty::Int) | (Ty::Int, Ty::Float) => Ty::Float,
            _ => Ty::Unknown,
        }
    }

    /// Infer the result type of `+`, which §9.1 defines for `int`, `float`,
    /// `string`, and `list`. Two equal list operands yield a list of the same
    /// element type; a `string` with a `string` yields a `string`; numeric
    /// operands promote like [`Self::infer_numeric`]. Anything else is
    /// `Unknown`.
    fn infer_add(&self, l: &Expr, r: &Expr) -> Ty {
        let (lt, rt) = (self.infer(l), self.infer(r));
        match (&lt, &rt) {
            (Ty::Int, Ty::Int) => Ty::Int,
            (Ty::Float, Ty::Float) | (Ty::Float, Ty::Int) | (Ty::Int, Ty::Float) => Ty::Float,
            (Ty::String, Ty::String) => Ty::String,
            (Ty::List(a), Ty::List(b)) if **a == **b => Ty::List(a.clone()),
            _ => Ty::Unknown,
        }
    }

    /// With explicit type arguments on a struct construction, check each field
    /// value against the field type with the substitution applied, so
    /// `Box<int> { value: "x" }` is rejected (`E3001`).
    fn check_construct_field_types(
        &self,
        name: &str,
        fields: &HashMap<String, Ty>,
        args: &[Arg],
        sigma: &HashMap<String, Ty>,
        span: Span,
    ) -> Result<()> {
        let order = self
            .struct_field_order
            .get(name)
            .cloned()
            .unwrap_or_default();
        for (i, arg) in args.iter().enumerate() {
            let fname = match &arg.name {
                Some(n) => n.clone(),
                None => match order.get(i) {
                    Some(n) => n.clone(),
                    None => continue,
                },
            };
            let Some(fty) = fields.get(&fname) else {
                continue;
            };
            let expected = fty.substitute(sigma);
            let actual = self.infer(&arg.value);
            if !expected.compatible_with(&actual) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "field `{fname}` of `{name}` expects `{}`, found `{}`",
                        expected.name(),
                        actual.name()
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }

    /// Validate a builtin call against its shared signature.
    /// The message for a value that cannot be stored in a container whose
    /// element type the checker can name. Used by list `push` and indexed
    /// assignment so a provable mismatch is rejected rather than hidden by the
    /// coarse registry.
    fn check_element_assignable(&self, expected: &Ty, actual: &Ty, span: Span) -> Result<()> {
        if !matches!(actual, Ty::Unknown) && !expected.compatible_with(actual) {
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!(
                    "list has element type `{}` but the value is `{}`",
                    expected.name(),
                    actual.name()
                ),
                span,
            ));
        }
        Ok(())
    }

    fn check_builtin_call(
        &self,
        sig: &crate::stdlib::signatures::Signature,
        args: &[Arg],
        span: Span,
    ) -> Result<()> {
        if let Some(message) = sig.check_arity(args.len()) {
            return Err(Diag::new(codes::TYPE_MISMATCH, message, span));
        }
        // The free `push(list, v)` builtin carries the same element contract as
        // the method form; the coarse registry cannot name the element type.
        if sig.name == "push" {
            if let (Some(list), Some(value)) = (args.first(), args.get(1)) {
                if let Ty::List(elem) = self.infer(&list.value) {
                    self.check_element_assignable(&elem, &self.infer(&value.value), span)?;
                }
            }
        }
        for (i, param) in sig.params.iter().enumerate() {
            let Some(arg) = args.get(i) else { break };
            let actual = self.infer(&arg.value);
            if param.accepts.accepts_ty(&actual) == Some(false) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "`{}` argument {} expects {}, found `{}`",
                        sig.name,
                        i + 1,
                        param.accepts.describe(),
                        actual.name()
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }

    /// The method table a receiver type uses, or a decision that it has none.
    ///
    /// Returns `Ok(None)` when the receiver type is `Unknown` and nothing can
    /// be decided (§2.3). Returns `Err` when the receiver type is known to
    /// have no methods at all (a user struct or enum). Otherwise returns the
    /// method table class; `range` uses the `Other` table.
    fn method_class_for(
        &self,
        name: &str,
        ty: &Ty,
        span: Span,
    ) -> Result<Option<crate::stdlib::signatures::TypeClass>> {
        use crate::stdlib::signatures::TypeClass;
        match ty {
            Ty::Unknown => Ok(None),
            Ty::Enum(_) => Err(Diag::new(
                codes::UNDEFINED,
                format!("enum has no method `{name}`"),
                span,
            )),
            Ty::Named(n) if n == "range" => Ok(Some(TypeClass::Range)),
            Ty::Named(_) => Err(Diag::new(
                codes::UNDEFINED,
                format!("struct has no method `{name}`"),
                span,
            )),
            other => Ok(other.type_class()),
        }
    }

    /// The nominal struct name of a statically known receiver, if it is a
    /// struct. `range` is a built-in (not a struct); enums and non-nominal
    /// primitives are not structs.
    fn struct_name_of(&self, ty: &Ty) -> Option<String> {
        match ty {
            Ty::Named(n) if n != "range" => Some(n.clone()),
            Ty::App(n, _) if n != "range" => Some(n.clone()),
            _ => None,
        }
    }

    /// The type arguments applied to a checker type, if any (`Box<int>` → the
    /// vector `[int]`; a bare `Named` → empty).
    fn ty_args_of(ty: &Ty) -> Vec<Ty> {
        match ty {
            Ty::App(_, args) => args.clone(),
            _ => Vec::new(),
        }
    }

    /// Compute the checker type of a generic struct construction, applying the
    /// substitution inferred from the explicit type arguments or the field
    /// values. An unbound parameter yields `Unknown` for that argument rather
    /// than a guessed type.
    fn construct_generic_ty(&self, name: &str, args: &[Arg], ty_args: &[TypeExpr]) -> Ty {
        let declared = self.type_type_params.get(name).cloned().unwrap_or_default();
        if declared.is_empty() {
            return Ty::Named(name.to_string());
        }
        let mut sigma: HashMap<String, Ty> = HashMap::new();
        if !ty_args.is_empty() {
            // Arity is validated by the construction checker; bind what lines up.
            for (p, t) in declared.iter().zip(ty_args) {
                if let Ok(t) = self.annotation(t, Span::default()) {
                    sigma.insert(p.clone(), t);
                }
            }
        } else if let Some(fields) = self.struct_fields.get(name) {
            // Infer from field values, positionally or by name.
            let ordered: Vec<(String, Ty)> = self
                .struct_field_order
                .get(name)
                .map(|order| {
                    order
                        .iter()
                        .map(|f| {
                            let ty = fields.get(f).cloned().unwrap_or(Ty::Unknown);
                            (f.clone(), ty)
                        })
                        .collect()
                })
                .unwrap_or_default();
            for (i, arg) in args.iter().enumerate() {
                let fty = match &arg.name {
                    Some(n) => ordered.iter().find(|(f, _)| f == n).map(|(_, t)| t.clone()),
                    None => ordered.get(i).map(|(_, t)| t.clone()),
                };
                if let Some(fty) = fty {
                    let actual = self.infer(&arg.value);
                    let _ = self.unify_into(&fty, &actual, &mut sigma);
                }
            }
        }
        let applied: Vec<Ty> = declared
            .iter()
            .map(|p| sigma.get(p).cloned().unwrap_or(Ty::Unknown))
            .collect();
        Ty::App(name.to_string(), applied)
    }

    /// The substitution mapping a struct's declared parameters to the type
    /// arguments of a receiver type. A bare `Named` receiver yields no
    /// bindings (the fields are then used as written).
    fn receiver_substitution(&self, struct_name: &str, recv: &Ty) -> HashMap<String, Ty> {
        let params = self
            .type_type_params
            .get(struct_name)
            .cloned()
            .unwrap_or_default();
        let args = Self::ty_args_of(recv);
        let mut sigma = HashMap::new();
        for (p, a) in params.iter().zip(args) {
            sigma.insert(p.clone(), a);
        }
        sigma
    }

    /// Instantiate a signature's return type for a call: build the substitution
    /// from the explicit type arguments or the argument types, then substitute
    /// it into the declared return type. An unbound parameter in the return
    /// becomes `Unknown`; it never becomes a guessed concrete type.
    fn instantiate_return_ty(&self, sig: &FnSig, args: &[Arg], ty_args: &[TypeExpr]) -> Ty {
        let Some(ret) = &sig.ret else {
            return Ty::Unknown;
        };
        if !ret.has_param() {
            return ret.clone();
        }
        let mut sigma: HashMap<String, Ty> = HashMap::new();
        if !ty_args.is_empty() {
            for ((p, _), t) in sig.type_params.iter().zip(ty_args) {
                if let Ok(t) = self.annotation(t, Span::default()) {
                    sigma.insert(p.clone(), t);
                }
            }
        }
        // Re-derive bindings from the (receiver-included) parameter/argument
        // pairs, so a return-only parameter is bound by the arguments.
        let actual: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
        // The receiver is not among `args` for a method, so bind from the
        // declared parameters offset appropriately: the trailing N parameters
        // correspond to the args.
        let n_params = sig.params.len();
        let offset = n_params.saturating_sub(actual.len());
        let mut pseudo: Vec<ParamSig> = Vec::new();
        for (i, p) in sig.params.iter().enumerate().skip(offset) {
            if i - offset < actual.len() {
                pseudo.push(p.clone());
            }
        }
        let _ = self.infer_substitution(&pseudo, &actual, &sigma);
        ret.erase_params(&sigma)
    }

    /// The overload set of a method on a struct, if declared.
    fn method_set(&self, struct_name: &str, method: &str) -> Option<&Vec<FnSig>> {
        self.struct_methods.get(struct_name)?.get(method)
    }

    /// Whether a method is reachable from the current module: it is `pub`, or
    /// the current module **is** the declaring module or a descendant
    /// (`LANGUAGE_SPEC.md` §28). A private method of another module is not
    /// nameable, exactly like a private free function.
    fn method_visible(&self, sig: &FnSig) -> bool {
        sig.public || is_descendant_module(&self.current_module, &sig.owner)
    }

    /// Whether a free function overload is reachable from the current module.
    /// A root-level function has an empty owner, so it is always visible.
    fn fn_visible(&self, sig: &FnSig) -> bool {
        sig.owner.is_empty() || sig.public || is_descendant_module(&self.current_module, &sig.owner)
    }

    /// Whether a struct field is reachable from the current module. A field is
    /// `pub`, or the current module **is** the struct's declaring module or a
    /// descendant. A field of an unknown struct (the type is `Unknown` or a
    /// built-in) is always treated as visible; this is about user structs.
    fn field_visible(&self, struct_name: &str, field: &str) -> bool {
        let owner = canonical_module_of(struct_name);
        let Some(public) = self.struct_public_fields.get(struct_name) else {
            return true;
        };
        // A field the struct does not declare at all is left to the ordinary
        // missing-member handling, not reported as a visibility error.
        let Some(is_pub) = public.get(field) else {
            return true;
        };
        *is_pub || is_descendant_module(&self.current_module, &owner)
    }

    /// Resolve a method overload by its non-receiver argument types. The
    /// receiver is not part of the argument list; candidates are compared on
    /// their parameters after the receiver.
    /// Resolve a method overload among the **visible** ones. An inaccessible
    /// overload is never a candidate, so it cannot be selected and cannot
    /// perturb the resolution of an accessible one (`LANGUAGE_SPEC.md` §28).
    fn resolve_visible_method_sig(&self, set: &[FnSig], args: &[Arg]) -> Option<usize> {
        let visible: Vec<FnSig> = set
            .iter()
            .filter(|s| self.method_visible(s))
            .cloned()
            .collect();
        self.resolve_method_sig(&visible, args)
    }

    /// Whether the method name exists on the struct but only in inaccessible
    /// overloads — so a call is `E2018`, not `E2003`.
    fn method_exists_but_private(&self, set: &[FnSig]) -> bool {
        set.iter().any(|s| !self.method_visible(s))
    }

    fn resolve_method_sig(&self, set: &[FnSig], args: &[Arg]) -> Option<usize> {
        let actual: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
        let candidates: Vec<crate::types::OverloadParams> = set
            .iter()
            .map(|s| s.params[1..].iter().map(|p| p.ty.clone()).collect())
            .collect();
        match crate::types::resolve_overload(&candidates, &actual) {
            crate::types::OverloadResolution::Selected(i) => Some(i),
            _ => None,
        }
    }

    /// Resolve the root binding of a place expression, if it is a simple
    /// binding reached through field/index projections. Returns the root name
    /// and its span. Anything else (a literal, a call result, a composite)
    /// has no binding, so mutation capability cannot apply.
    fn place_root(expr: &Expr) -> Option<(&str, Span)> {
        match expr {
            Expr::Name(n, s) => Some((n.as_str(), *s)),
            Expr::Field(b, _, _) | Expr::Index(b, _, _) => Self::place_root(b),
            _ => None,
        }
    }

    /// Enforce that a mutation reached through `place` has mutable capability:
    /// the root binding must be declared `mut` (`LANGUAGE_SPEC.md` §16.6).
    /// A place with no root binding (a temporary or a call result) is always
    /// mutable, since no caller-visible binding protects it.
    fn require_mutable_place(&self, place: &Expr, what: &str) -> Result<()> {
        let Some((name, span)) = Self::place_root(place) else {
            return Ok(());
        };
        match self.lookup(name) {
            Some(true) => Ok(()),
            Some(false) => Err(Diag::new(
                codes::ASSIGN_IMMUTABLE,
                format!("cannot {what} `{name}`: it is immutable (declare it `let mut`)"),
                span,
            )),
            // An unknown root is reported by the ordinary expression check;
            // do not duplicate that diagnostic here.
            None => Ok(()),
        }
    }

    /// Whether any declared struct has a method with this name. Used to keep
    /// an `Unknown`-receiver call conservative without gating it by the
    /// built-in registry (a user method may resolve at runtime).
    fn user_method_exists_anywhere(&self, method: &str) -> bool {
        self.struct_methods
            .values()
            .any(|table| table.contains_key(method))
    }

    /// Whether two methods of different union members are call-compatible:
    /// the same arity and mutually compatible parameter and return types. The
    /// receiver (`params[0]`) differs by member, so only the remaining
    /// parameters and the return are compared.
    fn method_sigs_compatible(&self, a: &FnSig, b: &FnSig) -> bool {
        // Receiver mutability is part of the contract: a trait that declares
        // `mut self` must be implemented with `mut self`, and vice versa.
        if a.mut_receiver != b.mut_receiver {
            return false;
        }
        let (a_rest, b_rest) = (&a.params[1..], &b.params[1..]);
        if a_rest.len() != b_rest.len() {
            return false;
        }
        for (at, bt) in a_rest.iter().zip(b_rest) {
            if !Self::param_tys_compatible(at.ty.as_ref(), bt.ty.as_ref()) {
                return false;
            }
        }
        Self::param_tys_compatible(a.ret.as_ref(), b.ret.as_ref())
    }

    /// Parameter/return type comparability. `None` (unannotated) is compatible
    /// with anything; two annotated types must be mutually compatible.
    fn param_tys_compatible(a: Option<&Ty>, b: Option<&Ty>) -> bool {
        match (a, b) {
            (Some(x), Some(y)) => x.compatible_with(y) && y.compatible_with(x),
            _ => true,
        }
    }

    /// Overload identity: two signatures are the same overload when they have
    /// the same ordered parameter types, **up to alpha-renaming of their
    /// generic type parameters**. The return type, `mut self`, parameter names,
    /// and type-parameter names are NOT part of identity (`LANGUAGE_SPEC.md`
    /// §15.7). So `f<T>(x: T)` and `f<U>(x: U)` are the same overload, while
    /// `f<T>(x: T)` and `f(x: int)` are different ones.
    fn sig_identical(a: &FnSig, b: &FnSig) -> bool {
        if a.params.len() != b.params.len() {
            return false;
        }
        // Rename each side's parameters to positional placeholders so the
        // spelling of `T` / `U` does not affect identity.
        let a_map: HashMap<&str, String> = a
            .type_params
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (n.as_str(), format!("${i}")))
            .collect();
        let b_map: HashMap<&str, String> = b
            .type_params
            .iter()
            .enumerate()
            .map(|(i, (n, _))| (n.as_str(), format!("${i}")))
            .collect();
        a.params
            .iter()
            .zip(&b.params)
            .all(|(x, y)| match (&x.ty, &y.ty) {
                (Some(t1), Some(t2)) => {
                    Self::alpha_normalize(t1, &a_map) == Self::alpha_normalize(t2, &b_map)
                }
                (None, None) => true,
                _ => false,
            })
    }

    /// Rename a type's parameters to positional placeholders, producing a
    /// canonical form for alpha-equivalence comparison.
    fn alpha_normalize(t: &Ty, map: &HashMap<&str, String>) -> Ty {
        match t {
            Ty::Param(n) => Ty::Param(map.get(n.as_str()).cloned().unwrap_or_else(|| n.clone())),
            Ty::List(i) => Ty::List(Box::new(Self::alpha_normalize(i, map))),
            Ty::Map(k, v) => Ty::Map(
                Box::new(Self::alpha_normalize(k, map)),
                Box::new(Self::alpha_normalize(v, map)),
            ),
            Ty::Union(ms) => Ty::union(ms.iter().map(|m| Self::alpha_normalize(m, map)).collect()),
            Ty::App(n, args) => Ty::App(
                n.clone(),
                args.iter().map(|a| Self::alpha_normalize(a, map)).collect(),
            ),
            other => other.clone(),
        }
    }

    /// Check a statically resolved struct method call. The receiver is the
    /// implicit first argument, so the user writes only the remaining
    /// arguments; arity and per-parameter types are checked like a function.
    fn check_struct_method_args(
        &self,
        struct_name: &str,
        name: &str,
        sig: &FnSig,
        site: MethodSite<'_>,
        args: &[Arg],
        span: Span,
    ) -> Result<()> {
        let MethodSite { recv, ty_args } = site;
        // `sig.params[0]` is the receiver; the call supplies the rest.
        let rest = &sig.params[1..];
        if args.len() != rest.len() {
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!(
                    "method `{struct_name}.{name}` expects {} argument(s), got {}",
                    rest.len(),
                    args.len()
                ),
                span,
            ));
        }
        // A generic method's parameters (the impl's plus its own) are bound by
        // the receiver's type arguments and the argument types.
        let sigma = if sig.type_params.is_empty() {
            HashMap::new()
        } else {
            let explicit = self.explicit_substitution(name, sig, ty_args, span)?;
            let mut sigma = explicit;
            // Bind the impl's parameters from the receiver type.
            let recv_args = Self::ty_args_of(recv);
            if let Some(recv_head) = recv.nominal_head() {
                if let Some(declared) = self.type_type_params.get(&recv_head) {
                    for (p, a) in declared.iter().zip(&recv_args) {
                        sigma.insert(p.clone(), a.clone());
                    }
                }
            }
            let actual: Vec<Ty> = args.iter().map(|a| self.infer(&a.value)).collect();
            let rest_params: Vec<ParamSig> = sig.params[1..].to_vec();
            let Some(sigma) = self.infer_substitution(&rest_params, &actual, &sigma) else {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!("method `{struct_name}.{name}` cannot be instantiated with these argument types"),
                    span,
                ));
            };
            sigma
        };
        for (param_index, param) in rest.iter().enumerate() {
            let Some(expected) = &param.ty else { continue };
            let arg = &args[param_index];
            if let Some(named) = &arg.name {
                if named != &param.name {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!("method `{struct_name}.{name}` has no parameter named `{named}`"),
                        span,
                    ));
                }
            }
            let actual = self.infer(&arg.value);
            if !expected.substitute(&sigma).compatible_with(&actual) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "method `{struct_name}.{name}` parameter `{}` expects `{}`, found `{}`",
                        param.name,
                        expected.substitute(&sigma).name(),
                        actual.name()
                    ),
                    span,
                ));
            }
        }
        for (pname, pbounds) in &sig.type_params {
            if let Some(ty) = sigma.get(pname) {
                self.satisfies_bounds(ty, pbounds, span)?;
            }
        }
        Ok(())
    }

    /// Validate a method call when the receiver type is statically known.
    fn check_method_call(&self, recv: &Expr, name: &str, args: &[Arg], span: Span) -> Result<()> {
        let Some(class) = self.method_class_for(name, &self.infer(recv), span)? else {
            return Ok(()); // unknown receiver: cannot decide
        };
        let Some(sig) = crate::stdlib::signatures::method(class, name) else {
            return Err(Diag::new(
                codes::UNDEFINED,
                format!("{} has no method `{name}`", class.name()),
                span,
            ));
        };
        if let Some(message) = sig.check_arity(args.len()) {
            return Err(Diag::new(codes::TYPE_MISMATCH, message, span));
        }
        // A mutating built-in method requires the receiver to be reachable
        // through a `mut` binding (§16.6).
        if sig.mutates_receiver {
            self.require_mutable_place(recv, "mutate")?;
        }
        // A map's key-taking methods (`get`, `has`, `remove`) are validated
        // against the receiver's declared key type, which the coarse registry
        // cannot name.
        if class == crate::stdlib::signatures::TypeClass::Map
            && matches!(name, "get" | "has" | "remove")
        {
            if let Some(arg) = args.first() {
                let actual = self.infer(&arg.value);
                if !actual.is_key_capable() {
                    return Err(actual.key_type_error(span));
                }
                if let Ty::Map(k, _) = self.infer(recv) {
                    if !k.compatible_with(&actual) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "map has key type `{}` but `{name}` received `{}`",
                                k.name(),
                                actual.name()
                            ),
                            span,
                        ));
                    }
                }
            }
        }
        // `list.push(v)` on a statically known `[T]` receiver must accept only
        // `T`. The coarse registry cannot name `T`, so the element contract is
        // checked here.
        if class == crate::stdlib::signatures::TypeClass::List && name == "push" {
            if let (Ty::List(elem), Some(arg)) = (self.infer(recv), args.first()) {
                self.check_element_assignable(&elem, &self.infer(&arg.value), span)?;
            }
        }
        for (i, param) in sig.params.iter().enumerate() {
            let Some(arg) = args.get(i) else { break };
            let actual = self.infer(&arg.value);
            if param.accepts.accepts_ty(&actual) == Some(false) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "`{name}` argument {} expects {}, found `{}`",
                        i + 1,
                        param.accepts.describe(),
                        actual.name()
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }

    fn annotation(&self, t: &TypeExpr, span: Span) -> Result<Ty> {
        let mut visiting = Vec::new();
        let resolved = self.resolve_type_expr(t, span, &mut visiting)?;
        self.ty_from_expr(&resolved, span)
    }

    /// Convert a resolved type expression to a checker type, treating a name in
    /// scope as a generic parameter and enforcing generic arity on every
    /// application. This is the checker's replacement for [`Ty::from_expr`],
    /// which has no notion of parameters.
    fn ty_from_expr(&self, t: &TypeExpr, span: Span) -> Result<Ty> {
        Ok(match t {
            TypeExpr::Named(n) => {
                if self.is_type_param(n) {
                    return Ok(Ty::Param(n.clone()));
                }
                // A generic type used without arguments is an arity error
                // unless it declares no parameters.
                if let Some(params) = self.type_type_params.get(n) {
                    if !params.is_empty() {
                        return Err(Diag::new(
                            codes::UNKNOWN_TYPE,
                            format!("`{n}` expects {} type argument(s), found 0", params.len()),
                            span,
                        ));
                    }
                }
                Ty::from_expr(t, &self.type_kinds, span)?
            }
            TypeExpr::App(n, args) => {
                if self.is_type_param(n) {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("type parameter `{n}` is not a generic type and takes no type arguments"),
                        span,
                    ));
                }
                let Some(params) = self.type_type_params.get(n) else {
                    // Either an unknown type or a non-generic declared type.
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("`{n}` is not a generic type and takes no type arguments"),
                        span,
                    ));
                };
                if args.len() != params.len() {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!(
                            "`{n}` expects {} type argument(s), found {}",
                            params.len(),
                            args.len()
                        ),
                        span,
                    ));
                }
                let mut tys = Vec::with_capacity(args.len());
                for a in args {
                    tys.push(self.ty_from_expr(a, span)?);
                }
                // An enum application is still an enum type; keep it
                // distinguishable so variant checking keeps working.
                if self.type_kinds.get(n).map(String::as_str) == Some("enum") {
                    Ty::App(n.clone(), tys)
                } else {
                    Ty::App(n.clone(), tys)
                }
            }
            TypeExpr::List(inner) => Ty::List(Box::new(self.ty_from_expr(inner, span)?)),
            TypeExpr::Map(k, v) => {
                let key = self.ty_from_expr(k, span)?;
                if !key.is_key_capable() {
                    return Err(key.key_type_error(span));
                }
                Ty::Map(Box::new(key), Box::new(self.ty_from_expr(v, span)?))
            }
            TypeExpr::Union(members) => {
                let mut tys = Vec::with_capacity(members.len());
                for m in members {
                    tys.push(self.ty_from_expr(m, span)?);
                }
                Ty::union(tys)
            }
            _ => Ty::from_expr(t, &self.type_kinds, span)?,
        })
    }

    /// Convert a persisted REPL type annotation, treating any name in
    /// `params` as a generic parameter (`Ty::Param`) rather than a declared
    /// type. This is the session counterpart of [`Checker::annotation`], which
    /// needs the parameter names because a restored declaration may reference
    /// its own parameters before it has been re-added to the checker.
    fn session_type(&self, t: &TypeExpr, params: &[String]) -> Ty {
        let resolved = self.resolve_type_expr_lenient_with(t, params);
        Self::session_conv(&resolved, params)
    }

    /// The structural conversion behind [`Checker::session_type`], as an
    /// associated function so it does not capture `self`.
    fn session_conv(t: &TypeExpr, params: &[String]) -> Ty {
        match t {
            TypeExpr::Named(n) if params.iter().any(|p| p == n) => Ty::Param(n.clone()),
            TypeExpr::Named(n) => Ty::from_expr_lenient(&TypeExpr::Named(n.clone())),
            TypeExpr::App(n, args) => Ty::App(
                n.clone(),
                args.iter().map(|a| Self::session_conv(a, params)).collect(),
            ),
            TypeExpr::List(i) => Ty::List(Box::new(Self::session_conv(i, params))),
            TypeExpr::Map(k, v) => Ty::Map(
                Box::new(Self::session_conv(k, params)),
                Box::new(Self::session_conv(v, params)),
            ),
            TypeExpr::Union(ms) => {
                Ty::union(ms.iter().map(|m| Self::session_conv(m, params)).collect())
            }
            other => Ty::from_expr_lenient(other),
        }
    }

    /// Alias resolution that leaves names in `params` untouched (they are
    /// generic parameters, never alias targets).
    fn resolve_type_expr_lenient_with(&self, t: &TypeExpr, params: &[String]) -> TypeExpr {
        if let TypeExpr::Named(n) = t {
            if params.iter().any(|p| p == n) {
                return t.clone();
            }
        }
        self.resolve_type_expr_lenient(t)
    }

    /// Whether `name` is a generic type parameter currently in scope.
    /// Check that a written type-argument list on `name` has the declared
    /// arity, and binds every argument to a known type. Used for `impl` heads.
    fn check_type_args_arity(&self, name: &str, args: &[TypeExpr], span: Span) -> Result<()> {
        let declared = self
            .type_type_params
            .get(name)
            .or_else(|| self.trait_type_params.get(name));
        match declared {
            Some(params) => {
                if args.len() != params.len() {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!(
                            "`{name}` expects {} type argument(s), found {}",
                            params.len(),
                            args.len()
                        ),
                        span,
                    ));
                }
            }
            None if args.is_empty() => {}
            None => {
                return Err(Diag::new(
                    codes::UNKNOWN_TYPE,
                    format!("`{name}` is not a generic type and takes no type arguments"),
                    span,
                ));
            }
        }
        for a in args {
            // Resolve each argument so an unknown or parameter reference is
            // reported at the `impl` head rather than silently accepted.
            let mut visiting = Vec::new();
            let r = self.resolve_type_expr(a, span, &mut visiting)?;
            self.ty_from_expr(&r, span)?;
        }
        Ok(())
    }

    /// Whether a type parameter's bounds currently in scope permit using a
    /// concrete type. A parameter satisfies a bound when the concrete type is
    /// (a) itself a parameter whose own bounds include it, or (b) a struct with
    /// a matching `impl Trait for Struct`.
    fn satisfies_bounds(&self, ty: &Ty, bounds: &[String], span: Span) -> Result<()> {
        for b in bounds {
            let ok = match ty {
                Ty::Param(p) => self.bounds_of_param(p).iter().any(|x| x == b),
                _ => self.type_satisfies_trait(ty, b),
            };
            if !ok {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!("type `{}` does not implement trait `{b}`", ty.name()),
                    span,
                ));
            }
        }
        Ok(())
    }

    /// Whether a concrete (or parameterised) type implements `trait_name`.
    fn type_satisfies_trait(&self, ty: &Ty, trait_name: &str) -> bool {
        let Some(head) = ty.nominal_head() else {
            return false;
        };
        // A generic trait `C<T>`: the struct's `impl C<...> for S<...>` is
        // recorded under the bare trait name and the bare struct name, so the
        // head match is the check the contract needs (bounds are structural on
        // the head, and the `impl` itself is arity-checked).
        self.trait_impls
            .contains_key(&(head, trait_name.to_string()))
    }

    fn is_type_param(&self, name: &str) -> bool {
        self.current_type_params
            .iter()
            .rev()
            .any(|scope| scope.iter().any(|(p, _)| p == name))
    }

    /// Push the type parameters of a declaration for the duration of checking
    /// its signature and body.
    fn push_type_params(&mut self, params: &[TypeParam]) {
        self.current_type_params.push(
            params
                .iter()
                .map(|p| (p.name.clone(), p.bounds.clone()))
                .collect(),
        );
    }

    fn pop_type_params(&mut self) {
        self.current_type_params.pop();
    }

    /// The bounds declared on a type parameter in scope, if any.
    fn bounds_of_param(&self, name: &str) -> Vec<String> {
        for scope in self.current_type_params.iter().rev() {
            for (p, bounds) in scope.iter().rev() {
                if p == name {
                    return bounds.clone();
                }
            }
        }
        Vec::new()
    }

    /// Check that every type parameter of a declaration is unique and does not
    /// shadow a declared type or an enclosing parameter. Returns `E2007` for a
    /// violation. Parameter identity is by position, so a duplicate name is the
    /// only way two parameters can be confused.
    fn check_type_params(&self, params: &[TypeParam], span: Span) -> Result<()> {
        let mut seen: Vec<&str> = Vec::new();
        for p in params {
            if seen.contains(&p.name.as_str()) {
                return Err(Diag::new(
                    codes::REDECLARED,
                    format!("type parameter `{}` is declared more than once", p.name),
                    p.span,
                ));
            }
            seen.push(&p.name);
            if self.types.contains_key(&p.name) {
                return Err(Diag::new(
                    codes::REDECLARED,
                    format!(
                        "type parameter `{}` shadows the declared type `{}`",
                        p.name, p.name
                    ),
                    p.span,
                ));
            }
            // A parameter may not shadow one of an enclosing declaration; the
            // enclosing parameter is reused instead (`LANGUAGE_SPEC.md`).
            if self.is_type_param(&p.name) {
                return Err(Diag::new(
                    codes::REDECLARED,
                    format!(
                        "type parameter `{}` shadows an enclosing type parameter of the same name",
                        p.name
                    ),
                    p.span,
                ));
            }
        }
        let _ = span;
        Ok(())
    }

    /// Check that a generic type parameter's declared bounds name real traits.
    fn check_type_param_bounds(&self, params: &[TypeParam]) -> Result<()> {
        for p in params {
            for b in &p.bounds {
                if !self.traits.contains_key(b) && !self.declared_traits.contains(b) {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("unknown trait `{b}` in the bound of `{}`", p.name),
                        p.span,
                    ));
                }
            }
        }
        Ok(())
    }

    /// Substitute `type` aliases by their targets, recursively, so a
    /// transparent alias denotes the type it names. Unknown names are left
    /// untouched and validated later by [`Ty::from_expr`].
    ///
    /// A cyclic alias (`type A = A`, or a longer cycle) has no concrete target
    /// and is rejected with `E3002` rather than recursing without bound.
    fn resolve_type_expr(
        &self,
        t: &TypeExpr,
        span: Span,
        visiting: &mut Vec<String>,
    ) -> Result<TypeExpr> {
        Ok(match t {
            TypeExpr::Named(n) => {
                // A bound generic parameter is a placeholder: it is left as
                // written and never resolved to a declared type.
                if self.is_type_param(n) {
                    return Ok(t.clone());
                }
                match self.alias_targets.get(n) {
                    Some(target) => {
                        if visiting.iter().any(|v| v == n) {
                            return Err(Diag::new(
                                codes::UNKNOWN_TYPE,
                                format!("recursive type alias `{n}` has no concrete target"),
                                span,
                            ));
                        }
                        // A successful resolution is context-free and reusable.
                        // Serving it from cache collapses the exponential fan-out
                        // of aliases that name another alias more than once.
                        if let Some(cached) = self.resolved_aliases.borrow().get(n) {
                            return Ok(cached.clone());
                        }
                        visiting.push(n.clone());
                        let resolved = self.resolve_type_expr(target, span, visiting)?;
                        visiting.pop();
                        self.resolved_aliases
                            .borrow_mut()
                            .insert(n.clone(), resolved.clone());
                        resolved
                    }
                    None => t.clone(),
                }
            }
            // A generic application: resolve the head's arguments, and expand a
            // parameterised alias by substituting its arguments into its target.
            TypeExpr::App(n, args) => {
                let mut resolved_args = Vec::with_capacity(args.len());
                for a in args {
                    resolved_args.push(self.resolve_type_expr(a, span, visiting)?);
                }
                if self.is_type_param(n) {
                    return Ok(TypeExpr::App(n.clone(), resolved_args));
                }
                if let Some(target) = self.alias_targets.get(n) {
                    return self.substitute_alias(n, &resolved_args, target, span, visiting);
                }
                TypeExpr::App(n.clone(), resolved_args)
            }
            TypeExpr::List(inner) => {
                TypeExpr::List(Box::new(self.resolve_type_expr(inner, span, visiting)?))
            }
            TypeExpr::Map(k, v) => TypeExpr::Map(
                Box::new(self.resolve_type_expr(k, span, visiting)?),
                Box::new(self.resolve_type_expr(v, span, visiting)?),
            ),
            TypeExpr::Union(members) => {
                let mut resolved = Vec::with_capacity(members.len());
                for m in members {
                    resolved.push(self.resolve_type_expr(m, span, visiting)?);
                }
                normalize_resolved_union(resolved)
            }
            _ => t.clone(),
        })
    }

    /// Expand a parameterised alias `Name<A...>` whose target mentions the
    /// declaration's parameters, by substituting argument `i` for parameter
    /// `i` in the target and resolving the result. Guards against a recursive
    /// alias with the same `visiting` stack the unparameterised case uses.
    fn substitute_alias(
        &self,
        name: &str,
        args: &[TypeExpr],
        target: &TypeExpr,
        span: Span,
        visiting: &mut Vec<String>,
    ) -> Result<TypeExpr> {
        let params = self.type_type_params.get(name).cloned().unwrap_or_default();
        if args.len() != params.len() {
            return Err(Diag::new(
                codes::UNKNOWN_TYPE,
                format!(
                    "type alias `{name}` expects {} type argument(s), found {}",
                    params.len(),
                    args.len()
                ),
                span,
            ));
        }
        if visiting.iter().any(|v| v == name) {
            return Err(Diag::new(
                codes::UNKNOWN_TYPE,
                format!("recursive type alias `{name}` has no concrete target"),
                span,
            ));
        }
        let mut subst: HashMap<String, TypeExpr> = HashMap::new();
        for (p, a) in params.iter().zip(args) {
            subst.insert(p.clone(), a.clone());
        }
        let expanded = self.substitute_type_expr(target, &subst);
        visiting.push(name.to_string());
        let resolved = self.resolve_type_expr(&expanded, span, visiting);
        visiting.pop();
        resolved
    }

    /// Replace generic parameters by name in a type expression (used to expand
    /// a parameterised alias). Structural, not memoized: alias memoisation
    /// applies to the fully expanded result.
    fn substitute_type_expr(&self, t: &TypeExpr, subst: &HashMap<String, TypeExpr>) -> TypeExpr {
        match t {
            TypeExpr::Named(n) => match subst.get(n) {
                Some(replacement) => replacement.clone(),
                None => t.clone(),
            },
            TypeExpr::App(n, args) => TypeExpr::App(
                n.clone(),
                args.iter()
                    .map(|a| self.substitute_type_expr(a, subst))
                    .collect(),
            ),
            TypeExpr::List(i) => TypeExpr::List(Box::new(self.substitute_type_expr(i, subst))),
            TypeExpr::Map(k, v) => TypeExpr::Map(
                Box::new(self.substitute_type_expr(k, subst)),
                Box::new(self.substitute_type_expr(v, subst)),
            ),
            TypeExpr::Union(ms) => TypeExpr::Union(
                ms.iter()
                    .map(|m| self.substitute_type_expr(m, subst))
                    .collect(),
            ),
            other => other.clone(),
        }
    }

    /// Alias resolution that never fails, used when rebuilding a session from
    /// declarations that were already validated. A cycle is left unresolved
    /// rather than recursing without bound; it cannot reach this path through
    /// checking because [`resolve_type_expr`] rejects it first.
    fn resolve_type_expr_lenient(&self, t: &TypeExpr) -> TypeExpr {
        fn go(checker: &Checker, t: &TypeExpr, visiting: &mut Vec<String>) -> TypeExpr {
            match t {
                TypeExpr::Named(n) => match checker.alias_targets.get(n) {
                    Some(target) if !visiting.iter().any(|v| v == n) => {
                        // Mirror the strict resolver's memoization so rebuilding
                        // a session from declarations cannot fan out
                        // exponentially. On this path the declarations already
                        // passed strict checking, so no cycle can reach here.
                        if let Some(cached) = checker.resolved_aliases.borrow().get(n) {
                            return cached.clone();
                        }
                        visiting.push(n.clone());
                        let resolved = go(checker, target, visiting);
                        visiting.pop();
                        checker
                            .resolved_aliases
                            .borrow_mut()
                            .insert(n.clone(), resolved.clone());
                        resolved
                    }
                    Some(_) => t.clone(),
                    None => t.clone(),
                },
                TypeExpr::List(inner) => TypeExpr::List(Box::new(go(checker, inner, visiting))),
                TypeExpr::Map(k, v) => TypeExpr::Map(
                    Box::new(go(checker, k, visiting)),
                    Box::new(go(checker, v, visiting)),
                ),
                TypeExpr::Union(members) => normalize_resolved_union(
                    members.iter().map(|m| go(checker, m, visiting)).collect(),
                ),
                _ => t.clone(),
            }
        }
        go(self, t, &mut Vec::new())
    }

    /// Validate a struct literal against the declared fields.
    ///
    /// Named construction (`S { a: 1 }`) requires every supplied name to be a
    /// declared field, every declared field to be supplied exactly once, and
    /// every value to be compatible with its field type. Positional
    /// construction (`S(1)`) requires exactly one value per field in
    /// declaration order. Only mismatches the checker can prove are reported;
    /// an `Unknown` value is accepted.
    fn check_struct_construction(
        &self,
        name: &str,
        fields: &HashMap<String, Ty>,
        args: &[Arg],
        span: Span,
    ) -> Result<()> {
        let named = args.iter().any(|a| a.name.is_some());
        if args.iter().any(|a| a.name.is_none()) && named {
            // A mixture would make "which field does this value belong to?"
            // ambiguous; the grammar allows it but the semantics do not.
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!("`{name}` mixes named and positional fields; use one form"),
                span,
            ));
        }
        // A private field cannot be named in a construction from outside the
        // struct's module, in either form (`LANGUAGE_SPEC.md` §28).
        for a in args {
            if let Some(f) = &a.name {
                if !self.field_visible(name, f) {
                    return Err(Diag::new(
                        codes::PRIVATE_ACCESS,
                        format!(
                            "field `{f}` of `{name}` is private; it cannot be set from this module"
                        ),
                        span,
                    ));
                }
            }
        }
        if !named {
            // Positional construction names every field in order, so any
            // private field makes the whole construction inaccessible.
            if let Some(order) = self.struct_field_order.get(name) {
                for f in order {
                    if !self.field_visible(name, f) {
                        return Err(Diag::new(
                            codes::PRIVATE_ACCESS,
                            format!(
                                "field `{f}` of `{name}` is private; it cannot be set from this module"
                            ),
                            span,
                        ));
                    }
                }
            }
        }
        if named {
            let mut seen: Vec<&str> = Vec::new();
            for a in args {
                let fname = a.name.as_deref().unwrap_or_default();
                if !fields.contains_key(fname) {
                    return Err(Diag::new(
                        codes::UNDEFINED,
                        format!("`{name}` has no field `{fname}`"),
                        span,
                    ));
                }
                if seen.contains(&fname) {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!("field `{fname}` is given more than once for `{name}`"),
                        span,
                    ));
                }
                seen.push(fname);
                if let Some(fty) = fields.get(fname) {
                    self.check_field_value(name, fname, fty, &a.value, span)?;
                }
            }
            let order = self
                .struct_field_order
                .get(name)
                .cloned()
                .unwrap_or_default();
            for f in order {
                if !seen.contains(&f.as_str()) {
                    return Err(Diag::new(
                        codes::TYPE_MISMATCH,
                        format!("missing field `{f}` for `{name}`"),
                        span,
                    ));
                }
            }
        } else {
            let order = self
                .struct_field_order
                .get(name)
                .cloned()
                .unwrap_or_default();
            if args.len() != order.len() {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "`{name}` expects {} field(s), got {}",
                        order.len(),
                        args.len()
                    ),
                    span,
                ));
            }
            for (f, a) in order.iter().zip(args) {
                if let Some(fty) = fields.get(f) {
                    self.check_field_value(name, f, fty, &a.value, span)?;
                }
            }
        }
        Ok(())
    }

    /// A field value must be compatible with the declared field type when both
    /// are statically known.
    fn check_field_value(
        &self,
        name: &str,
        field: &str,
        expected: &Ty,
        value: &Expr,
        span: Span,
    ) -> Result<()> {
        let actual = self.infer(value);
        if !expected.compatible_with(&actual) {
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!(
                    "field `{field}` of `{name}` is `{}` but the value is `{}`",
                    expected.name(),
                    actual.name()
                ),
                span,
            ));
        }
        Ok(())
    }

    /// Validate an enum-variant construction.
    ///
    /// Variants are positional: named arguments are rejected, the payload
    /// count must match exactly, and each value must be compatible with its
    /// declared type.
    fn check_variant_construction(&self, name: &str, args: &[Arg], span: Span) -> Result<()> {
        if let Some(a) = args.iter().find(|a| a.name.is_some()) {
            let fname = a.name.as_deref().unwrap_or_default();
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!("variant `{name}` is positional; `{fname}: ...` is not allowed here"),
                span,
            ));
        }
        let payload = self.variant_payloads.get(name).cloned().unwrap_or_default();
        if args.len() != payload.len() {
            return Err(Diag::new(
                codes::TYPE_MISMATCH,
                format!(
                    "variant `{name}` expects {} value(s), got {}",
                    payload.len(),
                    args.len()
                ),
                span,
            ));
        }
        for (expected, a) in payload.iter().zip(args) {
            let actual = self.infer(&a.value);
            if !expected.compatible_with(&actual) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "variant `{name}` field is `{}` but the value is `{}`",
                        expected.name(),
                        actual.name()
                    ),
                    span,
                ));
            }
        }
        Ok(())
    }

    fn block(&mut self, body: &[Stmt]) -> Result<()> {
        self.push();
        for s in body {
            self.stmt(s)?;
        }
        self.pop();
        Ok(())
    }

    fn stmt(&mut self, s: &Stmt) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_AST_DEPTH {
            self.depth -= 1;
            return Err(Diag::new(
                codes::NESTING,
                "statement nests too deeply",
                Span::default(),
            ));
        }
        let r = self.stmt_inner(s);
        self.depth -= 1;
        r
    }

    fn stmt_inner(&mut self, s: &Stmt) -> Result<()> {
        match s {
            Stmt::Let {
                mutable,
                name,
                ann,
                value,
                span,
            } => {
                self.expr(value)?;
                // The new binding's recorded type replaces any type the
                // shadowed binding had: a shadow is a *different* binding, so
                // it must not inherit the previous one's type when its own
                // type is unknown.
                let new_ty = if let Some(ann) = ann {
                    let expected = self.annotation(ann, *span)?;
                    let actual = self.infer(value);
                    if !expected.compatible_with(&actual) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "`{name}` is annotated as `{}` but its value is `{}`",
                                expected.name(),
                                actual.name()
                            ),
                            *span,
                        ));
                    }
                    Some(expected)
                } else {
                    // Without an annotation, still remember any inferred type
                    // (a user struct for field checking, or a builtin type so
                    // method calls on it can be validated). `Unknown` is not
                    // recorded, and clears any prior recording for this name.
                    match self.infer(value) {
                        Ty::Unknown => None,
                        t => Some(t),
                    }
                };
                if let Some(map) = self.value_types.last_mut() {
                    match new_ty {
                        Some(t) => {
                            map.insert(name.clone(), t);
                        }
                        None => {
                            map.remove(name);
                        }
                    }
                }
                // An ordinary `let`/`let mut` binding may shadow an existing
                // binding of the same name in this scope (`LANGUAGE_SPEC.md`
                // §16.3). The initializer above was checked before this call,
                // so it resolves against the bindings visible *before* the new
                // declaration.
                self.declare_shadowing(name, *mutable, *span)?;
            }
            Stmt::LetPattern {
                pattern,
                value,
                span,
            } => {
                // A destructuring `let` (§4.7): validate the pattern with the
                // same rules `match`/`for` use (duplicate binding `E2014`,
                // unknown variant `E3002`), then declare every name immutably.
                // Like a simple `let`, each name may shadow an existing
                // binding. No type inference is performed and no `value_types`
                // entry is created, so destructured names stay `Ty::Unknown`.
                self.expr(value)?;
                self.check_pattern(pattern)?;
                for name in pattern.bindings() {
                    self.declare_shadowing(&name, false, *span)?;
                }
            }
            Stmt::Assign {
                target,
                value,
                op,
                span,
            } => {
                self.expr(value)?;
                match target {
                    Expr::Name(name, nspan) => {
                        match self.lookup(name) {
                            Some(true) => {}
                            Some(false) => {
                                return Err(Diag::new(
                                    codes::ASSIGN_IMMUTABLE,
                                    format!(
                                    "cannot assign to `{name}`: it is immutable (declare it `let mut`)"
                                ),
                                    *nspan,
                                ))
                            }
                            None => {
                                return Err(Diag::new(
                                    codes::UNDEFINED,
                                    format!("undefined variable `{name}`"),
                                    *nspan,
                                ))
                            }
                        }
                        // If the binding has a known annotated type, an
                        // assignment must respect it.
                        if let Some(expected) = self.lookup_type(name) {
                            let actual = if op.is_some() {
                                Ty::Unknown
                            } else {
                                self.infer(value)
                            };
                            if !matches!(actual, Ty::Unknown) && !expected.compatible_with(&actual)
                            {
                                return Err(Diag::new(
                                    codes::TYPE_MISMATCH,
                                    format!(
                                        "`{name}` is `{}` but the assigned value is `{}`",
                                        expected.name(),
                                        actual.name()
                                    ),
                                    *span,
                                ));
                            }
                        }
                    }
                    Expr::Index(base, idx, span) => {
                        self.expr(base)?;
                        self.expr(idx)?;
                        self.require_mutable_place(base, "assign through")?;
                        // Indexed assignment on a statically known map obeys the
                        // same key/value contract as construction and lookup.
                        if let Ty::Map(k, v) = self.infer(base) {
                            let it = self.infer(idx);
                            if !it.is_key_capable() {
                                return Err(it.key_type_error(*span));
                            }
                            if !k.compatible_with(&it) {
                                return Err(Diag::new(
                                    codes::TYPE_MISMATCH,
                                    format!(
                                        "map has key type `{}` but the index is `{}`",
                                        k.name(),
                                        it.name()
                                    ),
                                    *span,
                                ));
                            }
                            let actual = self.infer(value);
                            if !matches!(actual, Ty::Unknown) && !v.compatible_with(&actual) {
                                return Err(Diag::new(
                                    codes::TYPE_MISMATCH,
                                    format!(
                                        "map has value type `{}` but the assigned value is `{}`",
                                        v.name(),
                                        actual.name()
                                    ),
                                    *span,
                                ));
                            }
                        } else if let Ty::List(elem) = self.infer(base) {
                            // Indexed assignment on a statically known list obeys
                            // the element contract. An `int` index is the only
                            // valid list index.
                            self.check_element_assignable(&elem, &self.infer(value), *span)?;
                        }
                    }
                    Expr::Field(base, fname, fspan) => {
                        self.expr(base)?;
                        self.require_mutable_place(base, "assign to a field of")?;
                        // A private field cannot be written from another
                        // module, even through a mutable binding.
                        if let Ty::Named(sname) = self.infer(base) {
                            if !self.field_visible(&sname, fname) {
                                return Err(Diag::new(
                                    codes::PRIVATE_ACCESS,
                                    format!("field `{fname}` of `{sname}` is private; it cannot be assigned from this module"),
                                    *fspan,
                                ));
                            }
                        }
                        // If the base is a known struct, the assigned value
                        // must match the declared field type.
                        if op.is_none() {
                            if let Ty::Named(sname) = self.infer(base) {
                                match self.struct_fields.get(&sname).and_then(|m| m.get(fname)) {
                                    Some(fty) => {
                                        let actual = self.infer(value);
                                        if !matches!(actual, Ty::Unknown)
                                            && !fty.compatible_with(&actual)
                                        {
                                            return Err(Diag::new(
                                                codes::TYPE_MISMATCH,
                                                format!(
                                                    "field `{fname}` is `{}` but the assigned value is `{}`",
                                                    fty.name(),
                                                    actual.name()
                                                ),
                                                *fspan,
                                            ));
                                        }
                                    }
                                    None => {
                                        return Err(Diag::new(
                                            codes::UNDEFINED,
                                            format!("`{sname}` has no field `{fname}`"),
                                            *fspan,
                                        ))
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        return Err(Diag::new(
                            codes::INVALID_ASSIGN,
                            "invalid assignment target",
                            *span,
                        ))
                    }
                }
            }
            Stmt::Expr(e, _) => self.expr(e)?,
            Stmt::Return(v, span) => {
                if let Some(v) = v {
                    self.expr(v)?;
                    if let Some(expected) = self.return_type.clone() {
                        let actual = self.infer(v);
                        if !return_compatible(&expected, &actual) {
                            return Err(Diag::new(
                                codes::RETURN_MISMATCH,
                                format!(
                                    "function returns `{}` but the value is `{}`",
                                    expected.name(),
                                    actual.name()
                                ),
                                *span,
                            ));
                        }
                    }
                }
            }
            Stmt::Throw(v, _) => self.expr(v)?,
            Stmt::Break(span) | Stmt::Continue(span) => {
                if self.loop_depth == 0 {
                    return Err(Diag::new(
                        codes::LOOP_CONTROL,
                        "`break`/`continue` can only appear inside a loop",
                        *span,
                    ));
                }
            }
            Stmt::While(c, body, _) => {
                self.expr(c)?;
                self.loop_depth += 1;
                let r = self.block(body);
                self.loop_depth -= 1;
                r?;
            }
            Stmt::Loop(body, _) => {
                self.loop_depth += 1;
                let r = self.block(body);
                self.loop_depth -= 1;
                r?;
            }
            Stmt::For(pat, iter, body, span) => {
                self.expr(iter)?;
                // A statically-known scalar can never be iterated.
                if matches!(self.infer(iter), Ty::Int | Ty::Float | Ty::Bool) {
                    return Err(Diag::new(
                        codes::NOT_ITERABLE,
                        format!("`{}` is not iterable", self.infer(iter).name()),
                        *span,
                    ));
                }
                self.push();
                // Validate the loop pattern with the same rules `match` and
                // destructuring `let` use, so a non-variant tag or a duplicate
                // binding is rejected statically and consistently with the
                // runtime.
                self.check_pattern(pat)?;
                for b in pat.bindings() {
                    self.declare(&b, false, Span::default())?;
                }
                self.loop_depth += 1;
                let loop_result: Result<()> = (|| {
                    for s in body {
                        self.stmt(s)?;
                    }
                    Ok(())
                })();
                self.loop_depth -= 1;
                self.pop();
                loop_result?;
            }
            Stmt::Try {
                body,
                catch,
                catch_body,
                finally,
                ..
            } => {
                self.block(body)?;
                self.push();
                self.declare(catch, false, Span::default())?;
                for s in catch_body {
                    self.stmt(s)?;
                }
                self.pop();
                if let Some(f) = finally {
                    self.block(f)?;
                }
            }
        }
        Ok(())
    }

    fn expr(&mut self, e: &Expr) -> Result<()> {
        self.depth += 1;
        if self.depth > MAX_AST_DEPTH {
            self.depth -= 1;
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                Span::default(),
            ));
        }
        let r = self.expr_inner(e);
        self.depth -= 1;
        r
    }

    fn expr_inner(&mut self, e: &Expr) -> Result<()> {
        match e {
            Expr::Lit(_, _) => {}
            Expr::Name(name, span) => {
                // A bare reference to an *overloaded* function carries no
                // argument types, so there is no way to pick an overload. This
                // is deferred (`LANGUAGE_SPEC.md` §15.7): referencing an
                // overloaded name as a value is rejected deterministically
                // rather than silently binding the first overload.
                if let Some(set) = self.functions.get(name) {
                    if set.len() > 1 && self.resolves_to_user_function(name) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "`{name}` is an overloaded function and cannot be used as a value; call it or bind a single-overload function"
                            ),
                            *span,
                        ));
                    }
                }
                if self.lookup(name).is_none() {
                    return Err(Diag::new(
                        codes::UNDEFINED,
                        format!("undefined variable `{name}`"),
                        *span,
                    ));
                }
                self.used_names.entry(name.clone()).or_insert(*span);
            }
            Expr::FStr(parts, _) => {
                for p in parts {
                    if let FPart::Expr(inner, _) = p {
                        self.expr(inner)?;
                    }
                }
            }
            Expr::Unary(_, o, _) => self.expr(o)?,
            Expr::Binary(op, l, r, span) => {
                self.expr(l)?;
                self.expr(r)?;
                if matches!(op, BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge) {
                    let lt = self.infer(l);
                    let rt = self.infer(r);
                    if lt.orderable_with(&rt) == Some(false) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!("cannot compare `{}` with `{}`", lt.name(), rt.name()),
                            *span,
                        ));
                    }
                }
            }
            Expr::Call(f, args, ty_args, span) => {
                match f.as_ref() {
                    Expr::Name(name, nspan) => {
                        // A direct call must resolve to a function, a builtin,
                        // or a callable local. Functions are hoisted, so
                        // forward references resolve.
                        if self.resolves_to_user_function(name) {
                            // A directly resolved top-level function: select the
                            // overload by argument types among the **visible**
                            // overloads only, then check the call against that
                            // signature (§6.5, §15.7, §28).
                            if let Some(set) = self.functions.get(name) {
                                let visible: Vec<FnSig> =
                                    set.iter().filter(|s| self.fn_visible(s)).cloned().collect();
                                if visible.is_empty() {
                                    return Err(Diag::new(
                                        codes::PRIVATE_ACCESS,
                                        format!(
                                            "function `{name}` is private; it is not visible from this module"
                                        ),
                                        *span,
                                    ));
                                }
                                match self.resolve_call_sig(&visible, args) {
                                    Some(i) => {
                                        let sig = visible[i].clone();
                                        self.check_user_call(name, &sig, args, ty_args, *span)?;
                                    }
                                    None => {
                                        // A matching but private overload is a
                                        // visibility problem, not an undefined
                                        // one; report it as such.
                                        let private_hits = set
                                            .iter()
                                            .filter(|s| !self.fn_visible(s))
                                            .cloned()
                                            .collect::<Vec<_>>();
                                        if self.resolve_call_sig(&private_hits, args).is_some() {
                                            return Err(Diag::new(
                                                codes::PRIVATE_ACCESS,
                                                format!(
                                                    "function `{name}` is private; it is not visible from this module"
                                                ),
                                                *span,
                                            ));
                                        }
                                        return Err(
                                            self.no_overload_diag(name, &visible, args, *span)
                                        );
                                    }
                                }
                            }
                        } else if let Some(sig) = crate::stdlib::signatures::builtin(name) {
                            // Builtins are positional; named arguments require
                            // a resolved user-function parameter list.
                            self.reject_named_args(name, args, *span)?;
                            self.check_builtin_call(sig, args, *span)?;
                            // A mutating builtin requires the mutated argument
                            // to be reached through a `mut` binding (§16.6).
                            if let Some(i) = sig.mutates_arg {
                                if let Some(arg) = args.get(i) {
                                    self.require_mutable_place(&arg.value, "mutate")?;
                                }
                            }
                        } else if self.lookup(name).is_some() {
                            // A callable binding (closure value): dynamic.
                            self.reject_named_args(name, args, *span)?;
                        } else {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!("undefined function `{name}`"),
                                *nspan,
                            ));
                        }
                    }
                    other => {
                        // A non-name callee is dynamic; named arguments cannot
                        // be resolved against a parameter list.
                        self.expr(other)?;
                        self.reject_named_args("callable", args, *span)?;
                    }
                }
                for a in args {
                    self.expr(&a.value)?;
                }
            }
            Expr::Method(r, name, args, ty_args, span) => {
                self.expr(r)?;
                let recv = self.infer(r);
                // A statically known struct resolves against its nominal method
                // table only (§17.6); there is no fallback to the built-in
                // registry or to another struct.
                if let Some(sname) = self.struct_name_of(&recv) {
                    let Some(set) = self.method_set(&sname, name) else {
                        return Err(Diag::new(
                            codes::UNDEFINED,
                            format!("struct `{sname}` has no method `{name}`"),
                            *span,
                        ));
                    };
                    self.reject_named_args(name, args, *span)?;
                    let Some(i) = self.resolve_visible_method_sig(set, args) else {
                        // The name may exist only in inaccessible overloads:
                        // that is a visibility error, not an undefined one.
                        if self.method_exists_but_private(set) {
                            return Err(Diag::new(
                                codes::PRIVATE_ACCESS,
                                format!(
                                    "method `{name}` of `{sname}` is private; it is not visible from this module"
                                ),
                                *span,
                            ));
                        }
                        return Err(self.no_method_overload_diag(&sname, name, set, args, *span));
                    };
                    let visible: Vec<FnSig> = set
                        .iter()
                        .filter(|s| self.method_visible(s))
                        .cloned()
                        .collect();
                    let sig = visible[i].clone();
                    self.check_struct_method_args(
                        &sname,
                        name,
                        &sig,
                        MethodSite {
                            recv: &recv,
                            ty_args,
                        },
                        args,
                        *span,
                    )?;
                    // A method whose receiver is `mut self` mutates the
                    // caller's value, so that value must be reachable through
                    // a `mut` binding (§16.6).
                    if sig.mut_receiver {
                        self.require_mutable_place(r, "mutate")?;
                    }
                } else if let Ty::Union(ms) = &recv {
                    // A union member is available only if every member type
                    // provides it as the same member kind with a compatible
                    // signature (`LANGUAGE_SPEC.md` §17.6). Comparing the
                    // signatures here keeps the checker from accepting a call
                    // that a concrete member would reject at runtime.
                    let mut first: Option<FnSig> = None;
                    for m in ms {
                        let Some(sname) = self.struct_name_of(m) else {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!("not every union member has method `{name}`"),
                                *span,
                            ));
                        };
                        let Some(set) = self.method_set(&sname, name) else {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!("not every union member has method `{name}`"),
                                *span,
                            ));
                        };
                        if self.method_exists_but_private(set)
                            && self.resolve_visible_method_sig(set, args).is_none()
                        {
                            return Err(Diag::new(
                                codes::PRIVATE_ACCESS,
                                format!(
                                    "method `{name}` of `{sname}` is private; it is not visible from this module"
                                ),
                                *span,
                            ));
                        }
                        let Some(i) = self.resolve_visible_method_sig(set, args) else {
                            return Err(
                                self.no_method_overload_diag(&sname, name, set, args, *span)
                            );
                        };
                        let visible: Vec<FnSig> = set
                            .iter()
                            .filter(|s| self.method_visible(s))
                            .cloned()
                            .collect();
                        let sig = visible[i].clone();
                        match &mut first {
                            None => first = Some(sig),
                            Some(prev) => {
                                if !self.method_sigs_compatible(prev, &sig) {
                                    return Err(Diag::new(
                                        codes::TYPE_MISMATCH,
                                        format!(
                                            "union members disagree on the signature of method `{name}`"
                                        ),
                                        *span,
                                    ));
                                }
                            }
                        }
                    }
                    self.reject_named_args(name, args, *span)?;
                    // If every member declares `mut self`, the union call
                    // mutates the receiver, so its binding must be `mut`.
                    if first.is_some_and(|s| s.mut_receiver) {
                        self.require_mutable_place(r, "mutate")?;
                    }
                } else if !crate::stdlib::signatures::method_exists_anywhere(name)
                    && !self.user_method_exists_anywhere(name)
                {
                    // For an `Unknown` receiver the checker stays conservative:
                    // a call is impossible only when the name is neither a
                    // built-in method nor a method of any declared struct. A
                    // user struct method with this name is a valid runtime
                    // target, so it must not be gated by the built-in registry
                    // alone (`LANGUAGE_SPEC.md` §2.3, §17.6).
                    return Err(Diag::new(
                        codes::UNDEFINED,
                        format!("no method `{name}` on any type"),
                        *span,
                    ));
                } else {
                    // Methods are positional; named arguments are out of scope.
                    self.reject_named_args(name, args, *span)?;
                    self.check_method_call(r, name, args, *span)?;
                }
                for a in args {
                    self.expr(&a.value)?;
                }
            }
            Expr::Field(r, name, span) => {
                self.expr(r)?;
                // On a statically known struct, `r.name` is always a field
                // read; a method must be invoked with parentheses (§17.6).
                // A missing field stays `Unknown` (§17.5), unchanged.
                if let Some(sname) = self.struct_name_of(&self.infer(r)) {
                    if self.method_set(&sname, name).is_some() {
                        return Err(Diag::new(
                            codes::UNDEFINED,
                            format!(
                                "struct `{sname}` has method `{name}`; call it as `{name}(...)`"
                            ),
                            *span,
                        ));
                    }
                    // A private field is not readable from another module.
                    if !self.field_visible(&sname, name) {
                        return Err(Diag::new(
                            codes::PRIVATE_ACCESS,
                            format!("field `{name}` of `{sname}` is private; it cannot be read from this module"),
                            *span,
                        ));
                    }
                    return Ok(());
                }
                match self.infer(r) {
                    Ty::Unknown => {}
                    Ty::Named(n) if n == "range" => {
                        if crate::stdlib::signatures::method(
                            crate::stdlib::signatures::TypeClass::Range,
                            name,
                        )
                        .is_none()
                        {
                            return Err(Diag::new(
                                codes::UNDEFINED,
                                format!("range has no method `{name}`"),
                                *span,
                            ));
                        }
                    }
                    // Structs: a field read. A missing field infers `Unknown`
                    // (§17.5), so it is left to the runtime. Enums have
                    // neither fields nor methods, so a no-parentheses member
                    // on a known enum is an unknown zero-argument method and
                    // is rejected at check time, exactly as the parenthesized
                    // form is (§24).
                    Ty::Named(_) => {}
                    Ty::Enum(_) => {
                        return Err(Diag::new(
                            codes::UNDEFINED,
                            format!("enum has no method `{name}`"),
                            *span,
                        ));
                    }
                    ty => {
                        if let Some(class) = ty.type_class() {
                            if crate::stdlib::signatures::method(class, name).is_none() {
                                return Err(Diag::new(
                                    codes::UNDEFINED,
                                    format!("{} has no method `{name}`", class.name()),
                                    *span,
                                ));
                            }
                        }
                    }
                }
            }
            Expr::Index(b, i, span) => {
                self.expr(b)?;
                self.expr(i)?;
                // On a statically known map, the index must be compatible with
                // the map's key type. A list/string index stays the runtime's
                // decision, and an `Unknown` map imposes no constraint.
                if let Ty::Map(k, _) = self.infer(b) {
                    let it = self.infer(i);
                    if !it.is_key_capable() {
                        return Err(it.key_type_error(*span));
                    }
                    if !k.compatible_with(&it) {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!(
                                "map has key type `{}` but the index is `{}`",
                                k.name(),
                                it.name()
                            ),
                            *span,
                        ));
                    }
                }
            }
            Expr::List(vs, _) | Expr::Tuple(vs, _) => {
                for v in vs {
                    self.expr(v)?;
                }
            }
            Expr::Map(kvs, span) => {
                for (k, v) in kvs {
                    self.expr(k)?;
                    self.expr(v)?;
                    // A statically known non-key-capable key is rejected
                    // before any literal is constructed. `infer` cannot fail,
                    // so the diagnostic is raised here where the span is
                    // available.
                    let kt = self.infer(k);
                    if !kt.is_key_capable() {
                        return Err(kt.key_type_error(*span));
                    }
                }
            }
            Expr::Construct(name, args, ty_args, span) => {
                // Every argument expression is checked regardless of which
                // construction rule applies.
                for a in args {
                    self.expr(&a.value)?;
                }
                // Explicit type arguments on a construction are validated for
                // arity and bounds, then used to check the field values against
                // the substituted field types.
                let explicit = if ty_args.is_empty() {
                    HashMap::new()
                } else {
                    let Some(params) = self.type_type_params.get(name).cloned() else {
                        return Err(Diag::new(
                            codes::UNKNOWN_TYPE,
                            format!("`{name}` is not a generic type and takes no type arguments"),
                            *span,
                        ));
                    };
                    if params.len() != ty_args.len() {
                        return Err(Diag::new(
                            codes::UNKNOWN_TYPE,
                            format!(
                                "`{name}` expects {} type argument(s), found {}",
                                params.len(),
                                ty_args.len()
                            ),
                            *span,
                        ));
                    }
                    let bounds = self
                        .type_type_param_bounds
                        .get(name)
                        .cloned()
                        .unwrap_or_default();
                    let mut sigma = HashMap::new();
                    for (i, (p, t)) in params.iter().zip(ty_args).enumerate() {
                        let ty = self.annotation(t, *span)?;
                        if let Some(pb) = bounds.get(i) {
                            self.satisfies_bounds(&ty, pb, *span)?;
                        }
                        sigma.insert(p.clone(), ty);
                    }
                    sigma
                };
                if let Some(fields) = self.struct_fields.get(name) {
                    self.check_struct_construction(name, fields, args, *span)?;
                    // Field values are checked against the substituted field
                    // types so `Box<int> { value: "x" }` is rejected.
                    if !explicit.is_empty() {
                        self.check_construct_field_types(name, fields, args, &explicit, *span)?;
                    }
                } else if self.variants.contains_key(name) {
                    self.check_variant_construction(name, args, *span)?;
                } else {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("`{name}` is not a declared struct or enum variant"),
                        *span,
                    ));
                }
            }
            Expr::Lambda(ps, body, _) => {
                self.push();
                let saved_loop = std::mem::take(&mut self.loop_depth);
                // A lambda has no declared return type, so a `return` in its
                // body must not be checked against the enclosing function's
                // annotation. `return` inside a lambda returns from the
                // lambda (`LANGUAGE_SPEC.md` §15.4).
                let saved_return = std::mem::take(&mut self.return_type);
                // A lambda shares the function parameter model: a `mut`
                // parameter grants body capability over the bound name, and a
                // parameter annotation is checked and recorded like a
                // function's.
                for p in ps {
                    self.declare(&p.name, p.mutable, p.span)?;
                    if let Some(pty) = &p.ty {
                        let t = self.annotation(pty, p.span)?;
                        self.value_types
                            .last_mut()
                            .map(|m| m.insert(p.name.clone(), t));
                    }
                }
                let r = self.expr(body);
                self.return_type = saved_return;
                self.loop_depth = saved_loop;
                self.pop();
                r?;
            }
            Expr::Pipe(l, r, _) => {
                self.expr(l)?;
                self.expr(r)?;
            }
            Expr::Range(l, r, span) => {
                self.expr(l)?;
                self.expr(r)?;
                // Mirror the `range(a, b)` builtin's static check: a bound
                // whose type is provably not `int` is `E3001`, and `Unknown`
                // imposes no constraint (§2.3). This keeps the literal and the
                // builtin statically equivalent.
                for (which, bound) in [("start", l), ("end", r)] {
                    let actual = self.infer(bound);
                    if let Some(false) =
                        crate::stdlib::signatures::TypeClass::Int.matches_ty(&actual)
                    {
                        return Err(Diag::new(
                            codes::TYPE_MISMATCH,
                            format!("range {which} expects an int, found `{}`", actual.name()),
                            *span,
                        ));
                    }
                }
            }
            Expr::If(c, then, els, _) => {
                self.expr(c)?;
                self.block(then)?;
                if let Some(e) = els {
                    self.expr(e)?;
                }
            }
            Expr::Match(subject, arms, _) => {
                self.expr(subject)?;
                for arm in arms {
                    self.check_pattern(&arm.pattern)?;
                    self.push();
                    for b in arm.pattern.bindings() {
                        self.declare(&b, false, Span::default())?;
                    }
                    if let Some(g) = &arm.guard {
                        self.expr(g)?;
                    }
                    for s in &arm.body {
                        self.stmt(s)?;
                    }
                    self.pop();
                }
            }
            Expr::Block(body, _) => self.block(body)?,
        }
        Ok(())
    }
}
