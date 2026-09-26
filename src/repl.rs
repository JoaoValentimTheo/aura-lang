//! A minimal, honest REPL.
//!
//! Each submission is parsed and checked with the real front end, then
//! executed in a persistent interpreter session. Declarations register
//! functions, structs, and enums; expressions are evaluated and printed.
//! `:quit` (or Ctrl-D) exits; `:help` lists commands.

use std::io::{BufRead, Write};

use crate::ast::{Item, Stmt};
use crate::check::GlobalDecl;
use crate::error::Diag;
use crate::run::{Ctl, Interp};

/// Run an interactive session on stdin/stdout.
///
/// The session runs on the large interpreter stack, so a deeply nested but
/// valid submission is bounded by the language nesting limit (`E1015`) rather
/// than by the platform's default main-thread stack.
///
/// # Errors
/// Returns a diagnostic only for unrecoverable I/O failures.
pub fn run() -> Result<(), Diag> {
    // Run the session on the large stack, with buffered owned handles so no
    // borrowed lock crosses the thread boundary.
    crate::on_large_stack(|| {
        let reader = std::io::BufReader::new(std::io::stdin());
        let writer = std::io::BufWriter::new(std::io::stdout());
        run_with(reader, writer)
    })
}

/// Run a session reading from `reader` and writing to `writer`.
///
/// This is the testable core of the REPL: it performs no direct I/O, so
/// callers can drive it with in-memory buffers.
///
/// # Errors
/// Returns a diagnostic only for unrecoverable I/O failures.
pub fn run_with<R: BufRead, W: Write>(mut reader: R, mut writer: W) -> Result<(), Diag> {
    let mut interp = Interp::new();
    let mut decls: Vec<GlobalDecl> = Vec::new();
    let _ = writeln!(writer, "Aura {} REPL — :help for commands", crate::VERSION);
    let mut pending = String::new();
    loop {
        let prompt = if pending.is_empty() {
            "aura> "
        } else {
            "  ... "
        };
        let _ = write!(writer, "{prompt}");
        let _ = writer.flush();
        let mut line = String::new();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            let _ = writeln!(writer);
            return Ok(());
        }
        let trimmed = line.trim();
        if pending.is_empty() {
            match trimmed {
                "" => continue,
                ":quit" | ":q" | ":exit" => return Ok(()),
                ":help" => {
                    let _ = writeln!(writer, ":help     show this help");
                    let _ = writeln!(writer, ":quit     exit");
                    let _ = writeln!(writer, "Otherwise type an expression or a declaration.");
                    continue;
                }
                _ => {}
            }
        }
        pending.push_str(&line);
        // Wait for balanced braces before parsing.
        if unbalanced(&pending) {
            continue;
        }
        let source = std::mem::take(&mut pending);
        eval_line(&mut interp, &mut decls, &source, &mut writer);
    }
}

fn unbalanced(src: &str) -> bool {
    let mut depth = 0i32;
    let mut in_str: Option<char> = None;
    let mut in_block_comment = false;
    let mut prev = '\0';
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
                if c == q && prev != '\\' {
                    in_str = None;
                }
            }
            None => match c {
                '"' | '\'' => in_str = Some(c),
                '{' => depth += 1,
                '}' => depth -= 1,
                '#' => {
                    // line comment: skip to end of line
                    while i < bytes.len() && bytes[i] != b'\n' {
                        i += 1;
                    }
                    prev = '\0';
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
        prev = c;
        i += 1;
    }
    // A submission is incomplete while braces are open or a multiline comment
    // has not been closed yet, so the REPL keeps reading continuation lines.
    depth > 0 || in_block_comment
}

