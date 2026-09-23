//! A minimal, honest REPL.
//!
//! Each submission is parsed and checked with the real front end, then
//! executed in a persistent interpreter session. Declarations register
//! functions, structs, and enums; expressions are evaluated and printed.
//! `:quit` (or Ctrl-D) exits; `:help` lists commands.

use std::io::{BufRead, Write};

use crate::ast::Item;
use crate::error::Diag;
use crate::run::{Ctl, Interp};

/// Run an interactive session on stdin/stdout.
///
/// # Errors
/// Returns a diagnostic only for unrecoverable I/O failures.
pub fn run() -> Result<(), Diag> {
    let mut interp = Interp::new();
    let mut globals: Vec<String> = Vec::new();
    let stdin = std::io::stdin();
    let mut input = stdin.lock();
    println!("Aura {} REPL — :help for commands", crate::VERSION);
    let mut pending = String::new();
    loop {
        let prompt = if pending.is_empty() {
            "aura> "
        } else {
            "  ... "
        };
        print!("{prompt}");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        if input.read_line(&mut line).unwrap_or(0) == 0 {
            println!();
            return Ok(());
        }
        let trimmed = line.trim();
        if pending.is_empty() {
            match trimmed {
                "" => continue,
                ":quit" | ":q" | ":exit" => return Ok(()),
                ":help" => {
                    println!(":help     show this help");
                    println!(":quit     exit");
                    println!("Otherwise type an expression or a declaration.");
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
        eval_line(&mut interp, &mut globals, &source);
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

fn eval_line(interp: &mut Interp, globals: &mut Vec<String>, source: &str) {
    match crate::parse::parse(source) {
        Ok(module) => {
            let mut checker = crate::check::Checker::with_globals(globals);
            if let Err(e) = checker.check(&module) {
                println!("{e}");
                return;
            }
            for item in &module.items {
                if let Item::Fn { name, .. } | Item::Const { name, .. } = item {
                    if !globals.contains(name) {
                        globals.push(name.clone());
                    }
                }
                let result = match item {
                    Item::Expr(expr, _) => match interp.eval_globals(expr) {
                        Ok(Ctl::Val(v)) => {
                            println!("{}", v.display());
                            Ok(())
                        }
                        Ok(_) => Ok(()),
                        Err(e) => Err(e),
                    },
                    other => interp.run_item(other),
                };
                if let Err(e) = result {
                    println!("{e}");
                    return;
                }
            }
            // A bare declaration prints nothing; a successful change is silent.
        }
        Err(e) => println!("{e}"),
    }
}
