//! Static checks: reserved names, mutability, and name resolution.
//!
//! Aura checks are conservative but complete: every name used must be
//! declared, every assignment must target a `let mut` binding, and every
//! forbidden construct (e.g. `else if`) is rejected by the parser.

use std::collections::HashMap;

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use crate::types::Ty;

/// Re-export of the shared type representation, for callers that expect
/// `check::types::Ty`.
pub use crate::types;

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
        /// Parameters in declaration order: `(name, annotation)`; the
        /// annotation is `None` when the parameter is unannotated.
        params: Vec<(String, Option<TypeExpr>)>,
    },
    /// `struct Name { fields... }`
    Struct {
        /// Name.
        name: String,
        /// Field type annotations in declaration order.
        fields: Vec<(String, TypeExpr)>,
    },
    /// `enum Name { variants... }`
    Enum {
        /// Name.
        name: String,
        /// Variant tags with payload type annotations.
        variants: Vec<(String, Vec<TypeExpr>)>,
    },
    /// `type Name = T`
    Alias {
        /// Name.
        name: String,
        /// The aliased type expression.
        target: TypeExpr,
    },
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
    /// One entry per parameter, in declaration order: `(name, annotation)`.
    params: Vec<(String, Option<Ty>)>,
}

/// The checker. Reports the first error, matching the CLI contract.
pub struct Checker {
    scopes: Vec<Scope>,
    /// Whether a `main` function was seen.
    pub has_main: bool,
    /// Every top-level function (hoisted), by name, for call resolution and
    /// static argument checking.
    functions: HashMap<String, FnSig>,
    /// Names of every top-level type (struct, enum, alias).
    types: HashMap<String, Span>,
    /// The kind of each user type: `struct`, `enum`, or `alias`.
    type_kinds: HashMap<String, String>,
    /// Field types of each struct, by struct name.
    struct_fields: HashMap<String, HashMap<String, Ty>>,
    /// Struct field names in declaration order, for positional construction.
    struct_field_order: HashMap<String, Vec<String>>,
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
    /// Targets of `type Name = T` aliases, so a transparent alias resolves to
    /// the type it names.
    alias_targets: HashMap<String, TypeExpr>,
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
            alias_targets: HashMap::new(),
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
                }
                GlobalDecl::Function { name, ret, params } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    let ret_ty = ret.as_ref().map(Ty::from_expr_lenient);
                    let param_tys = params
                        .iter()
                        .map(|(pname, pty)| {
                            let ty = pty
                                .as_ref()
                                .map(|t| Ty::from_expr_lenient(&c.resolve_type_expr_lenient(t)));
                            (pname.clone(), ty)
                        })
                        .collect();
                    c.functions.insert(
                        name.clone(),
                        FnSig {
                            ret: ret_ty,
                            params: param_tys,
                        },
                    );
                }
                GlobalDecl::Struct { name, .. } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.types.insert(name.clone(), Span::default());
                    c.type_kinds.insert(name.clone(), "struct".to_string());
                }
                GlobalDecl::Enum { name, .. } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.types.insert(name.clone(), Span::default());
                    c.type_kinds.insert(name.clone(), "enum".to_string());
                }
                GlobalDecl::Alias { name, target } => {
                    c.scopes[0].declares.insert(name.clone(), Span::default());
                    c.scopes[0].vars.insert(name.clone(), false);
                    c.types.insert(name.clone(), Span::default());
                    c.type_kinds.insert(name.clone(), "alias".to_string());
                    c.alias_targets.insert(name.clone(), target.clone());
                }
            }
        }
        // Then record fields, payloads, and variant tags, resolving aliases
        // now that every alias target is known.
        for d in decls {
            match d {
                GlobalDecl::Struct { name, fields } => {
                    let mut map = HashMap::new();
                    let mut order = Vec::new();
                    for (f, t) in fields {
                        map.insert(
                            f.clone(),
                            Ty::from_expr_lenient(&c.resolve_type_expr_lenient(t)),
                        );
                        order.push(f.clone());
                    }
                    c.struct_fields.insert(name.clone(), map);
                    c.struct_field_order.insert(name.clone(), order);
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
        let mut declared: HashMap<String, Span> = HashMap::new();
        for item in &m.items {
            match item {
                Item::Fn {
                    name,
                    span,
                    ret,
                    params,
                    ..
                } => {
                    if declared.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{name}` is already declared in this scope"),
                            *span,
                        ));
                    }
                    let ret_ty = ret.as_ref().map(Ty::from_expr_lenient);
                    // Parameter names are recorded now; parameter types are
                    // filled in by the annotation pass below, once every type
                    // name is known.
                    let param_sigs = params.iter().map(|p| (p.name.clone(), None)).collect();
                    self.functions.entry(name.clone()).or_insert(FnSig {
                        ret: ret_ty,
                        params: param_sigs,
                    });
                    self.scopes[0].declares.entry(name.clone()).or_insert(*span);
                    self.scopes[0].vars.insert(name.clone(), false);
                }
                Item::Const { name, span, .. } => {
                    if declared.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::REDECLARED,
                            format!("`{name}` is already declared in this scope"),
                            *span,
                        ));
                    }
                    self.scopes[0].declares.entry(name.clone()).or_insert(*span);
                    self.scopes[0].vars.insert(name.clone(), false);
                    let ord = self.const_order.len();
                    self.const_order.insert(name.clone(), ord);
                }
                Item::Struct { name, span, .. } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "struct".to_string());
                }
                Item::Enum {
                    name,
                    variants,
                    span,
                } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "enum".to_string());
                    for (tag, _) in variants {
                        if let Some(prev) = self.variants.get(tag) {
                            return Err(Diag::new(
                                codes::DUPLICATE_VARIANT,
                                if prev == name {
                                    format!("variant `{tag}` is declared more than once")
                                } else {
                                    format!(
                                        "variant `{tag}` is already declared by enum `{prev}`; variant names must be unique across the program"
                                    )
                                },
                                *span,
                            ));
                        }
                        self.variants.insert(tag.clone(), name.clone());
                    }
                }
                Item::Alias { name, target, span } => {
                    if self.types.insert(name.clone(), *span).is_some() {
                        return Err(Diag::new(
                            codes::DUPLICATE_TYPE,
                            format!("type `{name}` is already declared"),
                            *span,
                        ));
                    }
                    self.type_kinds.insert(name.clone(), "alias".to_string());
                    self.alias_targets.insert(name.clone(), target.clone());
                }
                Item::Use { .. } | Item::Expr(..) => {}
            }
        }
        // Validate every written type annotation now that all names are known.
        for item in &m.items {
            match item {
                Item::Struct { name, fields, .. } => {
                    let mut map = HashMap::new();
                    let mut order = Vec::new();
                    for (fname, fty) in fields {
                        let ty = self.annotation(fty, Span::default())?;
                        map.insert(fname.clone(), ty);
                        order.push(fname.clone());
                    }
                    self.struct_fields.insert(name.clone(), map);
                    self.struct_field_order.insert(name.clone(), order);
                }
                Item::Enum { variants, .. } => {
                    for (tag, payload) in variants {
                        let mut tys = Vec::with_capacity(payload.len());
                        for pty in payload {
                            tys.push(self.annotation(pty, Span::default())?);
                        }
                        self.variant_payloads.insert(tag.clone(), tys);
                    }
                }
                Item::Alias { target, .. } => {
                    self.annotation(target, Span::default())?;
                }
                Item::Fn {
                    name, params, ret, ..
                } => {
                    let mut param_tys = Vec::with_capacity(params.len());
                    for p in params {
                        let ty = match &p.ty {
                            Some(pty) => Some(self.annotation(pty, p.span)?),
                            None => None,
                        };
                        param_tys.push((p.name.clone(), ty));
                    }
                    let ret_ty = match ret {
                        Some(rt) => Some(self.annotation(rt, Span::default())?),
                        None => None,
                    };
                    if let Some(sig) = self.functions.get_mut(name) {
                        sig.ret = ret_ty;
                        sig.params = param_tys;
                    }
                }
                _ => {}
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

    /// Record a REPL global so later submissions can see it. Functions are
    /// also made callable.
    pub fn add_global(&mut self, name: &str, is_fn: bool) {
        self.scopes[0]
            .declares
            .insert(name.to_string(), Span::default());
        self.scopes[0].vars.insert(name.to_string(), false);
        if is_fn {
            self.functions.insert(
                name.to_string(),
                FnSig {
                    ret: None,
                    params: Vec::new(),
                },
            );
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
        if name.starts_with('_') {
            // leading-underscore names are allowed but flagged if used
        }
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
        if scope.declares.contains_key(name) {
            return Err(Diag::new(
                codes::REDECLARED,
                format!("`{name}` is already declared in this scope"),
                span,
            ));
        }
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
    fn check_user_call(&self, name: &str, sig: &FnSig, args: &[Arg], span: Span) -> Result<()> {
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
                    let Some(param_index) = sig.params.iter().position(|(p, _)| p == param_name)
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
        for (param_index, (param_name, _)) in sig.params.iter().enumerate() {
            if filled[param_index].is_none() {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!("missing argument for parameter `{param_name}` of `{name}`"),
                    span,
                ));
            }
        }

        // Type-check each parameter against the argument mapped to it.
        for (param_index, (_, expected)) in sig.params.iter().enumerate() {
            let Some(expected) = expected else { continue };
            let Some(arg_index) = filled[param_index] else {
                continue;
            };
            let actual = self.infer(&args[arg_index].value);
            if !expected.compatible_with(&actual) {
                return Err(Diag::new(
                    codes::TYPE_MISMATCH,
                    format!(
                        "`{name}` parameter `{}` expects `{}`, found `{}`",
                        sig.params[param_index].0,
                        expected.name(),
                        actual.name()
                    ),
                    span,
                ));
            }
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
                params,
                ret,
                body,
                ..
            } => {
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
                    self.declare(&p.name, false, p.span)?;
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
                    Some(rt) => Some(self.annotation(rt, Span::default())?),
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
                self.pop();
            }
            Item::Const {
                name,
                ann,
                value,
                span,
            } => {
                let saved = self.active_const;
                self.active_const = self.const_order.get(name).copied();
                let r = self.expr(value);
                self.active_const = saved;
                r?;
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
                }
            }
            Item::Expr(e, _) => self.expr(e)?,
            Item::Struct { .. } | Item::Enum { .. } | Item::Alias { .. } | Item::Use { .. } => {}
        }
        Ok(())
    }

    /// Validate a pattern: every variant pattern names a declared variant,
    /// and a pattern never binds the same name twice.
    fn check_pattern(&self, pat: &Pattern) -> Result<()> {
        match pat {
            Pattern::Variant(tag, ps) => {
                if !self.variants.contains_key(tag) && !self.types.contains_key(tag) {
                    return Err(Diag::new(
                        codes::UNKNOWN_TYPE,
                        format!("`{tag}` is not a declared enum variant"),
                        Span::default(),
                    ));
                }
                for p in ps {
                    self.check_pattern(p)?;
                }
            }
            Pattern::List(ps) => {
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
                    Span::default(),
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
            Expr::List(items, _) => {
                let mut elem = Ty::Unknown;
                for item in items {
                    let t = self.infer(item);
                    if !matches!(t, Ty::Unknown) {
                        elem = t;
                        break;
                    }
                }
                Ty::List(Box::new(elem))
            }
            Expr::Map(entries, _) => {
                let mut val = Ty::Unknown;
                for (_, v) in entries {
                    let t = self.infer(v);
                    if !matches!(t, Ty::Unknown) {
                        val = t;
                        break;
                    }
                }
                Ty::Map(Box::new(val))
            }
            Expr::Construct(name, _, _) => {
                if self.variants.contains_key(name) {
                    Ty::Unknown
                } else {
                    Ty::Named(name.clone())
                }
            }
            Expr::Unary(UnOp::Not, _, _) => Ty::Bool,
            Expr::Unary(UnOp::Neg, inner, _) => self.infer(inner),
            Expr::Binary(op, l, _, _) => match op {
                BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::And
                | BinOp::Or => Ty::Bool,
                _ => self.infer(l),
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
            Expr::Call(f, _, _) => match f.as_ref() {
                Expr::Name(name, _) => {
                    if let Some(sig) = self.functions.get(name) {
                        sig.ret.clone().unwrap_or(Ty::Unknown)
                    } else if let Some(sig) = crate::stdlib::signatures::builtin(name) {
                        sig.returns.ty()
                    } else {
                        Ty::Unknown
                    }
                }
                _ => Ty::Unknown,
            },
            Expr::Method(recv, name, _, _) => {
                if let Some(class) = self.infer(recv).type_class() {
                    if let Some(sig) = crate::stdlib::signatures::method(class, name) {
                        return sig.returns.ty();
                    }
                }
                Ty::Unknown
            }
            Expr::Field(recv, name, _) => {
                // FEATURE_003: a field read on a statically known struct has
                // that struct's declared field type. The field table stores
                // alias-resolved types, so the propagated `Ty` is the
                // semantically resolved one. Anything else (an `Unknown`
                // receiver, a primitive, a list, a map, an enum, `range`, or a
                // struct without that field) stays `Unknown` — the checker
                // never speculates (`LANGUAGE_SPEC.md` §17.5).
                if let Ty::Named(sname) = self.infer(recv) {
                    if let Some(fty) = self.struct_fields.get(&sname).and_then(|m| m.get(name)) {
                        return fty.clone();
                    }
                }
                Ty::Unknown
            }
            Expr::Pipe(_, _, _)
            | Expr::Index(_, _, _)
            | Expr::Tuple(_, _)
            | Expr::Lambda(_, _, _) => Ty::Unknown,
        }
    }

    /// Validate a builtin call against its shared signature.
    fn check_builtin_call(
        &self,
        sig: &crate::stdlib::signatures::Signature,
        args: &[Arg],
        span: Span,
    ) -> Result<()> {
        if let Some(message) = sig.check_arity(args.len()) {
            return Err(Diag::new(codes::TYPE_MISMATCH, message, span));
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
            Ty::Named(n) if n == "range" => Ok(Some(TypeClass::Other)),
            Ty::Named(_) => Err(Diag::new(
                codes::UNDEFINED,
                format!("struct has no method `{name}`"),
                span,
            )),
            other => Ok(other.type_class()),
        }
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
        Ty::from_expr(&resolved, &self.type_kinds, span)
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
            TypeExpr::Named(n) => match self.alias_targets.get(n) {
                Some(target) => {
                    if visiting.iter().any(|v| v == n) {
                        return Err(Diag::new(
                            codes::UNKNOWN_TYPE,
                            format!("recursive type alias `{n}` has no concrete target"),
                            span,
                        ));
                    }
                    visiting.push(n.clone());
                    let resolved = self.resolve_type_expr(target, span, visiting)?;
                    visiting.pop();
                    resolved
                }
                None => t.clone(),
            },
            TypeExpr::List(inner) => {
                TypeExpr::List(Box::new(self.resolve_type_expr(inner, span, visiting)?))
            }
            TypeExpr::Map(k, v) => TypeExpr::Map(
                Box::new(self.resolve_type_expr(k, span, visiting)?),
                Box::new(self.resolve_type_expr(v, span, visiting)?),
            ),
            TypeExpr::Optional(inner) => {
                TypeExpr::Optional(Box::new(self.resolve_type_expr(inner, span, visiting)?))
            }
            _ => t.clone(),
        })
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
                        visiting.push(n.clone());
                        let resolved = go(checker, target, visiting);
                        visiting.pop();
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
                TypeExpr::Optional(inner) => {
                    TypeExpr::Optional(Box::new(go(checker, inner, visiting)))
                }
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
                    // Without an annotation, still remember any inferred type
                    // (a user struct for field checking, or a builtin type so
                    // method calls on it can be validated). `Unknown` is not
                    // recorded.
                    let inferred = self.infer(value);
                    if !matches!(inferred, Ty::Unknown) {
                        self.value_types
                            .last_mut()
                            .map(|m| m.insert(name.clone(), inferred));
                    }
                }
                self.declare(name, *mutable, *span)?;
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
                    Expr::Index(base, idx, _) => {
                        self.expr(base)?;
                        self.expr(idx)?;
                    }
                    Expr::Field(base, fname, fspan) => {
                        self.expr(base)?;
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
                        if !expected.compatible_with(&actual) {
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
                    if let FPart::Expr(inner) = p {
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
            Expr::Call(f, args, span) => {
                match f.as_ref() {
                    Expr::Name(name, nspan) => {
                        // A direct call must resolve to a function, a builtin,
                        // or a callable local. Functions are hoisted, so
                        // forward references resolve.
                        if self.resolves_to_user_function(name) {
                            // A directly resolved top-level function: check the
                            // call against its declared signature (§6.5, §15.7).
                            if let Some(sig) = self.functions.get(name) {
                                self.check_user_call(name, sig, args, *span)?;
                            }
                        } else if let Some(sig) = crate::stdlib::signatures::builtin(name) {
                            // Builtins are positional; named arguments require
                            // a resolved user-function parameter list.
                            self.reject_named_args(name, args, *span)?;
                            self.check_builtin_call(sig, args, *span)?;
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
            Expr::Method(r, name, args, span) => {
                self.expr(r)?;
                if !crate::stdlib::signatures::method_exists_anywhere(name) {
                    return Err(Diag::new(
                        codes::UNDEFINED,
                        format!("no method `{name}` on any type"),
                        *span,
                    ));
                }
                // Methods are positional; named arguments are out of scope.
                self.reject_named_args(name, args, *span)?;
                self.check_method_call(r, name, args, *span)?;
                for a in args {
                    self.expr(&a.value)?;
                }
            }
            Expr::Field(r, name, span) => {
                self.expr(r)?;
                // `receiver.name` without parentheses is a field read on a
                // struct, and a zero-argument method call on any other known
                // receiver kind (§24). A known struct field needs no method
                // check; an enum has neither fields nor methods; `range` has
                // only `len`; primitive/list/map receivers are method calls.
                match self.infer(r) {
                    Ty::Unknown => {}
                    Ty::Named(n) if n == "range" => {
                        if crate::stdlib::signatures::method(
                            crate::stdlib::signatures::TypeClass::Other,
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
                    // Structs: a field read. Enums: neither fields nor methods;
                    // the runtime reports. Both are left to runtime here.
                    Ty::Named(_) | Ty::Enum(_) => {}
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
            Expr::Index(b, i, _) => {
                self.expr(b)?;
                self.expr(i)?;
            }
            Expr::List(vs, _) | Expr::Tuple(vs, _) => {
                for v in vs {
                    self.expr(v)?;
                }
            }
            Expr::Map(kvs, _) => {
                for (k, v) in kvs {
                    self.expr(k)?;
                    self.expr(v)?;
                }
            }
            Expr::Construct(name, args, span) => {
                // Every argument expression is checked regardless of which
                // construction rule applies.
                for a in args {
                    self.expr(&a.value)?;
                }
                if let Some(fields) = self.struct_fields.get(name) {
                    self.check_struct_construction(name, fields, args, *span)?;
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
                for p in ps {
                    self.declare(p, false, Span::default())?;
                }
                let r = self.expr(body);
                self.loop_depth = saved_loop;
                self.pop();
                r?;
            }
            Expr::Pipe(l, r, _) => {
                self.expr(l)?;
                self.expr(r)?;
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
