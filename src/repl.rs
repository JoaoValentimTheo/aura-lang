//! A minimal, honest REPL.
//!
//! Each submission is parsed and checked with the real front end, then
//! executed in a persistent interpreter session. Declarations register
//! functions, structs, and enums; expressions are evaluated and printed.
//! `:quit` (or Ctrl-D) exits; `:help` lists commands.

use std::io::{BufRead, Write};

use crate::ast::{Item, Stmt, TypeParam};
use crate::check::{GlobalDecl, MethodDecl, ParamDecl};
use crate::error::{Diag, SourceDiagnostic, Span};
use crate::resolve::{resolve_stmt, resolve_with_session, session_items, Session};
use crate::run::{Ctl, Interp};
use crate::source::{SourceId, SourceMap};

/// The persistent state of one REPL session, independent of how lines are
/// obtained. Both the buffered driver and the interactive (rustyline) driver
/// share this, so they cannot drift.
pub struct SessionEngine {
    interp: Interp,
    decls: Vec<GlobalDecl>,
    session: Session,
    /// Source records for completed submissions, retained so locations from
    /// earlier cells never alias later ones.
    sources: SourceMap,
    /// One-based deterministic display sequence for completed submissions.
    next_submission: usize,
    /// The in-progress multi-line submission.
    pub(crate) pending: String,
}

/// What to do after feeding a line.
pub enum Step {
    /// Keep reading (the submission is incomplete, or the line was a command).
    Continue,
    /// The session should end (`:quit` or `:help`? no — `:quit` only).
    Quit,
}

impl SessionEngine {
    /// Create a fresh session engine.
    #[must_use]
    pub fn new() -> SessionEngine {
        SessionEngine {
            interp: Interp::new(),
            decls: Vec::new(),
            session: Session::default(),
            sources: SourceMap::new(),
            next_submission: 1,
            pending: String::new(),
        }
    }

    /// The prompt for the next line: the continuation prompt while a
    /// submission is pending, the primary prompt otherwise.
    #[must_use]
    pub fn prompt(&self) -> &'static str {
        if self.pending.is_empty() {
            "aura> "
        } else {
            "  ... "
        }
    }

    /// Feed one line. Returns whether the session should continue or end, and
    /// writes any output (echoed values, diagnostics, command help) to `out`.
    pub fn feed_line<W: Write>(&mut self, line: &str, out: &mut W) -> Step {
        // Commands are only recognized at the start of a fresh submission.
        if self.pending.is_empty() {
            match line.trim() {
                "" => return Step::Continue,
                ":quit" => return Step::Quit,
                ":help" => {
                    let _ = writeln!(out, ":help     show this help");
                    let _ = writeln!(out, ":quit     exit");
                    let _ = writeln!(out, "Otherwise type an expression or a declaration.");
                    return Step::Continue;
                }
                _ => {}
            }
        }
        self.pending.push_str(line);
        self.pending.push('\n');
        // Wait for balanced delimiters before parsing.
        if unbalanced(&self.pending) {
            return Step::Continue;
        }
        let source = std::mem::take(&mut self.pending);
        let source_id = self
            .sources
            .add(format!("<repl:{}>", self.next_submission), source.clone());
        self.next_submission += 1;
        eval_line(
            &mut self.interp,
            &mut self.decls,
            &mut self.session,
            source_id,
            &source,
            out,
        );
        Step::Continue
    }
}

impl Default for SessionEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// Run an interactive session on stdin/stdout.
///
/// The session runs on the large interpreter stack, so a deeply nested but
/// valid submission is bounded by the language nesting limit (`E1015`) rather
/// than by the platform's default main-thread stack.
///
/// # Errors
/// Returns a diagnostic only for unrecoverable I/O failures.
pub fn run() -> Result<(), Diag> {
    crate::on_large_stack(|| {
        // With a terminal, use rustyline for line editing and history. Without
        // one (piped input, tests), fall back to the buffered reader so
        // behavior is identical and testable.
        #[cfg(feature = "repl")]
        if std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            return run_interactive();
        }
        let reader = std::io::BufReader::new(std::io::stdin());
        let writer = std::io::BufWriter::new(std::io::stdout());
        run_with(reader, writer)
    })
}

