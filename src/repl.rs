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
/// # Errors
/// Returns a diagnostic only for unrecoverable I/O failures.
pub fn run() -> Result<(), Diag> {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    run_with(stdin.lock(), stdout.lock())
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
    let mut prev = '\0';
    for c in src.chars() {
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
                    // comment to end of line
                    break;
                }
                _ => {}
            },
        }
        prev = c;
    }
    depth > 0
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
        match interp.exec_stmt_globals(&stmt) {
            // A bare expression echoes its value; declarations stay silent.
            Ok(Ctl::Val(v)) if matches!(stmt, Stmt::Expr(..)) => {
                let _ = writeln!(writer, "{}", v.display());
            }
            Ok(_) => {}
            Err(e) => {
                let _ = writeln!(writer, "{e}");
            }
        }
        // A `let` introduces a binding; persist it only after it executed.
        if let Stmt::Let { name, mutable, .. } = &stmt {
            if !decls.iter().any(|d| decl_name(d) == name) {
                decls.push(GlobalDecl::Binding {
                    name: name.clone(),
                    mutable: *mutable,
                });
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
                        Ok(Ctl::Val(v)) => {
                            let _ = writeln!(writer, "{}", v.display());
                            Ok(())
                        }
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
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
                    if !decls.iter().any(|e| decl_name(e) == decl_name(&d)) {
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

/// The name a declaration introduces.
fn decl_name(d: &GlobalDecl) -> &str {
    match d {
        GlobalDecl::Binding { name, .. }
        | GlobalDecl::Function { name, .. }
        | GlobalDecl::Struct { name, .. }
        | GlobalDecl::Enum { name, .. }
        | GlobalDecl::Alias { name, .. } => name,
    }
}

/// The session declarations a top-level item introduces.
fn declarations_of(item: &Item) -> Vec<GlobalDecl> {
    match item {
        Item::Fn { name, ret, .. } => vec![GlobalDecl::Function {
            name: name.clone(),
            ret: ret.clone(),
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
        }],
        Item::Use { .. } | Item::Expr(..) => Vec::new(),
    }
}