fn eval_line<W: Write>(
    interp: &mut Interp,
    decls: &mut Vec<GlobalDecl>,
    source: &str,
    writer: &mut W,
) {
    // Try a single statement first, so `let mut x = ...` works at the top
    // level of the REPL. Fall back to module items (fn/struct/enum/use) and
    // then to a bare expression.
    if let Ok(stmt) = crate::parse::parse_stmt(source) {
        let mut checker = crate::check::Checker::with_declarations(decls);
        if let Err(e) = checker.check_stmt(&stmt) {
            let _ = writeln!(writer, "{e}");
            return;
        }
        // Capture the inferred binding type here, while the checker still has
        // the statement checked and the session declarations loaded.
        let inferred_ty = checker.let_binding_type(&stmt);
        let executed = match interp.exec_stmt_globals(&stmt) {
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
                    let _ = writeln!(writer, "{d}");
                }
                false
            }
            Err(e) => {
                let _ = writeln!(writer, "{}", interp.uncaught_diag(e));
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
            if !decls.iter().any(|d| decl_name(d) == name) {
                let ty = ann.clone().or(inferred_ty);
                decls.push(GlobalDecl::Binding {
                    name: name.clone(),
                    mutable: *mutable,
                    ty,
                });
            }
        }
        // A destructuring `let` persists each name it bound, but only when the
        // statement actually executed: a failed match must not alter the
        // session (§4.7). Destructured names carry no annotation.
        if executed {
            if let Stmt::LetPattern { pattern, .. } = &stmt {
                for name in pattern.bindings() {
                    if !decls.iter().any(|d| decl_name(d) == name) {
                        decls.push(GlobalDecl::Binding {
                            name,
                            mutable: false,
                            ty: None,
                        });
                    }
                }
            }
        }
        return;
    }

    match crate::parse::parse(source) {
        Ok(module) => {
            let mut checker = crate::check::Checker::with_declarations(decls);
            if let Err(e) = checker.check_mode(&module, crate::CompileMode::Module) {
                let _ = writeln!(writer, "{e}");
                return;
            }
            // Execute first; only persist declarations the session accepted.
            for item in &module.items {
                let result = match item {
                    Item::Expr(expr, _) => match interp.eval_globals(expr) {
                        Ok(ctl) => match interp.finish_global(ctl) {
                            Ok(v) => {
                                let _ = writeln!(writer, "{}", v.display());
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(interp.uncaught_diag(e)),
                    },
                    other => interp.run_item(other),
                };
                if let Err(e) = result {
                    let _ = writeln!(writer, "{e}");
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
        }
        Err(e) => {
            // Last resort: a bare expression.
            match crate::parse::parse_expr(source) {
                Ok(expr) => match interp.eval_globals(&expr) {
                    Ok(Ctl::Val(v)) => {
                        let _ = writeln!(writer, "{}", v.display());
                    }
                    Ok(_) => {}
                    Err(e) => {
                        let _ = writeln!(writer, "{e}");
                    }
                },
                Err(_) => {
                    let _ = writeln!(writer, "{e}");
                }
            }
        }
    }
}

/// Whether two session declarations are the same one (so a re-submission does
/// not duplicate it). An `impl` is identified by its target; other
/// declarations by their introduced name.
fn same_decl(a: &GlobalDecl, b: &GlobalDecl) -> bool {
    match (a, b) {
        (GlobalDecl::Impl { .. }, GlobalDecl::Impl { .. }) => decl_name(a) == decl_name(b),
        (GlobalDecl::Impl { .. }, _) | (_, GlobalDecl::Impl { .. }) => false,
        _ => decl_name(a) == decl_name(b),
    }
}

/// The name a declaration introduces.
fn decl_name(d: &GlobalDecl) -> &str {
    match d {
        GlobalDecl::Binding { name, .. }
        | GlobalDecl::Function { name, .. }
        | GlobalDecl::Struct { name, .. }
        | GlobalDecl::Enum { name, .. }
        | GlobalDecl::Alias { name, .. } => name,
        // An `impl` block introduces no new global name; it is identified by
        // its target struct, which the surrounding `struct` declaration owns.
        GlobalDecl::Impl { target, .. } => target,
    }
}

/// The session declarations a top-level item introduces.
fn declarations_of(item: &Item) -> Vec<GlobalDecl> {
    match item {
        Item::Fn {
            name, ret, params, ..
        } => vec![GlobalDecl::Function {
            name: name.clone(),
            ret: ret.clone(),
            params: params
                .iter()
                .map(|p| (p.name.clone(), p.ty.clone()))
                .collect(),
        }],
        Item::Struct { name, fields, .. } => vec![GlobalDecl::Struct {
            name: name.clone(),
            fields: fields.clone(),
        }],
        Item::Enum { name, variants, .. } => vec![GlobalDecl::Enum {
            name: name.clone(),
            variants: variants.clone(),
        }],
        Item::Alias { name, target, .. } => vec![GlobalDecl::Alias {
            name: name.clone(),
            target: target.clone(),
        }],
        Item::Const { name, .. } => vec![GlobalDecl::Binding {
            name: name.clone(),
            mutable: false,
            ty: None,
        }],
        Item::Use { .. } | Item::Expr(..) => Vec::new(),
        Item::Impl {
            target, methods, ..
        } => vec![GlobalDecl::Impl {
            target: target.clone(),
            methods: methods
                .iter()
                .filter_map(|m| match m {
                    Item::Fn {
                        name, ret, params, ..
                    } => Some((
                        name.clone(),
                        ret.clone(),
                        params
                            .iter()
                            .map(|p| (p.name.clone(), p.ty.clone()))
                            .collect(),
                    )),
                    _ => None,
                })
                .collect(),
        }],
    }
}