/// The rustyline-backed interactive driver: editable current line and Up/Down
/// history, sharing [`SessionEngine`] with the buffered driver.
#[cfg(feature = "repl")]
fn run_interactive() -> Result<(), Diag> {
    use rustyline::error::ReadlineError;
    // Keep the persistent session off the terminal thread's stack. The REPL
    // deliberately accepts AST-valid inputs near the parser depth boundary;
    // provenance state must not reduce the host stack available to parsing.
    let mut engine = Box::new(SessionEngine::new());
    let mut rl = rustyline::DefaultEditor::new().map_err(|e| {
        Diag::new(
            crate::error::codes::EXPECTED,
            e.to_string(),
            Span::default(),
        )
    })?;
    let mut out = std::io::stdout();
    let _ = writeln!(out, "Aura {} REPL — :help for commands", crate::VERSION);
    loop {
        match rl.readline(engine.prompt()) {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                let mut sink = std::io::stdout();
                match engine.feed_line(&line, &mut sink) {
                    Step::Continue => {}
                    Step::Quit => return Ok(()),
                }
            }
            // Ctrl-C cancels the current pending submission without killing the
            // process; the include a newline so the next prompt starts clean.
            Err(ReadlineError::Interrupted) => {
                engine.pending.clear();
                let _ = writeln!(out);
            }
            Err(ReadlineError::Eof) => return Ok(()),
            Err(e) => {
                return Err(Diag::new(
                    crate::error::codes::EXPECTED,
                    e.to_string(),
                    Span::default(),
                ))
            }
        }
    }
}

/// Run a session reading from `reader` and writing to `writer`.
///
/// This is the testable core of the REPL: it performs no direct I/O, so
/// callers can drive it with in-memory buffers.
///
/// # Errors
/// Returns a diagnostic only for unrecoverable I/O failures.
pub fn run_with<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> Result<(), Diag> {
    // The buffered driver is also used directly by tests on ordinary test
    // threads. Heap-owning the persistent session keeps the parser's existing
    // nesting boundary independent of SessionEngine's bookkeeping size.
    let mut engine = Box::new(SessionEngine::new());
    let _ = writeln!(writer, "Aura {} REPL — :help for commands", crate::VERSION);
    loop {
        let _ = write!(writer, "{}", engine.prompt());
        let _ = writer.flush();
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            let _ = writeln!(writer);
            return Ok(());
        }
        match engine.feed_line(&line, &mut writer) {
            Step::Continue => {}
            Step::Quit => return Ok(()),
        }
    }
}

/// Whether a submission is *incomplete* and the REPL should keep reading
/// continuation lines.
///
/// A small lexical scan over the current buffer — not a second parser — tracks
/// the delimiter stack (`()`, `[]`, `{}`), string/f-string body, line comments,
/// and multiline comments. It reports incomplete when a delimiter is open, a
/// multiline comment or string is unterminated, or a backslash continues a
/// line. A submission that is *complete* but malformed is handled by the real
/// parser, which reports its own diagnostic (`LANGUAGE_SPEC.md` §28.4).
fn unbalanced(src: &str) -> bool {
    let mut stack: Vec<char> = Vec::new();
    let mut in_str: Option<char> = None;
    let mut in_block_comment = false;
    let bytes = src.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i] as char;
        if in_block_comment {
            if bytes[i..].starts_with(b"--!>") {
                in_block_comment = false;
                i += 4;
                continue;
            }
            i += 1;
            continue;
        }
        match in_str {
            Some(q) => {
                if c == q {
                    in_str = None;
                }
            }
            None => match c {
                '"' | '\'' => in_str = Some(c),
                '(' | '[' | '{' => stack.push(c),
                ')' => {
                    if stack.last() == Some(&'(') {
                        stack.pop();
                    }
                }
                ']' => {
                    if stack.last() == Some(&'[') {
                        stack.pop();
                    }
                }
                '}' => {
                    if stack.last() == Some(&'{') {
                        stack.pop();
                    }
                }
                '#' => {
                    // line comment: skip to end of line
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                    continue;
                }
                '<' if bytes[i..].starts_with(b"<!--") => {
                    in_block_comment = true;
                    i += 4;
                    continue;
                }
                _ => {}
            },
        }
        i += 1;
    }
    // Incomplete while any delimiter is open or a multiline comment is still
    // open. An unterminated string is *not* a continuation: Aura strings do
    // not span lines, so a typo'd quote must reach the parser and produce its
    // real `E1004` rather than hanging the prompt.

    !stack.is_empty() || in_block_comment
}

fn eval_line<W: Write>(
    interp: &mut Interp,
    decls: &mut Vec<GlobalDecl>,
    session: &mut Session,
    source_id: SourceId,
    source: &str,
    writer: &mut W,
) {
    // Try a single statement first, so `let mut x = ...` works at the top
    // level of the REPL. Fall back to module items (fn/struct/enum/use) and
    // then to a bare expression.
    if let Ok(stmt) = crate::parse::parse_stmt(source) {
        // Resolve free references against the session's modules, so a bare
        // `shapes::area` or a `use`-imported name is canonicalized (and its
        // visibility enforced) exactly as in a whole module.
        let stmt = match resolve_stmt(stmt, session) {
            Ok(s) => s,
            Err(e) => {
                emit_diag(writer, source_id, e);
                return;
            }
        };
        let mut checker = crate::check::Checker::with_declarations(decls);
        if let Err(e) = checker.check_stmt(&stmt) {
            emit_diag(writer, source_id, e);
            return;
        }
        // Capture the inferred binding type here, while the checker still has
        // the statement checked and the session declarations loaded.
        let inferred_ty = checker.let_binding_type(&stmt);
        let executed = match interp.exec_stmt_globals_iterative(&stmt) {
            // A bare expression echoes its value; declarations stay silent.
            Ok(Ctl::Val(v)) if matches!(stmt, Stmt::Expr(..)) => {
                let _ = writeln!(writer, "{}", v.display());
                true
            }
            Ok(Ctl::Val(_)) => true,
            // A residual control-flow signal at the top level is reported with
            // the same diagnostic the other entry points produce, rather than
            // being silently swallowed (`LANGUAGE_SPEC.md` §28.5).
            Ok(other) => {
                if let Err(d) = interp.finish_global(other) {
                    emit_diag(writer, source_id, d);
                }
                false
            }
            Err(e) => {
                emit_diag(writer, source_id, interp.uncaught_diag(e));
                false
            }
        };
        // A `let` introduces a binding; persist it only after it executed.
        // The declared annotation is used when present; otherwise the
        // statically inferred nominal type (a user struct, or a builtin type)
        // is persisted so later submissions keep the same type information a
        // single module would have (`LANGUAGE_SPEC.md` §3.2, §17.6).
        if let Stmt::Let {
            name, mutable, ann, ..
        } = &stmt
        {
            // An ordinary `let`/`let mut` may shadow an earlier session
            // binding (`LANGUAGE_SPEC.md` §16.3). Persist the *latest*
            // declaration so later submissions resolve the shadowing binding's
            // mutability and type, not the binding it replaced.
            let ty = ann.clone().or(inferred_ty);
            let decl = GlobalDecl::Binding {
                name: name.clone(),
                mutable: *mutable,
                ty,
            };
            if let Some(slot) = decls
                .iter_mut()
                .find(|d| matches!(d, GlobalDecl::Binding { name: n, .. } if n == name))
            {
                *slot = decl;
            } else {
                decls.push(decl);
            }
        }
        // A destructuring `let` persists each name it bound, but only when the
        // statement actually executed: a failed match must not alter the
        // session (§4.7). Destructured names carry no annotation, and each
        // name may shadow an earlier binding like a simple `let`.
        if executed {
            if let Stmt::LetPattern { pattern, .. } = &stmt {
                for name in pattern.bindings() {
                    let decl = GlobalDecl::Binding {
                        name: name.clone(),
                        mutable: false,
                        ty: None,
                    };
                    if let Some(slot) = decls
                        .iter_mut()
                        .find(|d| matches!(d, GlobalDecl::Binding { name: n, .. } if n == &name))
                    {
                        *slot = decl;
                    } else {
                        decls.push(decl);
                    }
                }
            }
        }
        return;
    }

    match crate::parse::parse(source) {
        Ok(module) => {
            // Modules are real boundaries, so resolve the submission against
            // the session's modules before checking: a `pub` item becomes
            // reachable by its canonical path, a private one is `E2018`.
            let (module, additions) = match resolve_with_session(module, session) {
                Ok(m) => m,
                Err(e) => {
                    emit_diag(writer, source_id, e);
                    return;
                }
            };
            let mut checker = crate::check::Checker::with_declarations(decls);
            if let Err(e) = checker.check_mode(&module, crate::CompileMode::Module) {
                emit_diag(writer, source_id, e);
                return;
            }
            // Execute first; only persist declarations the session accepted.
            for item in &module.items {
                let result = match item {
                    Item::Expr(expr, _) => match interp.eval_globals_iterative(expr) {
                        Ok(ctl) => match interp.finish_global(ctl) {
                            Ok(v) => {
                                let _ = writeln!(writer, "{}", v.display());
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(interp.uncaught_diag(e)),
                    },
                    other => interp.run_item_iterative(other),
                };
                if let Err(e) = result {
                    emit_diag(writer, source_id, e);
                    return;
                }
            }
            for item in &module.items {
                for d in declarations_of(item) {
                    if !decls.iter().any(|e| same_decl(e, &d)) {
                        decls.push(d);
                    }
                }
            }
            // Persist the canonical declarations so later submissions resolve
            // and visibility-check them against the same module tree.
            for item in session_items(&module) {
                if !session.items.contains(&item) {
                    session.items.push(item);
                }
            }
            for imp in additions.imports {
                if !session.imports.contains(&imp) {
                    session.imports.push(imp);
                }
            }
        }
        Err(e) => {
            // Last resort: a bare expression.
            match crate::parse::parse_expr(source) {
                Ok(expr) => match interp.eval_globals_iterative(&expr) {
                    Ok(Ctl::Val(v)) => {
                        let _ = writeln!(writer, "{}", v.display());
                    }
                    Ok(_) => {}
                    Err(e) => {
                        emit_diag(writer, source_id, e);
                    }
                },
                Err(_) => {
                    emit_diag(writer, source_id, e);
                }
            }
        }
    }
}

/// Attach the current submission identity at the REPL source boundary while
/// preserving the REPL's existing compact `E####: ...` presentation.
fn emit_diag<W: Write>(writer: &mut W, source: SourceId, diag: Diag) {
    let diag = SourceDiagnostic::new(diag, source);
    let _ = writeln!(writer, "{}", diag.diagnostic());
}

/// Whether two method declaration lists are the same set: same names and same
/// ordered parameter type annotations, in any order. Used to decide whether an
/// `impl` re-submission is the same declaration or a new overload.
fn methods_identical(a: &[MethodDecl], b: &[MethodDecl]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().all(|m| {
        b.iter().any(|n| {
            n.name == m.name
                && n.ret == m.ret
                && n.params.len() == m.params.len()
                && n.params
                    .iter()
                    .zip(&m.params)
                    .all(|(x, y)| x.ty == y.ty && x.mutable == y.mutable)
        })
    })
}

/// Whether two session declarations are the same one (so a re-submission does
/// not duplicate it). A declaration is identified by its name **and its kind**:
/// a function, a constant, and a struct that share a name are distinct
/// declarations in distinct namespaces, exactly as in a module, so a `struct S`
/// after a `fn S` must still be recorded. An `impl` is identified by its target
/// and method set; a trait by its name.
fn same_decl(a: &GlobalDecl, b: &GlobalDecl) -> bool {
    match (a, b) {
        (
            GlobalDecl::Impl {
                target: at,
                trait_name: an,
                owner: ao,
                methods: am,
                ..
            },
            GlobalDecl::Impl {
                target: bt,
                trait_name: bn,
                owner: bo,
                methods: bm,
                ..
            },
        ) => {
            // Two `impl` blocks are the same declaration only when they define
            // the identical method signature set. A block that adds a new
            // overload (`LANGUAGE_SPEC.md` §15.7) is a distinct declaration
            // and must be persisted so the overload set grows across
            // submissions.
            at == bt && an == bn && ao == bo && methods_identical(am, bm)
        }
        (GlobalDecl::Trait { name: an, .. }, GlobalDecl::Trait { name: bn, .. }) => an == bn,
        (GlobalDecl::Impl { .. }, _)
        | (_, GlobalDecl::Impl { .. })
        | (GlobalDecl::Trait { .. }, _)
        | (_, GlobalDecl::Trait { .. }) => false,
        (GlobalDecl::Binding { name: an, .. }, GlobalDecl::Binding { name: bn, .. }) => an == bn,
        (
            GlobalDecl::Function {
                name: an,
                ret: ar,
                params: ap,
                ..
            },
            GlobalDecl::Function {
                name: bn,
                ret: br,
                params: bp,
                ..
            },
        ) => {
            // Two function declarations are the same only when they share a
            // name AND an identical parameter-type list (`LANGUAGE_SPEC.md`
            // §15.7). A same-name declaration with different parameter types is
            // a new overload and must be persisted so the set grows.
            an == bn
                && ar == br
                && ap.len() == bp.len()
                && ap
                    .iter()
                    .zip(bp)
                    .all(|(x, y)| x.ty == y.ty && x.mutable == y.mutable)
        }
        (GlobalDecl::Struct { name: an, .. }, GlobalDecl::Struct { name: bn, .. }) => an == bn,
        (GlobalDecl::Enum { name: an, .. }, GlobalDecl::Enum { name: bn, .. }) => an == bn,
        (GlobalDecl::Alias { name: an, .. }, GlobalDecl::Alias { name: bn, .. }) => an == bn,
        _ => false,
    }
}

/// The session declarations a top-level item introduces.
fn declarations_of(item: &Item) -> Vec<GlobalDecl> {
    fn tp(params: &[TypeParam]) -> Vec<(String, Vec<String>)> {
        params
            .iter()
            .map(|p| (p.name.clone(), p.bounds.clone()))
            .collect()
    }
    match item {
        Item::Fn {
            name,
            type_params,
            ret,
            params,
            ..
        } => vec![GlobalDecl::Function {
            name: name.clone(),
            ret: ret.clone(),
            params: params.iter().map(ParamDecl::from_param).collect(),
            type_params: tp(type_params),
        }],
        Item::Struct {
            name,
            type_params,
            fields,
            ..
        } => vec![GlobalDecl::Struct {
            name: name.clone(),
            type_params: tp(type_params),
            fields: fields
                .iter()
                .map(|f| (f.name.clone(), f.ty.clone(), f.public))
                .collect(),
        }],
        Item::Enum { name, variants, .. } => vec![GlobalDecl::Enum {
            name: name.clone(),
            variants: variants
                .iter()
                .map(|v| (v.tag.clone(), v.payload.clone()))
                .collect(),
        }],
        Item::Alias {
            name,
            type_params,
            target,
            ..
        } => vec![GlobalDecl::Alias {
            name: name.clone(),
            type_params: tp(type_params),
            target: target.clone(),
        }],
        Item::Const { name, .. } => vec![GlobalDecl::Binding {
            name: name.clone(),
            mutable: false,
            ty: None,
        }],
        Item::Use { .. } | Item::Expr(..) | Item::Module { .. } => Vec::new(),
        Item::Impl {
            target,
            type_params,
            trait_name,
            methods,
            owner,
            ..
        } => vec![GlobalDecl::Impl {
            target: target.clone(),
            type_params: tp(type_params),
            trait_name: trait_name.clone(),
            owner: owner.clone(),
            methods: method_decls(methods),
        }],
        Item::Trait {
            name,
            type_params,
            methods,
            ..
        } => vec![GlobalDecl::Trait {
            name: name.clone(),
            type_params: tp(type_params),
            methods: method_decls(methods),
        }],
    }
}

/// The declarations of each `fn` in an `impl` or `trait` block, for REPL
/// persistence.
fn method_decls(methods: &[Item]) -> Vec<MethodDecl> {
    methods
        .iter()
        .filter_map(|m| match m {
            Item::Fn {
                name,
                ret,
                params,
                public,
                ..
            } => Some(MethodDecl {
                name: name.clone(),
                ret: ret.clone(),
                params: params.iter().map(ParamDecl::from_param).collect(),
                public: *public,
            }),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod provenance_tests {
    use super::*;

    #[test]
    fn completed_submissions_receive_distinct_stable_source_ids() {
        let mut engine = SessionEngine::new();
        let mut out = Vec::new();

        assert!(matches!(
            engine.feed_line("let x = 1", &mut out),
            Step::Continue
        ));
        assert!(matches!(
            engine.feed_line("let y = 2", &mut out),
            Step::Continue
        ));

        let sources: Vec<_> = engine.sources.iter().collect();
        assert_eq!(sources.len(), 2);
        assert_eq!(sources[0].name(), "<repl:1>");
        assert_eq!(sources[1].name(), "<repl:2>");
        assert_ne!(sources[0].id(), sources[1].id());
        assert_eq!(engine.sources.order(sources[0].id()), Some(0));
        assert_eq!(engine.sources.order(sources[1].id()), Some(1));
    }
}
