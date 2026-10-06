//! Recursive-descent + Pratt parser.

use std::sync::Arc;

use crate::ast::*;
use crate::error::{codes, Diag, Result, Span};
use crate::lex::token::{Tok, Token};
use crate::lex::{lex, lex_at};

/// Parse a full file.
///
/// # Errors
/// Returns the first lexer, parser, or nesting-limit diagnostic.
pub fn parse(src: &str) -> Result<Module> {
    on_parse_stack(src, parse_inner)
}

/// Parse one physical source as contents already nested under
/// `initial_depth` synthetic module wrappers.
///
/// Filesystem/provider-backed module splitting must consume the same parser
/// recursion budget as equivalent in-source `module` blocks. This crate-local
/// entry point lets the provider-neutral graph builder preserve that invariant
/// without putting source identity into the AST.
pub(crate) fn parse_with_initial_depth(src: &str, initial_depth: usize) -> Result<Module> {
    on_parse_stack(src, move |src| {
        parse_inner_with_initial_depth(src, initial_depth)
    })
}

/// Parse one expression (used by the REPL and tests).
///
/// # Errors
/// Returns the first lexer, parser, or nesting-limit diagnostic.
pub fn parse_expr(src: &str) -> Result<Expr> {
    on_parse_stack(src, parse_expr_inner)
}

/// Parse exactly one statement. Unlike module items, a bare `let mut` is a
/// valid statement here, which is what the REPL needs.
///
/// # Errors
/// Returns the first lexer, parser, or nesting-limit diagnostic.
pub fn parse_stmt(src: &str) -> Result<Stmt> {
    on_parse_stack(src, parse_stmt_inner)
}

/// Run a parsing function on the execution substrate, so deep (but bounded)
/// input does not overflow a small caller stack.
///
/// This delegates to [`crate::on_execution_stack`], which uses a dedicated
/// 64 MiB stack on native and runs inline on WebAssembly. The parser's
/// recursion budget ([`parse_recursion_budget`]) is calibrated per substrate
/// so that over-deep grouping is reported as `E1015` rather than trapping.
fn on_parse_stack<T, F>(src: &str, f: F) -> Result<T>
where
    T: Send + 'static,
    F: FnOnce(&str) -> Result<T> + Send + 'static,
{
    let src = src.to_string();
    crate::on_execution_stack(move || f(&src))
}

fn parse_inner(src: &str) -> Result<Module> {
    parse_inner_with_initial_depth(src, 0)
}

fn parse_inner_with_initial_depth(src: &str, initial_depth: usize) -> Result<Module> {
    if initial_depth > parse_recursion_budget() {
        return Err(Diag::new(
            codes::NESTING,
            "expression nests too deeply",
            Span::default(),
        ));
    }
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: initial_depth,
        expr_nodes: 0,
        atom_depth: 0,
        type_depth: 0,
        // A physical wrapper (a loaded child source) is equivalent to an
        // in-source `module` block, so it consumes the same semantic nesting
        // budget: `initial_depth` is that wrapper depth.
        module_depth: initial_depth,
    };
    let module = p.module()?;
    // Reject trees deeper than the language limit here, before any later
    // phase clones or walks them on an arbitrary stack. The check is
    // iterative, so it cannot itself overflow.
    enforce_depth(&module)?;
    Ok(module)
}

fn parse_expr_inner(src: &str) -> Result<Expr> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        expr_nodes: 0,
        atom_depth: 0,
        type_depth: 0,
        module_depth: 0,
    };
    p.skip_newlines();
    let e = p.expr()?;
    p.skip_newlines();
    p.expect_eof()?;
    // Enforce the language nesting limit for this entry point too, so a
    // statement/expression parsed for the REPL cannot exceed the same bound
    // as a module.
    check_expr_depth(&e, 1)?;
    Ok(e)
}

/// Parse one expression whose text is a verbatim substring of a larger source
/// beginning at byte offset `base` (used for f-string interpolations). Token
/// spans stay absolute, so diagnostics name the real file location.
fn parse_expr_at(src: &str, base: usize) -> Result<Expr> {
    let toks = lex_at(src, base)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        expr_nodes: 0,
        atom_depth: 0,
        type_depth: 0,
        module_depth: 0,
    };
    p.skip_newlines();
    let e = p.expr()?;
    p.skip_newlines();
    p.expect_eof()?;
    check_expr_depth(&e, 1)?;
    Ok(e)
}

fn parse_stmt_inner(src: &str) -> Result<Stmt> {
    let toks = lex(src)?;
    let mut p = Parser {
        toks,
        pos: 0,
        depth: 0,
        expr_nodes: 0,
        atom_depth: 0,
        type_depth: 0,
        module_depth: 0,
    };
    p.skip_newlines();
    let s = p.stmt()?;
    p.skip_newlines();
    p.expect_eof()?;
    // Enforce the language nesting limit for this entry point too.
    check_stmt_depth(std::slice::from_ref(&s), 1)?;
    Ok(s)
}

/// Language limit on AST nesting. This is the single bound that keeps every
/// later phase (clone, check, evaluate, drop) safe on a small stack.
pub const MAX_AST_DEPTH: usize = 256;

/// The parser's recursion backstop: an implementation-safety bound on the
/// depth of grouping tokens (parentheses), which add no AST depth and so are
/// not governed by [`MAX_AST_DEPTH`]. It is measured in recursive-descent
/// frames, not AST nodes.
///
/// This is **not** a language limit; it protects the host stack. It is
/// substrate-calibrated:
///
/// * Native runs the parser on a dedicated 64 MiB stack, so the backstop is
///   far above anything the semantic limit can require.
/// * WebAssembly runs the parser inline on the engine's call stack. The
///   runtime crate reserves a larger linear stack for it
///   (`playground/runtime/build.rs`), but the backstop is still calibrated
///   against that stack's frame budget: the most frame-expensive path (nested
///   call arguments, `f(f(f(…)))`) exhausted the default 1 MiB stack at
///   roughly 907 nesting levels. The budget must therefore stay *below* the
///   physical ceiling while still comfortably above what any AST-valid program
///   needs. A program at the semantic AST limit (256 nodes) costs on the order
///   of 256–512 frames, so 768 is chosen: it accepts every AST-valid program
///   with room for grouping, and fires `E1015` well before the engine stack is
///   exhausted. (Native, on a dedicated 64 MiB stack, keeps a larger budget.)
///   This value is re-calibrated whenever parser frame sizes change; the
///   mutation-capability and f-string work enlarged those frames, so 1024 no
///   longer sat below the physical ceiling.
///
/// Exceeding it is `E1015` on every substrate; it never redefines the
/// semantic AST limit.
#[must_use]
pub const fn parse_recursion_budget() -> usize {
    #[cfg(target_arch = "wasm32")]
    {
        768
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        2048
    }
}

/// Iteratively verify that no expression/statement nests beyond
/// [`MAX_AST_DEPTH`]. Uses an explicit heap stack and never recurses.
fn enforce_depth(module: &Module) -> Result<()> {
    // Worklist of (node depth). Items are walked with a small recursive
    // helper bounded by the module structure; expressions and statements use
    // the explicit stack below.
    for item in &module.items {
        match item {
            Item::Fn { body, .. } => check_stmt_depth(body, 1)?,
            Item::Const { value, .. } => check_expr_depth(value, 1)?,
            Item::Expr(e, _) => check_expr_depth(e, 1)?,
            _ => {}
        }
    }
    Ok(())
}

fn check_expr_depth(root: &Expr, start: usize) -> Result<()> {
    let mut stack: Vec<(&Expr, usize)> = vec![(root, start)];
    while let Some((e, depth)) = stack.pop() {
        if depth > MAX_AST_DEPTH {
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                e.span(),
            ));
        }
        let d = depth + 1;
        match e {
            Expr::Lit(..) | Expr::Name(..) | Expr::FStr(..) => {}
            Expr::Unary(_, o, _) => stack.push((o, d)),
            Expr::Binary(_, l, r, _) => {
                stack.push((l, d));
                stack.push((r, d));
            }
            Expr::Call(f, args, _, _) => {
                stack.push((f, d));
                for a in args.iter() {
                    stack.push((&a.value, d));
                }
            }
            Expr::Method(r, _, args, _, _) => {
                stack.push((r, d));
                for a in args.iter() {
                    stack.push((&a.value, d));
                }
            }
            Expr::Field(r, _, _) => stack.push((r, d)),
            Expr::Index(b, i, _) => {
                stack.push((b, d));
                stack.push((i, d));
            }
            Expr::List(items, _) | Expr::Tuple(items, _) => {
                for i in items.iter() {
                    stack.push((i, d));
                }
            }
            Expr::ListComp {
                value,
                iterable,
                filter,
                ..
            } => {
                stack.push((value, d));
                stack.push((iterable, d));
                if let Some(f) = filter {
                    stack.push((f, d));
                }
            }
            Expr::MapComp {
                key,
                value,
                iterable,
                filter,
                ..
            } => {
                stack.push((key, d));
                stack.push((value, d));
                stack.push((iterable, d));
                if let Some(f) = filter {
                    stack.push((f, d));
                }
            }
            Expr::Map(entries, _) => {
                for (k, v) in entries.iter() {
                    stack.push((k, d));
                    stack.push((v, d));
                }
            }
            Expr::Construct(_, args, _, _) => {
                for a in args.iter() {
                    stack.push((&a.value, d));
                }
            }
            Expr::Lambda(_, body, _) => stack.push((body, d)),
            Expr::Pipe(l, r, _) => {
                stack.push((l, d));
                stack.push((r, d));
            }
            Expr::Range(l, r, _) => {
                stack.push((l, d));
                stack.push((r, d));
            }
            Expr::If(c, then, els, _) => {
                stack.push((c, d));
                check_stmt_depth(then, d)?;
                if let Some(e) = els {
                    stack.push((e, d));
                }
            }
            Expr::Match(subject, arms, _) => {
                stack.push((subject, d));
                for arm in arms.iter() {
                    if let Some(g) = &arm.guard {
                        stack.push((g, d));
                    }
                    check_stmt_depth(&arm.body, d)?;
                }
            }
            Expr::Block(stmts, _) => check_stmt_depth(stmts, d)?,
        }
        // F-string interpolations are expressions too.
        if let Expr::FStr(parts, _) = e {
            for p in parts.iter() {
                if let FPart::Expr(inner, _) = p {
                    stack.push((inner, d));
                }
            }
        }
    }
    Ok(())
}

fn check_stmt_depth(stmts: &[Stmt], start: usize) -> Result<()> {
    let mut stack: Vec<&Stmt> = stmts.iter().collect();
    let mut guard = 0usize;
    while let Some(s) = stack.pop() {
        guard += 1;
        if guard > 10_000_000 {
            return Err(Diag::new(
                codes::NESTING,
                "program nests too deeply",
                s.span(),
            ));
        }
        match s {
            Stmt::Let { value, .. } => check_expr_depth(value, start)?,
            Stmt::LetPattern { value, .. } => check_expr_depth(value, start)?,
            Stmt::Assign { target, value, .. } => {
                check_expr_depth(target, start)?;
                check_expr_depth(value, start)?;
            }
            Stmt::Expr(e, _) => check_expr_depth(e, start)?,
            Stmt::Return(Some(e), _) | Stmt::Throw(e, _) => check_expr_depth(e, start)?,
            Stmt::Return(None, _) | Stmt::Break(_) | Stmt::Continue(_) => {}
            Stmt::While(c, body, _) => {
                check_expr_depth(c, start)?;
                stack.extend(body.iter());
            }
            Stmt::Loop(body, _) => stack.extend(body.iter()),
            Stmt::For(_, iter, body, _) => {
                check_expr_depth(iter, start)?;
                stack.extend(body.iter());
            }
            Stmt::Try {
                body,
                catch_body,
                finally,
                ..
            } => {
                stack.extend(body.iter());
                stack.extend(catch_body.iter());
                if let Some(f) = finally {
                    stack.extend(f.iter());
                }
            }
        }
    }
    Ok(())
}

/// Raw recursion budget for the recursive-descent parser.
///
/// This is a **host-safety** backstop, not the language's semantic nesting
/// limit. The semantic limit is [`MAX_AST_DEPTH`] (256), measured on the
/// *AST* and enforced iteratively by [`enforce_depth`]. Grouping tokens such
/// as parentheses recurse in the parser without adding AST depth, so the
/// parser needs its own frame budget; it is set well above what the semantic
/// limit can consume (an AST level costs a small constant number of frames).
/// The budget is substrate-calibrated by [`parse_recursion_budget`], and
/// over-limit input is reported as `E1015`, the same code as the semantic
/// limit, so users see one consistent "nesting" diagnostic.
struct Parser {
    toks: Vec<Token>,
    pos: usize,
    depth: usize,
    /// Number of infix/postfix wraps applied to the expression currently
    /// being built. Bounds the depth of a flat chain without recursing.
    expr_nodes: usize,
    /// Current nesting depth of `atom()` wrappers (groups, lists, maps). This
    /// is at most the final AST depth, so checking it against
    /// [`MAX_AST_DEPTH`] *during* parsing rejects over-deep input with the same
    /// `E1015` the post-parse walk would produce, but before the recursive
    /// descent can exhaust the host stack on a substrate whose physical
    /// ceiling is below the recursion budget (`LANGUAGE_SPEC.md` §31.2/§31.5).
    atom_depth: usize,
    /// Current structural nesting depth of a type annotation (`Box<…>`, `[T]`,
    /// `{K: V}`). Type nesting counts toward the same [`MAX_AST_DEPTH`] budget
    /// as every other node kind (ADR-0004), so "valid under the semantic limit"
    /// holds identically on native and WASM. A flat union (`A | B | …`) is a
    /// loop, not nesting, and is not counted per member.
    type_depth: usize,
    /// Current structural nesting depth of in-source `module` blocks. Module
    /// nesting is structural, so it counts toward the same [`MAX_AST_DEPTH`]
    /// budget as expressions, statements, and types: without this a deep
    /// module chain was bounded only by the substrate-calibrated parser
    /// backstop (native accepted 2047, WASM 767), the same native/WASM
    /// acceptance divergence ADR-0004 removed for types.
    module_depth: usize,
}

impl Parser {
    fn at(&self) -> &Tok {
        &self.toks[self.pos.min(self.toks.len() - 1)].tok
    }

    fn span(&self) -> Span {
        self.toks[self.pos.min(self.toks.len() - 1)].span
    }

    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos.min(self.toks.len() - 1)].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.at() == t {
            self.bump();
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.at(), Tok::Newline) {
            self.bump();
        }
    }

    /// Verify and consume the separator between items in an item list (a
    /// module body, an `impl` body, or the file root): a newline or `;`, or the
    /// zero-width boundary before `}`/EOF (CONF-PARSE-8). `fn a() {} fn b() {}`
    /// on one line is rejected in every item-list body.
    fn item_separator(&mut self) -> Result<()> {
        match self.at() {
            Tok::Eof | Tok::RBrace | Tok::Newline | Tok::Semi => {
                while matches!(self.at(), Tok::Newline | Tok::Semi) {
                    self.bump();
                }
                Ok(())
            }
            other => Err(Diag::new(
                codes::EXPECTED,
                format!(
                    "expected a newline or `;` between items, found {}",
                    other.describe()
                ),
                self.span(),
            )),
        }
    }

    /// Verify and consume the statement separator (§3.7, CONF-PARSE-8).
    ///
    /// A statement is terminated by a newline or `;`, or by the zero-width
    /// boundary directly before `}` or EOF. Adjacent statements without a real
    /// separator (`1 2`, `let x = 1 let y = 2`) are rejected rather than
    /// accepted as a parser accident.
    /// The separator token is left in place for the enclosing block/module
    /// loop to consume, so an item parser and its loop never disagree about
    /// which side owns it.
    fn end_stmt(&mut self) -> Result<()> {
        match self.at() {
            Tok::RBrace | Tok::Eof | Tok::Semi | Tok::Newline => Ok(()),
            other => Err(Diag::new(
                codes::EXPECTED,
                format!(
                    "expected a newline or `;` between statements, found {}",
                    other.describe()
                ),
                self.span(),
            )),
        }
    }

    fn expect(&mut self, t: &Tok) -> Result<Span> {
        if self.at() == t {
            Ok(self.bump().span)
        } else {
            Err(self.expected(&t.to_string()))
        }
    }

    fn expect_eof(&mut self) -> Result<()> {
        if matches!(self.at(), Tok::Eof) {
            Ok(())
        } else {
            Err(self.expected("end of input"))
        }
    }

    fn expected(&self, what: &str) -> Diag {
        Diag::new(
            codes::EXPECTED,
            format!("expected {what}, found {}", self.at().describe()),
            self.span(),
        )
    }

    fn enter(&mut self) -> Result<()> {
        self.depth += 1;
        if self.depth > parse_recursion_budget() {
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                self.span(),
            ));
        }
        Ok(())
    }

    fn leave(&mut self) {
        self.depth -= 1;
    }

    // ------------------------------------------------------------- module

    fn module(&mut self) -> Result<Module> {
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.at(), Tok::Eof) {
                break;
            }
            items.push(self.item()?);
            self.item_separator()?;
        }
        Ok(Module { items })
    }

    fn item(&mut self) -> Result<Item> {
        let public = self.eat(&Tok::Pub);
        // `impl`, `trait`, and `module` are NOT reserved words: they stay
        // ordinary identifiers everywhere. A behavior block, trait, or module
        // is recognized only in this item position (§17.6, §17.7, §28, §3.3).
        if self.at_impl_block() {
            if public {
                return Err(Diag::new(
                    codes::EXPECTED,
                    "`pub` does not apply to an `impl` block; visibility names an item, and an `impl` block has no name of its own",
                    self.span(),
                ));
            }
            return self.impl_item();
        }
        if self.at_trait_block() {
            return self.trait_item(public);
        }
        if self.at_const_decl() {
            return self.const_decl_item(public);
        }
        if self.at_module_block() {
            return self.module_item(public);
        }
        match self.at().clone() {
            Tok::Fn => self.fn_item(public),
            Tok::Struct => self.struct_item(public),
            Tok::Enum => self.enum_item(public),
            Tok::Type => self.alias_item(public),
            Tok::Use => self.use_item(public),
            Tok::Let => self.const_item(public),
            _ => {
                if public {
                    let found = self.at().describe();
                    return Err(Diag::new(
                        codes::EXPECTED,
                        format!(
                            "`pub` must precede a declaration (`fn`, `struct`, `enum`, `type`, `const`, `let`, `trait`, or `module`), found {found}"
                        ),
                        self.span(),
                    ));
                }
                let e = self.expr()?;
                let span = span_of(&e);
                self.end_stmt()?;
                Ok(Item::Expr(e, span))
            }
        }
    }

    /// Whether the tokens here begin a module declaration: `module` `Name` `{`.
    /// Contextual like `impl`: `module` is an ordinary identifier everywhere
    /// else, so `let module = 1` remains valid.
    fn at_module_block(&self) -> bool {
        if !matches!(self.at(), Tok::Ident(n) if n == "module") {
            return false;
        }
        let Some(next) = self.toks.get(self.pos + 1) else {
            return false;
        };
        let Some(after) = self.toks.get(self.pos + 2) else {
            return false;
        };
        matches!(&next.tok, Tok::Ident(_)) && matches!(after.tok, Tok::LBrace)
    }

    /// `module Name { items }` — an in-source module (`LANGUAGE_SPEC.md` §28).
    /// A module is a real visibility boundary; its items are private unless
    /// marked `pub`. Modules nest.
    fn module_item(&mut self, public: bool) -> Result<Item> {
        // Nested modules recurse through `item()`, so — like a nested type or
        // expression — the descent must consume the parser's host-safety
        // backstop. Without it a deeply nested module aborted the process with
        // a native stack overflow instead of the stable `E1015`
        // (`LANGUAGE_SPEC.md` §31.2, §31.5).
        self.enter()?;
        // Module nesting is structural and counts toward the semantic
        // `MAX_AST_DEPTH` on every substrate (ADR-0004), so acceptance does not
        // depend on the substrate-calibrated backstop.
        self.module_depth += 1;
        if self.module_depth > MAX_AST_DEPTH {
            self.module_depth -= 1;
            self.leave();
            return Err(Diag::new(
                codes::NESTING,
                "module nests too deeply",
                self.span(),
            ));
        }
        let r = self.module_item_inner(public);
        self.module_depth -= 1;
        self.leave();
        r
    }

    fn module_item_inner(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("module name")?;
        self.expect(&Tok::LBrace)?;
        let mut items = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if matches!(self.at(), Tok::Eof) {
                return Err(self.expected("`}` closing the `module` block"));
            }
            items.push(self.item()?);
            self.item_separator()?;
        }
        Ok(Item::Module {
            name,
            items,
            public,
            span,
        })
    }

    /// Whether the tokens here begin a behavior block or a trait
    /// implementation: `impl` `TypeName` `{` / `impl` `TypeName` `for` `Type`.
    /// An `impl` identifier followed by a capitalized name and either `{` or
    /// `for` is unmistakable; any other use of `impl` is left untouched.
    fn at_impl_block(&self) -> bool {
        if !matches!(self.at(), Tok::Ident(n) if n == "impl") {
            return false;
        }
        let Some(next) = self.toks.get(self.pos + 1) else {
            return false;
        };
        // `impl<T> ...` — a generic parameter list first.
        let mut i = self.pos + 1;
        if matches!(next.tok, Tok::Lt) {
            let Some(after) = self.balanced_angles_from(i) else {
                return false;
            };
            i = after;
        }
        let Some(head) = self.toks.get(i) else {
            return false;
        };
        // The first segment is a type name (`impl Shape {`), or the leading
        // segment of a module path (`impl shapes::Shape {`), which is lowercase
        // by convention and followed by `::`.
        let first_ok = match &head.tok {
            Tok::Ident(name) if name.chars().next().is_some_and(char::is_uppercase) => true,
            Tok::Ident(_) => {
                matches!(self.toks.get(i + 1).map(|t| &t.tok), Some(Tok::ColonColon))
            }
            _ => false,
        };
        if !first_ok {
            return false;
        }
        // Walk the rest of the `::`-separated path starting *after* the first
        // segment, then require `{`, `for`, `<`, or a bare identifier (a
        // malformed header such as `impl A B {`, treated as a behavior block so
        // the error is a parse diagnostic rather than falling through to
        // expression parsing).
        i += 1;
        loop {
            match self.toks.get(i).map(|t| &t.tok) {
                Some(Tok::ColonColon) => {
                    if !matches!(self.toks.get(i + 1).map(|t| &t.tok), Some(Tok::Ident(_))) {
                        return false;
                    }
                    i += 2;
                }
                Some(Tok::Ident(_)) => {
                    // `impl A B {` malformed header.
                    return matches!(self.toks.get(i + 1).map(|t| &t.tok), Some(Tok::LBrace));
                }
                Some(Tok::Lt) => {
                    // `impl shapes::Shape<int> {` — skip the argument list.
                    match self.balanced_angles_from(i) {
                        Some(after) => i = after,
                        None => return false,
                    }
                }
                Some(Tok::LBrace | Tok::For) => return true,
                _ => return false,
            }
        }
    }

    /// Whether the tokens here begin a trait declaration: `trait` `Name` `{`.
    /// Contextual like `impl`: only an identifier `trait` followed by a
    /// capitalized name and `{` is a declaration.
    fn at_trait_block(&self) -> bool {
        if !matches!(self.at(), Tok::Ident(n) if n == "trait") {
            return false;
        }
        let Some(name) = self.toks.get(self.pos + 1) else {
            return false;
        };
        if !matches!(name.tok, Tok::Ident(ref n) if n.chars().next().is_some_and(char::is_uppercase))
        {
            return false;
        }
        // `trait Name {` or `trait Name<T, U> {`.
        let mut i = self.pos + 2;
        if matches!(self.toks.get(i).map(|t| &t.tok), Some(Tok::Lt)) {
            let Some(after) = self.balanced_angles_from(i) else {
                return false;
            };
            i = after;
        }
        matches!(self.toks.get(i).map(|t| &t.tok), Some(Tok::LBrace))
    }

    /// Whether the tokens here begin a `const` declaration:
    /// `const` `IDENT` (`=` | `:`). `const` is contextual, like `impl` and
    /// `trait`: an identifier followed by a name and either `=` or an
    /// annotation is unmistakably a declaration; any other use of `const` is
    /// an ordinary name. A lowercase name is still recognized, so it can be
    /// rejected with a clear "must be uppercase" diagnostic rather than an
    /// incidental parse error.
    fn at_const_decl(&self) -> bool {
        if !matches!(self.at(), Tok::Ident(n) if n == "const") {
            return false;
        }
        let Some(next) = self.toks.get(self.pos + 1) else {
            return false;
        };
        let Some(after) = self.toks.get(self.pos + 2) else {
            return false;
        };
        matches!(&next.tok, Tok::Ident(_))
            && matches!(
                after.tok,
                Tok::Assign | Tok::Colon | Tok::Newline | Tok::Semi | Tok::Eof
            )
    }

    /// `const NAME [: T] = expr` — the canonical module-level constant
    /// declaration (`LANGUAGE_SPEC.md` §4.2, §26). The name MUST be uppercase
    /// so a constant declaration reads differently from an ordinary binding.
    fn const_decl_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("constant name")?;
        if !name.chars().next().is_some_and(char::is_uppercase) {
            return Err(Diag::new(
                codes::EXPECTED,
                "a `const` name must begin with an uppercase letter",
                span,
            ));
        }
        let ann = if self.eat(&Tok::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        if !self.eat(&Tok::Assign) {
            return Err(Diag::new(
                codes::LET_NO_INIT,
                format!("`const {name}` needs an initializer: `const {name} = ...`"),
                span,
            ));
        }
        let value = self.expr()?;
        self.end_stmt()?;
        Ok(Item::Const {
            name,
            ann,
            value,
            public,
            span,
        })
    }

    // ------------------------------------------------------------- generics

    /// Whether a `<` at the current position begins a generic type-parameter
    /// *declaration* list (`fn f<T>(...)`, `struct S<T> {`) rather than a
    /// comparison. It is a declaration list here only because the caller is in
    /// a declaration position: after `fn Name`, `struct Name`, `trait Name`,
    /// `type Name`, or `impl`, and before `(` / `{` / `for`.
    ///
    /// The decision is made by scanning the balanced `<...>` and checking that
    /// the token after it is the declaration's own delimiter. Because the
    /// caller controls the context, an angle bracket in this position is
    /// unambiguously generic.
    fn at_type_params(&self) -> bool {
        if !matches!(self.at(), Tok::Lt) {
            return false;
        }
        self.balanced_angles_from(self.pos).is_some()
    }

    /// Scan a balanced `<...>` starting at `start` (which must be `<`),
    /// returning the index just past the matching `>` if the brackets nest to
    /// exactly one level and per-level angle depth stays positive. A `<<` or
    /// `>>` token is treated as one level of opening/closing respectively, so
    /// `A<B<C>>` lexed as `>>` still balances. Returns `None` on imbalance or
    /// newline/EOF before the close.
    fn balanced_angles_from(&self, start: usize) -> Option<usize> {
        let mut depth = 0usize;
        let mut i = start;
        while let Some(t) = self.toks.get(i).map(|t| &t.tok) {
            match t {
                Tok::Lt => depth += 1,
                Tok::Shl => depth += 2,
                Tok::Gt => {
                    depth = depth.checked_sub(1)?;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
                Tok::Shr => {
                    depth = depth.checked_sub(2)?;
                    if depth == 0 {
                        return Some(i + 1);
                    }
                }
                Tok::Newline | Tok::Eof | Tok::Semi => return None,
                _ => {}
            }
            i += 1;
        }
        None
    }

    /// Consume one closing `>`. When the lexer produced `>>` or `>>=` (a shift
    /// token) as the two closing angles of nested generics, split it: consume
    /// one `>` now and leave the remainder as a single `>` token in place. This
    /// is the classic generic-versus-shift disambiguation, resolved in the
    /// parser's generic positions only.
    fn expect_close_angle(&mut self) -> Result<Span> {
        match self.at().clone() {
            Tok::Gt => Ok(self.bump().span),
            Tok::Shr => {
                let span = self.span();
                if let Some(t) = self.toks.get_mut(self.pos) {
                    t.tok = Tok::Gt;
                }
                Ok(span)
            }
            other => Err(Diag::new(
                codes::EXPECTED,
                format!("expected `>`, found {}", other.describe()),
                self.span(),
            )),
        }
    }

    /// Parse a generic type-parameter declaration list `<T, U: Trait>` in a
    /// declaration position, when one is present. Returns an empty list when
    /// no `<` begins one.
    fn opt_type_params(&mut self) -> Result<Vec<TypeParam>> {
        if !self.at_type_params() {
            return Ok(Vec::new());
        }
        self.bump(); // `<`
        let mut params = Vec::new();
        loop {
            self.skip_newlines();
            let span = self.span();
            let name = self.ident("type parameter name")?;
            let mut bounds = Vec::new();
            if self.eat(&Tok::Colon) {
                loop {
                    let b = self.ident("trait name")?;
                    // A bound may apply type arguments (`U: Container<T>`);
                    // they are consumed here and the bound is recorded on the
                    // trait's head, which is what bound satisfaction checks.
                    let _ = self.decl_type_args()?;
                    bounds.push(b);
                    if !self.eat(&Tok::Plus) {
                        break;
                    }
                }
            }
            params.push(TypeParam { name, bounds, span });
            self.skip_newlines();
            if self.eat(&Tok::Comma) {
                self.skip_newlines();
                if matches!(self.at(), Tok::Gt | Tok::Shr) {
                    self.expect_close_angle()?;
                    break;
                }
                continue;
            }
            self.expect_close_angle()?;
            break;
        }
        Ok(params)
    }

    /// Parse a generic type-argument list `<T, [U]>` in a type or expression
    /// position, when one is present. Returns an empty list when no `<`
    /// begins one. `AtTypeArgs` disambiguates from comparison by the trailing
    /// `(`/`{`/`,`/`>` that a type-only list demands.
    fn ty_args(&mut self) -> Result<Vec<TypeExpr>> {
        if !self.at_type_args() {
            return Ok(Vec::new());
        }
        self.bump(); // `<`
        let mut args = Vec::new();
        loop {
            self.skip_newlines();
            args.push(self.ty()?);
            self.skip_newlines();
            if self.eat(&Tok::Comma) {
                self.skip_newlines();
                if matches!(self.at(), Tok::Gt | Tok::Shr) {
                    self.expect_close_angle()?;
                    break;
                }
                continue;
            }
            self.expect_close_angle()?;
            break;
        }
        Ok(args)
    }

    /// Whether a `<` at the current position begins a generic type *argument*
    /// list. In expression position this is ambiguous with `a < b`, so it is
    /// recognized only under the tightest rule that keeps existing programs
    /// unchanged:
    ///
    /// * the `<` must be **adjacent** to the preceding identifier (no space),
    ///   so `a < b > (c)` is still two comparisons; and
    /// * the balanced `<...>` must be immediately followed by `(`, `{`, or
    ///   `::`, the only suffixes a parameterised call/type may take.
    ///
    /// `identity<int>(x)` and `Box<int> { ... }` qualify; `a < b` does not.
    fn at_type_args(&self) -> bool {
        if !matches!(self.at(), Tok::Lt) {
            return false;
        }
        // Adjacency: the token before `<` must end exactly where `<` begins.
        let adjacent = self
            .pos
            .checked_sub(1)
            .and_then(|p| self.toks.get(p))
            .is_some_and(|prev| prev.span.end == self.span().start);
        if !adjacent {
            return false;
        }
        let Some(after) = self.balanced_angles_from(self.pos) else {
            return false;
        };
        matches!(
            self.toks.get(after).map(|t| &t.tok),
            Some(Tok::LParen | Tok::LBrace | Tok::ColonColon)
        )
    }

    fn fn_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("function name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::LParen)?;
        let params = self.params(false)?;
        let (ret, ret_span) = self.opt_return_ty()?;
        let body = self.block()?;
        Ok(Item::Fn {
            name,
            type_params,
            params,
            ret,
            ret_span,
            body,
            public,
            span,
        })
    }

    /// `impl Struct { fn method(self, ...) { ... } ... }` — a behavior block
    /// attached to an already-declared nominal struct — or `impl Trait for
    /// Struct { ... }` — a trait implementation (`LANGUAGE_SPEC.md` §17.6,
    /// §17.7). Only `fn` declarations may appear; each must declare the
    /// explicit receiver `self` as its first parameter.
    fn impl_item(&mut self) -> Result<Item> {
        let span = self.span();
        self.bump();
        let type_params = self.opt_type_params()?;
        // The head is either `Name` or `Name<Args>`.
        let (first, first_args) = self.type_head()?;
        // `impl Trait for Struct` — `for` is a reserved token, used here as the
        // trait-implementation separator.
        let (trait_name, trait_args, target, target_args) = if self.at() == &Tok::For {
            self.bump();
            let (target, target_args) = self.type_head()?;
            (Some(first), first_args, target, target_args)
        } else {
            (None, Vec::new(), first, first_args)
        };
        self.expect(&Tok::LBrace)?;
        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if matches!(self.at(), Tok::Eof) {
                return Err(self.expected("`}` closing the `impl` block"));
            }
            let public = self.eat(&Tok::Pub);
            if !matches!(self.at(), Tok::Fn) {
                let found = self.at().describe();
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("expected a `fn` method declaration or `}}` in `impl`, found {found}"),
                    self.span(),
                ));
            }
            methods.push(self.method_item(public)?);
            self.item_separator()?;
        }
        Ok(Item::Impl {
            target,
            target_args,
            trait_name,
            trait_args,
            type_params,
            methods,
            owner: Vec::new(),
            span,
        })
    }

    /// Parse a `::`-qualified type head with optional type arguments:
    /// `Name`, `Name<T, U>`, `mod::Name<T>`. Returns the joined name and the
    /// type arguments (empty when none are written).
    ///
    /// In a declaration head (`impl Trait<T> for Struct<T>`) a `<` is
    /// unambiguous, so it is parsed unconditionally rather than through the
    /// expression-oriented [`Parser::at_type_args`].
    fn type_head(&mut self) -> Result<(String, Vec<TypeExpr>)> {
        let (segments, _) = self.path_segments()?;
        let name = segments.join("::");
        let args = self.decl_type_args()?;
        Ok((name, args))
    }

    /// Parse a type-argument list in a declaration head position, where a `<`
    /// after the head name is always generic. Returns an empty list when the
    /// next token is not `<`.
    fn decl_type_args(&mut self) -> Result<Vec<TypeExpr>> {
        if !matches!(self.at(), Tok::Lt) {
            return Ok(Vec::new());
        }
        self.bump();
        let mut args = Vec::new();
        loop {
            self.skip_newlines();
            args.push(self.ty()?);
            self.skip_newlines();
            if self.eat(&Tok::Comma) {
                self.skip_newlines();
                if matches!(self.at(), Tok::Gt | Tok::Shr) {
                    self.expect_close_angle()?;
                    break;
                }
                continue;
            }
            self.expect_close_angle()?;
            break;
        }
        Ok(args)
    }

    /// `trait Name { fn method(self, ...) -> T ... }` — a behavioral contract
    /// (`LANGUAGE_SPEC.md` §17.7). Declarations only: every member is a `fn`
    /// with a `self` receiver and no body.
    fn trait_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("trait name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::LBrace)?;
        let mut methods = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            if matches!(self.at(), Tok::Eof) {
                return Err(self.expected("`}` closing the `trait` block"));
            }
            let public = self.eat(&Tok::Pub);
            if !matches!(self.at(), Tok::Fn) {
                let found = self.at().describe();
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!(
                        "expected a `fn` signature or `}}` in `trait`, found {found}; traits declare signatures only (no bodies, fields, or associated items)"
                    ),
                    self.span(),
                ));
            }
            methods.push(self.trait_method_decl(public)?);
            self.item_separator()?;
        }
        Ok(Item::Trait {
            name,
            type_params,
            methods,
            public,
            owner: Vec::new(),
            span,
        })
    }

    /// `fn name(self, ...) -> T` inside a trait: a declaration with no body.
    fn trait_method_decl(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("trait method name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::LParen)?;
        let params = self.params(true)?;
        if params.is_empty() {
            return Err(Diag::new(
                codes::EXPECTED,
                format!(
                    "trait method `{name}` must declare the receiver `self` as its first parameter"
                ),
                span,
            ));
        }
        let (ret, ret_span) = self.opt_return_ty()?;
        // A trait method is a declaration: no body. A following `{` would be a
        // default body, which V1 does not support.
        if matches!(self.at(), Tok::LBrace) {
            return Err(Diag::new(
                codes::EXPECTED,
                format!(
                    "trait method `{name}` must not have a body; traits declare signatures only"
                ),
                span,
            ));
        }
        self.end_stmt()?;
        Ok(Item::Fn {
            name,
            type_params,
            params,
            ret,
            ret_span,
            body: Arc::from([]),
            public,
            span,
        })
    }

    /// `fn name(self, ...) { ... }` inside an `impl` block. Identical to
    /// [`Parser::fn_item`] except that the first parameter MUST be `self`.
    fn method_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("method name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::LParen)?;
        let params = self.params(true)?;
        if params.is_empty() {
            return Err(Diag::new(
                codes::EXPECTED,
                format!("method `{name}` must declare the receiver `self` as its first parameter"),
                span,
            ));
        }
        let (ret, ret_span) = self.opt_return_ty()?;
        let body = self.block()?;
        Ok(Item::Fn {
            name,
            type_params,
            params,
            ret,
            ret_span,
            body,
            public,
            span,
        })
    }

    /// Parse an optional `-> T` return annotation, returning the type and the
    /// span of its source text. The span is recorded so an unknown return type
    /// is diagnosed at the annotation rather than the enclosing item
    /// (`LANGUAGE_SPEC.md` §17.6, §17.7).
    fn opt_return_ty(&mut self) -> Result<(Option<TypeExpr>, Option<Span>)> {
        if !self.eat(&Tok::Arrow) {
            return Ok((None, None));
        }
        let start = self.span().start;
        let ty = self.ty()?;
        // The last consumed token ends the annotation; `pos` now points past
        // it, so its span is at `pos - 1`.
        let end = self.toks[self.pos.saturating_sub(1)].span.end;
        Ok((Some(ty), Some(Span::new(start, end))))
    }

    /// Parse a parameter list. When `receiver` is true, the first parameter
    /// MUST be the identifier `self`, which becomes the method's receiver
    /// binding. `self` is not a reserved word elsewhere: this is the only
    /// position with receiver semantics (§17.6, §3.3).
    fn params(&mut self, receiver: bool) -> Result<Vec<Param>> {
        let mut out = Vec::new();
        let mut first = true;
        self.skip_newlines();
        if self.eat(&Tok::RParen) {
            return Ok(out);
        }
        loop {
            let span = self.span();
            // `mut` may prefix a receiver (`fn m(mut self)`) or an ordinary
            // parameter (`fn f(mut x)`), granting mutable capability over the
            // bound name for the body. A receiver's `mut` additionally marks
            // the method as mutating its caller's value (§16.6).
            let mutable = self.eat(&Tok::Mut);
            let name = if receiver && first {
                match self.at() {
                    Tok::Ident(n) if n == "self" => {
                        self.bump();
                        "self".to_string()
                    }
                    other => {
                        let found = other.describe();
                        return Err(Diag::new(
                            codes::EXPECTED,
                            format!("a method's first parameter must be `self`, found {found}"),
                            span,
                        ));
                    }
                }
            } else {
                self.ident("parameter name")?
            };
            first = false;
            let ty = if self.eat(&Tok::Colon) {
                Some(self.ty()?)
            } else {
                None
            };
            out.push(Param {
                name,
                ty,
                mutable,
                span,
            });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.expect(&Tok::RParen)?;
                return Ok(out);
            }
            self.skip_newlines();
            // A trailing comma before the closing `)` is allowed.
            if self.eat(&Tok::RParen) {
                return Ok(out);
            }
        }
    }

    fn struct_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("struct name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::LBrace)?;
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            let fspan = self.span();
            let fpublic = self.eat(&Tok::Pub);
            let fname = self.ident("field name")?;
            self.expect(&Tok::Colon)?;
            let fty = self.ty()?;
            fields.push(FieldDecl {
                name: fname,
                ty: fty,
                public: fpublic,
                span: fspan,
            });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.skip_newlines();
                self.expect(&Tok::RBrace)?;
                break;
            }
        }
        Ok(Item::Struct {
            name,
            type_params,
            fields,
            public,
            span,
        })
    }

    fn enum_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("enum name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::LBrace)?;
        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Tok::RBrace) {
                break;
            }
            let vspan = self.span();
            let vname = self.ident("variant name")?;
            let mut payload = Vec::new();
            if self.eat(&Tok::LParen) {
                self.skip_newlines();
                if !self.eat(&Tok::RParen) {
                    loop {
                        self.skip_newlines();
                        payload.push(self.ty()?);
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) {
                            self.expect(&Tok::RParen)?;
                            break;
                        }
                        self.skip_newlines();
                        if self.eat(&Tok::RParen) {
                            break;
                        }
                    }
                }
            }
            variants.push(VariantDecl {
                tag: vname,
                payload,
                span: vspan,
            });
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.skip_newlines();
                self.expect(&Tok::RBrace)?;
                break;
            }
        }
        Ok(Item::Enum {
            name,
            type_params,
            variants,
            public,
            span,
        })
    }

    fn alias_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let name = self.ident("type alias name")?;
        let type_params = self.opt_type_params()?;
        self.expect(&Tok::Assign)?;
        let target = self.ty()?;
        self.end_stmt()?;
        Ok(Item::Alias {
            name,
            type_params,
            target,
            public,
            span,
        })
    }

    fn use_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        let mut path = vec![self.ident("module or item name")?];
        // The canonical module path separator is `::` (§27, §34). The historical
        // dotted `use a.b.c` spelling is rejected rather than kept as a second
        // spelling; ordinary field/method `.` is unaffected.
        loop {
            if self.eat(&Tok::ColonColon) {
                path.push(self.ident("module path segment")?);
            } else if matches!(self.at(), Tok::Dot) {
                return Err(Diag::new(
                    codes::EXPECTED,
                    "module paths use `::`, not `.`; write `use a::b` instead of `use a.b`",
                    self.span(),
                ));
            } else {
                break;
            }
        }
        // `use path as Alias` binds the imported item under `Alias` instead of
        // its own name. `as` is a reserved token.
        let alias = if self.eat(&Tok::As) {
            Some(self.ident("import alias")?)
        } else {
            None
        };
        self.end_stmt()?;
        Ok(Item::Use {
            path,
            alias,
            public,
            span,
        })
    }

    /// Parse a `::`-separated path beginning at the current identifier. The
    /// first segment is assumed to be present. Returns the segments and the
    /// span of the whole path.
    fn path_segments(&mut self) -> Result<(Vec<String>, Span)> {
        let start = self.span().start;
        let mut segments = vec![self.ident("name")?];
        while self.eat(&Tok::ColonColon) {
            segments.push(self.ident("path segment")?);
        }
        let end = self.toks[self.pos.saturating_sub(1)].span.end;
        Ok((segments, Span::new(start, end)))
    }

    fn const_item(&mut self, public: bool) -> Result<Item> {
        let span = self.span();
        self.bump();
        if self.eat(&Tok::Mut) {
            return Err(Diag::new(
                codes::EXPECTED,
                "top-level bindings are module constants and are always immutable; `mut` is not allowed here",
                span,
            ));
        }
        let name = self.ident("binding name")?;
        let ann = if self.eat(&Tok::Colon) {
            Some(self.ty()?)
        } else {
            None
        };
        self.expect(&Tok::Assign)?;
        let value = self.expr()?;
        self.end_stmt()?;
        Ok(Item::Const {
            name,
            ann,
            value,
            public,
            span,
        })
    }

    fn ident(&mut self, what: &str) -> Result<String> {
        match self.bump() {
            Token {
                tok: Tok::Ident(n), ..
            } => Ok(n),
            other => {
                let found = other.tok.describe();
                let code = if is_keyword(&other.tok) {
                    codes::RESERVED_NAME
                } else {
                    codes::EXPECTED
                };
                Err(Diag::new(
                    code,
                    format!("expected {what}, found {found}"),
                    other.span,
                ))
            }
        }
    }

    // ---------------------------------------------------------------- types

    fn ty(&mut self) -> Result<TypeExpr> {
        // A generic application nests (`Box<Box<int>>`), so a type recurses
        // through the same host-stack backstop as an expression. An over-deep
        // annotation is `E1015`, never a host-stack trap. Structural type
        // nesting counts toward the same semantic budget as every other node
        // kind (ADR-0004), so the limit is identical on native and WASM. A
        // *flat* union is a loop over members and stays at one level, so a long
        // union is still accepted.
        self.type_depth += 1;
        if self.type_depth > MAX_AST_DEPTH {
            self.type_depth -= 1;
            return Err(Diag::new(
                codes::NESTING,
                "type nests too deeply",
                self.span(),
            ));
        }
        self.enter()?;
        let r = self.ty_inner();
        self.leave();
        self.type_depth -= 1;
        r
    }

    fn ty_inner(&mut self) -> Result<TypeExpr> {
        let first = self.ty_member()?;
        if !self.eat(&Tok::Bar) {
            return Ok(first);
        }
        let mut members = vec![first];
        loop {
            members.push(self.ty_member()?);
            if !self.eat(&Tok::Bar) {
                break;
            }
        }
        Ok(TypeExpr::Union(members))
    }

    /// One member of a type expression (a union member).
    ///
    /// In a type position a `<` after a name is unambiguously a generic
    /// application (there is no comparison operator in a type), so
    /// `Name<T, U>` is parsed directly. `::`-qualified paths are joined.
    fn ty_member(&mut self) -> Result<TypeExpr> {
        Ok(match self.at().clone() {
            Tok::Ident(id) => {
                // A `::`-qualified path names an item in another module
                // (`shapes::Point`). The canonical name is joined with `::`.
                let name = if matches!(
                    self.toks.get(self.pos + 1).map(|t| &t.tok),
                    Some(Tok::ColonColon)
                ) {
                    let (segments, _) = self.path_segments()?;
                    segments.join("::")
                } else {
                    self.bump();
                    id
                };
                // A primitive has no arguments; every other name may be
                // applied. `int<...>` is left to the checker to reject.
                if self.eat(&Tok::Lt) {
                    let mut args = Vec::new();
                    loop {
                        self.skip_newlines();
                        args.push(self.ty()?);
                        self.skip_newlines();
                        if self.eat(&Tok::Comma) {
                            self.skip_newlines();
                            if matches!(self.at(), Tok::Gt | Tok::Shr) {
                                self.expect_close_angle()?;
                                break;
                            }
                            continue;
                        }
                        self.expect_close_angle()?;
                        break;
                    }
                    if matches!(name.as_str(), "int" | "float" | "bool" | "string") {
                        return Err(Diag::new(
                            codes::EXPECTED,
                            format!("`{name}` is not a generic type and takes no type arguments"),
                            self.span(),
                        ));
                    }
                    TypeExpr::App(name, args)
                } else {
                    match name.as_str() {
                        "int" => TypeExpr::Int,
                        "float" => TypeExpr::Float,
                        "bool" => TypeExpr::Bool,
                        "string" => TypeExpr::String,
                        // `never` is the bottom type (§4.3). It is an ordinary
                        // identifier elsewhere (`let never = 1`) and only takes
                        // on its type meaning in a type position.
                        "never" => TypeExpr::Never,
                        _ => TypeExpr::Named(name),
                    }
                }
            }
            Tok::None => {
                self.bump();
                TypeExpr::None
            }
            Tok::LBracket => {
                self.bump();
                let inner = self.ty()?;
                self.expect(&Tok::RBracket)?;
                TypeExpr::List(Box::new(inner))
            }
            Tok::LBrace => {
                self.bump();
                let k = self.ty()?;
                self.expect(&Tok::Colon)?;
                let v = self.ty()?;
                self.expect(&Tok::RBrace)?;
                TypeExpr::Map(Box::new(k), Box::new(v))
            }
            other => {
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("expected a type, found {}", other.describe()),
                    self.span(),
                ))
            }
        })
    }

    // --------------------------------------------------------------- blocks

    /// Parse a `{ ... }` block into a shareable statement sequence.
    fn block(&mut self) -> Result<Arc<[Stmt]>> {
        Ok(self.block_vec()?.into())
    }

    fn block_vec(&mut self) -> Result<Vec<Stmt>> {
        self.enter()?;
        self.expect(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        loop {
            while matches!(self.at(), Tok::Newline | Tok::Semi) {
                self.bump();
            }
            if matches!(self.at(), Tok::RBrace | Tok::Eof) {
                break;
            }
            stmts.push(self.stmt()?);
        }
        self.expect(&Tok::RBrace)?;
        self.leave();
        Ok(stmts)
    }

    fn stmt(&mut self) -> Result<Stmt> {
        self.enter()?;
        // Each statement gets its own AST-node budget; nested expressions
        // within it accumulate (§31.1).
        let saved_nodes = self.expr_nodes;
        self.expr_nodes = 0;
        let r = self.stmt_inner();
        self.expr_nodes = saved_nodes;
        self.leave();
        r
    }

    fn stmt_inner(&mut self) -> Result<Stmt> {
        let span = self.span();
        match self.at().clone() {
            Tok::Let => {
                self.bump();
                let mutable = self.eat(&Tok::Mut);
                // A `let` binds either a single identifier (the ordinary,
                // optionally annotated and optionally mutable form) or a
                // destructuring pattern (§4.7). Parse the pattern first, then
                // route a lone binding name through the ordinary path so
                // `let x`, `let mut x`, and `let x: T` keep their exact
                // behavior.
                let pattern = self.let_pattern()?;
                if let Pattern::Bind(name, _) = pattern {
                    let ann = if self.eat(&Tok::Colon) {
                        Some(self.ty()?)
                    } else {
                        None
                    };
                    if !self.eat(&Tok::Assign) {
                        return Err(Diag::new(
                            codes::LET_NO_INIT,
                            format!("`let {name}` needs an initializer: `let {name} = ...`"),
                            span,
                        ));
                    }
                    let value = self.expr()?;
                    self.end_stmt()?;
                    return Ok(Stmt::Let {
                        mutable,
                        name,
                        ann,
                        value,
                        span,
                    });
                }
                // Genuine destructuring: `mut` and annotations are not allowed
                // (§4.7), and the pattern must be followed by `=`.
                if mutable {
                    return Err(Diag::new(
                        codes::EXPECTED,
                        "`mut` is not allowed on a destructuring `let`; every name it binds is immutable",
                        span,
                    ));
                }
                if self.eat(&Tok::Colon) {
                    return Err(Diag::new(
                        codes::EXPECTED,
                        "a destructuring `let` cannot have a type annotation",
                        span,
                    ));
                }
                if !self.eat(&Tok::Assign) {
                    return Err(Diag::new(
                        codes::LET_NO_INIT,
                        "a destructuring `let` needs an initializer: `let <pattern> = ...`",
                        span,
                    ));
                }
                let value = self.expr()?;
                self.end_stmt()?;
                Ok(Stmt::LetPattern {
                    pattern,
                    value,
                    span,
                })
            }
            Tok::Return => {
                self.bump();
                let value = if starts_expr(self.at()) {
                    Some(self.expr()?)
                } else {
                    None
                };
                self.end_stmt()?;
                Ok(Stmt::Return(value, span))
            }
            Tok::Throw => {
                self.bump();
                let value = self.expr()?;
                self.end_stmt()?;
                Ok(Stmt::Throw(value, span))
            }
            Tok::Break => {
                self.bump();
                self.end_stmt()?;
                Ok(Stmt::Break(span))
            }
            Tok::Continue => {
                self.bump();
                self.end_stmt()?;
                Ok(Stmt::Continue(span))
            }
            Tok::While => {
                self.bump();
                let cond = self.expr()?;
                let body = self.block()?;
                Ok(Stmt::While(cond, body, span))
            }
            Tok::Loop => {
                self.bump();
                let body = self.block()?;
                Ok(Stmt::Loop(body, span))
            }
            Tok::For => {
                self.bump();
                let pat = self.pattern()?;
                self.expect(&Tok::In)?;
                let iter = self.expr()?;
                let body = self.block()?;
                Ok(Stmt::For(pat, iter, body, span))
            }
            Tok::Try => {
                self.bump();
                let body = self.block()?;
                self.expect(&Tok::Catch)?;
                // The catch clause is a full pattern (RFC 0001): a bare
                // identifier is `Pattern::Bind`, and a variant pattern selects
                // nominally. It is followed directly by its block:
                // `catch e { ... }` / `catch MyErr::Bad(m) { ... }`. There is
                // no `->` here (`LANGUAGE_SPEC.md` §14.5).
                let catch = self.pattern()?;
                let catch_body = self.block()?;
                let finally = if self.eat(&Tok::Finally) {
                    Some(self.block()?)
                } else {
                    None
                };
                self.end_stmt()?;
                Ok(Stmt::Try {
                    body,
                    catch,
                    catch_body,
                    finally,
                    span,
                })
            }
            _ => {
                let expr = self.expr()?;
                let span = span_of(&expr);
                let op = match self.at() {
                    Tok::Assign => Some(None),
                    Tok::PlusEq => Some(Some(BinOp::Add)),
                    Tok::MinusEq => Some(Some(BinOp::Sub)),
                    Tok::StarEq => Some(Some(BinOp::Mul)),
                    Tok::SlashEq => Some(Some(BinOp::Div)),
                    Tok::PercentEq => Some(Some(BinOp::Rem)),
                    Tok::CaretEq => Some(Some(BinOp::Pow)),
                    Tok::BarEq => Some(Some(BinOp::BitOr)),
                    Tok::AmpEq => Some(Some(BinOp::BitAnd)),
                    Tok::ShlEq => Some(Some(BinOp::Shl)),
                    Tok::ShrEq => Some(Some(BinOp::Shr)),
                    _ => None,
                };
                if let Some(compound) = op {
                    self.bump();
                    let value = self.expr()?;
                    self.end_stmt()?;
                    return Ok(Stmt::Assign {
                        target: expr,
                        value,
                        op: compound,
                        span,
                    });
                }
                self.end_stmt()?;
                Ok(Stmt::Expr(expr, span))
            }
        }
    }

    /// Parse a `let`-position pattern (§4.7).
    ///
    /// This is the restricted subset of [`Self::pattern`] accepted by `let`:
    /// binding names, list patterns, variant patterns, and any nesting of
    /// those. Literal and `none` patterns are rejected with `E1006`, and the
    /// general [`Self::pattern`] used by `for` and `match` is left untouched.
    fn let_pattern(&mut self) -> Result<Pattern> {
        self.enter()?;
        let r = self.let_pattern_inner();
        self.leave();
        r
    }

    fn let_pattern_inner(&mut self) -> Result<Pattern> {
        let span = self.span();
        match self.at().clone() {
            Tok::Ident(n) => {
                self.bump();
                if self.eat(&Tok::LParen) {
                    let mut ps = Vec::new();
                    self.skip_newlines();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            self.skip_newlines();
                            ps.push(self.let_pattern()?);
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                            self.skip_newlines();
                            if self.eat(&Tok::RParen) {
                                break;
                            }
                        }
                    }
                    Ok(Pattern::Variant(n, ps, span))
                } else if n.chars().next().is_some_and(char::is_uppercase) {
                    Ok(Pattern::Variant(n, Vec::new(), span))
                } else {
                    Ok(Pattern::Bind(n, span))
                }
            }
            Tok::LBracket => {
                self.bump();
                let mut parts = Vec::new();
                self.skip_newlines();
                if !self.eat(&Tok::RBracket) {
                    loop {
                        self.skip_newlines();
                        parts.push(self.let_pattern()?);
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) {
                            self.skip_newlines();
                            self.expect(&Tok::RBracket)?;
                            break;
                        }
                        self.skip_newlines();
                        // A trailing comma before `]` is allowed (§4.7).
                        if matches!(self.at(), Tok::RBracket) {
                            self.bump();
                            break;
                        }
                    }
                }
                Ok(Pattern::List(parts, span))
            }
            other => {
                // Literal and `none` patterns are unsupported in `let` and
                // report `E1006`; a reserved word in binding position keeps
                // its dedicated `E1009` diagnostic (matching the ordinary
                // `let x` path); anything else is an unsupported pattern.
                let code = match other {
                    Tok::Int(_) | Tok::Str(_) | Tok::True | Tok::False | Tok::None => {
                        codes::EXPECTED
                    }
                    ref t if is_keyword(t) => codes::RESERVED_NAME,
                    _ => codes::EXPECTED,
                };
                Err(Diag::new(
                    code,
                    format!(
                        "expected a binding, list, or variant pattern in `let`, found {}",
                        other.describe()
                    ),
                    span,
                ))
            }
        }
    }

    fn pattern(&mut self) -> Result<Pattern> {
        self.enter()?;
        let r = self.pattern_inner();
        self.leave();
        r
    }

    /// The optional `if condition` clause of a comprehension. Returns `None`
    /// when no `if` follows. Exactly one generator and zero or one filter are
    /// supported (`LANGUAGE_SPEC.md` §24/§25); a second `for` or `if` is left
    /// for the caller's terminator check to reject.
    fn comprehension_filter(&mut self) -> Result<Option<Expr>> {
        if self.eat(&Tok::If) {
            Ok(Some(self.expr()?))
        } else {
            Ok(None)
        }
    }

    fn pattern_inner(&mut self) -> Result<Pattern> {
        let span = self.span();
        match self.at().clone() {
            Tok::Ident(n) => {
                self.bump();
                // A `::`-qualified pattern names a variant in another module
                // (`shapes::Color::Red`); join the segments into one canonical
                // name, exactly like a construct expression.
                let n = if matches!(self.at(), Tok::ColonColon) {
                    let mut segs = vec![n];
                    while self.eat(&Tok::ColonColon) {
                        segs.push(self.ident("path segment")?);
                    }
                    segs.join("::")
                } else {
                    n
                };
                if self.eat(&Tok::LParen) {
                    let mut ps = Vec::new();
                    self.skip_newlines();
                    if !self.eat(&Tok::RParen) {
                        loop {
                            self.skip_newlines();
                            ps.push(self.pattern()?);
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                self.expect(&Tok::RParen)?;
                                break;
                            }
                            self.skip_newlines();
                            if self.eat(&Tok::RParen) {
                                break;
                            }
                        }
                    }
                    Ok(Pattern::Variant(n, ps, span))
                } else if n
                    .rsplit("::")
                    .next()
                    .unwrap_or(&n)
                    .chars()
                    .next()
                    .is_some_and(char::is_uppercase)
                {
                    Ok(Pattern::Variant(n, Vec::new(), span))
                } else {
                    Ok(Pattern::Bind(n, span))
                }
            }
            Tok::Int(v) => {
                self.bump();
                Ok(Pattern::Int(v, span))
            }
            Tok::Str(s) => {
                self.bump();
                Ok(Pattern::Str(s, span))
            }
            Tok::True => {
                self.bump();
                Ok(Pattern::Bool(true, span))
            }
            Tok::False => {
                self.bump();
                Ok(Pattern::Bool(false, span))
            }
            Tok::None => {
                self.bump();
                Ok(Pattern::None(span))
            }
            Tok::LBracket => {
                self.bump();
                let mut parts = Vec::new();
                self.skip_newlines();
                if !self.eat(&Tok::RBracket) {
                    loop {
                        self.skip_newlines();
                        parts.push(self.pattern()?);
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) {
                            self.skip_newlines();
                            self.expect(&Tok::RBracket)?;
                            break;
                        }
                        self.skip_newlines();
                        // A trailing comma before `]` is allowed (§4.6).
                        if matches!(self.at(), Tok::RBracket) {
                            self.bump();
                            break;
                        }
                    }
                }
                Ok(Pattern::List(parts, span))
            }
            other => Err(Diag::new(
                codes::EXPECTED,
                format!("expected a pattern, found {}", other.describe()),
                span,
            )),
        }
    }

    // ---------------------------------------------------------- expressions

    fn expr(&mut self) -> Result<Expr> {
        self.enter()?;
        // The AST-node budget is per *statement*, not per nested expression.
        // Resetting it here would give every nesting level its own 256-node
        // allowance, so a chain of nested calls (`f(f(f(…)))`) could recurse
        // far past the semantic limit and overflow the host stack before the
        // post-parse depth check could report `E1015`. The counter is reset
        // once per statement in `stmt_inner` (and per top-level item
        // expression); sub-expressions accumulate into it.
        let r = self.expr_bp(0);
        self.leave();
        r
    }

    fn expr_bp(&mut self, min_bp: u8) -> Result<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let span = self.span();
            // `a..b` — Rust-style range. Its binding power sits above every
            // bitwise operator and below additive (20/21), so both operands are
            // arithmetic expressions: `1 + 2..n - 1` is `(1 + 2)..(n - 1)`.
            // Ranges are right-associative and not chainable with meaning;
            // `a..b..c` parses as `a..(b..c)` and is a runtime type error at
            // evaluation, like any other non-int bound.
            if matches!(self.at(), Tok::DotDot) {
                let (lbp, rbp) = (17, 17);
                if lbp < min_bp {
                    break;
                }
                self.bump();
                if matches!(self.at(), Tok::Eof | Tok::Newline) {
                    return Err(Diag::new(
                        codes::EXPECTED,
                        "expected an expression after `..`",
                        self.span(),
                    ));
                }
                self.count_node(span)?;
                let rhs = self.expr_bp(rbp)?;
                lhs = Expr::Range(Arc::new(lhs), Arc::new(rhs), span);
                continue;
            }
            let Some((lbp, rbp, op)) = infix(self.at()) else {
                break;
            };
            if lbp < min_bp {
                break;
            }
            self.bump();
            self.count_node(span)?;
            match op {
                None => {
                    // Pipeline: `x |> f` calls `f(x)`; `x |> f(a)` calls
                    // `f(x, a)`. The left operand becomes the first argument.
                    let rhs = self.expr_bp(rbp)?;
                    let piped = desugar_pipe(lhs, rhs, span);
                    lhs = piped;
                }
                Some(op) => {
                    let rhs = self.expr_bp(rbp)?;
                    lhs = Expr::Binary(op, Arc::new(lhs), Arc::new(rhs), span);
                }
            }
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr> {
        let span = self.span();
        match self.at().clone() {
            Tok::Minus => {
                self.bump();
                // `-9223372036854775808` is `i64::MIN`; the magnitude alone is
                // out of range and is only valid directly under this minus.
                if matches!(self.at(), Tok::IntMinMagnitude) {
                    self.bump();
                    return Ok(Expr::Lit(Lit::Int(i64::MIN), span));
                }
                // A chain of unary operators (`- - - x`, `not not x`,
                // `~ ~ x`) recurses through `unary()` directly. It must be
                // bounded so a long chain cannot exhaust the native stack
                // (`LANGUAGE_SPEC.md` §31.5). It is bounded by the *AST-level*
                // counter, not the host-stack `depth`: each unary operator is
                // one AST level, so `MAX_AST_DEPTH` is the exact limit and the
                // bound composes with grouping (`enter`/`leave`), which must
                // stay free to accept any AST-valid program (§31.2).
                self.count_node(span)?;
                let e = self.unary()?;
                Ok(Expr::Unary(UnOp::Neg, Arc::new(e), span))
            }
            Tok::Not => {
                self.bump();
                self.count_node(span)?;
                let e = self.unary()?;
                Ok(Expr::Unary(UnOp::Not, Arc::new(e), span))
            }
            Tok::Tilde => {
                self.bump();
                self.count_node(span)?;
                let e = self.unary()?;
                Ok(Expr::Unary(UnOp::BitNot, Arc::new(e), span))
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> Result<Expr> {
        let mut e = self.atom()?;
        loop {
            // Only count a node when this iteration actually wraps `e`; a
            // plain operand must not consume the postfix budget. Counting
            // unconditionally made the effective flat-expression limit half
            // the documented one.
            match self.at().clone() {
                Tok::LParen => {
                    let span = self.span();
                    self.count_node(span)?;
                    self.bump();
                    let args = self.call_args()?;
                    e = Expr::Call(Arc::new(e), args.into(), Vec::new(), span);
                }
                Tok::LBracket => {
                    let span = self.span();
                    self.count_node(span)?;
                    self.bump();
                    let idx = self.expr()?;
                    self.expect(&Tok::RBracket)?;
                    e = Expr::Index(Arc::new(e), Arc::new(idx), span);
                }
                Tok::Dot => {
                    let span = self.span();
                    self.count_node(span)?;
                    self.bump();
                    let name = self.ident("field or method name")?;
                    let targs = self.ty_args()?;
                    if matches!(self.at(), Tok::LParen) {
                        self.bump();
                        let args = self.call_args()?;
                        e = Expr::Method(Arc::new(e), name, args.into(), targs, span);
                    } else if !targs.is_empty() {
                        return Err(Diag::new(
                            codes::EXPECTED,
                            format!(
                                "type arguments on `.{name}` require a call: `.{name}<...>(...)`"
                            ),
                            span,
                        ));
                    } else {
                        e = Expr::Field(Arc::new(e), name, span);
                    }
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// Parse the `{ field: value, ... }` body of a struct literal, after the
    /// opening brace has been consumed by the caller's position. Assumes the
    /// current token is `{`.
    fn construct_fields(&mut self, _name: String, _span: Span) -> Result<Vec<Arg>> {
        self.expect(&Tok::LBrace)?;
        let mut args = Vec::new();
        self.skip_newlines();
        if !self.eat(&Tok::RBrace) {
            loop {
                self.skip_newlines();
                let fname = self.ident("field name")?;
                self.expect(&Tok::Colon)?;
                let value = self.expr()?;
                args.push(Arg {
                    name: Some(fname),
                    value,
                });
                self.skip_newlines();
                if !self.eat(&Tok::Comma) {
                    self.expect(&Tok::RBrace)?;
                    break;
                }
                // A trailing comma before `}` is allowed.
                self.skip_newlines();
                if self.eat(&Tok::RBrace) {
                    break;
                }
            }
        }
        Ok(args)
    }

    /// Parse the `(v, ...)` body of a variant construction, after the opening
    /// paren has been consumed by the caller.
    fn construct_positional(&mut self, _span: Span) -> Result<Vec<Arg>> {
        self.expect(&Tok::LParen)?;
        let mut args = Vec::new();
        self.skip_newlines();
        if !self.eat(&Tok::RParen) {
            loop {
                self.skip_newlines();
                args.push(self.cons_arg()?);
                self.skip_newlines();
                if !self.eat(&Tok::Comma) {
                    self.expect(&Tok::RParen)?;
                    break;
                }
                // A trailing comma before `)` is allowed.
                self.skip_newlines();
                if self.eat(&Tok::RParen) {
                    break;
                }
            }
        }
        Ok(args)
    }

    /// Parse a parenthesized call argument list, up to and including `)`.
    ///
    /// Arguments may be positional (`expr`) or named (`name: expr`). Named
    /// arguments are supported only for directly resolved user functions, but
    /// the parser accepts the form and the checker validates it. Positional
    /// arguments MUST precede named arguments; a positional argument after a
    /// named one is `E1006` (`LANGUAGE_SPEC.md` §4.5).
    fn call_args(&mut self) -> Result<Vec<Arg>> {
        let mut out = Vec::new();
        self.skip_newlines();
        if self.eat(&Tok::RParen) {
            return Ok(out);
        }
        let mut seen_named = false;
        loop {
            self.skip_newlines();
            let arg = self.cons_arg()?;
            if arg.name.is_some() {
                seen_named = true;
            } else if seen_named {
                return Err(Diag::new(
                    codes::EXPECTED,
                    "positional arguments must come before named arguments",
                    span_of(&arg.value),
                ));
            }
            out.push(arg);
            self.skip_newlines();
            if !self.eat(&Tok::Comma) {
                self.expect(&Tok::RParen)?;
                return Ok(out);
            }
            // A trailing comma before the closing `)` is allowed, so a list of
            // arguments may be extended line by line.
            self.skip_newlines();
            if self.eat(&Tok::RParen) {
                return Ok(out);
            }
        }
    }

    /// Consume one unit of the flat-expression node budget, reporting the
    /// semantic nesting diagnostic if it is exceeded.
    fn count_node(&mut self, span: Span) -> Result<()> {
        self.expr_nodes += 1;
        if self.expr_nodes > MAX_AST_DEPTH {
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                span,
            ));
        }
        Ok(())
    }

    fn atom(&mut self) -> Result<Expr> {
        self.enter()?;
        let r = self.atom_inner();
        self.leave();
        r
    }

    /// Consume one level of *container* atom nesting (`[...]`, `{...}`, and a
    /// parenthesized comma-list). Pure grouping parentheses create no AST node,
    /// so they are not counted — exactly as [`check_expr_depth`] does not count
    /// them. Counting here means over-deep nested literals report the same
    /// `E1015` the post-parse walk would, but *during* parsing, before the
    /// recursive descent can exhaust a substrate whose physical stack ceiling
    /// is below the recursion budget (§31.2/§31.5).
    fn enter_container(&mut self) -> Result<()> {
        self.atom_depth += 1;
        if self.atom_depth > MAX_AST_DEPTH {
            self.atom_depth -= 1;
            return Err(Diag::new(
                codes::NESTING,
                "expression nests too deeply",
                self.span(),
            ));
        }
        Ok(())
    }

    fn leave_container(&mut self) {
        self.atom_depth -= 1;
    }

    /// Finish the `[value for pattern in iterable (if filter)?]` form once the
    /// leading `for` has been consumed. Kept out of `atom_inner` and
    /// `#[inline(never)]` so the grouping recursion path's frame stays small
    /// (see the call site).
    #[inline(never)]
    fn finish_list_comprehension(&mut self, value: Expr, span: Span) -> Result<Expr> {
        let pattern = self.pattern()?;
        self.expect(&Tok::In)?;
        let iterable = self.expr()?;
        self.skip_newlines();
        let filter = self.comprehension_filter()?;
        self.skip_newlines();
        self.expect(&Tok::RBracket)?;
        Ok(Expr::ListComp {
            value: Arc::new(value),
            pattern,
            iterable: Arc::new(iterable),
            filter: filter.map(Arc::new),
            span,
        })
    }

    /// Finish the `{key: value for pattern in iterable (if filter)?}` form once
    /// the leading `for` has been consumed. Kept out of `atom_inner` and
    /// `#[inline(never)]` for the same reason as [`Self::finish_list_comprehension`].
    #[inline(never)]
    fn finish_map_comprehension(&mut self, key: Expr, value: Expr, span: Span) -> Result<Expr> {
        let pattern = self.pattern()?;
        self.expect(&Tok::In)?;
        let iterable = self.expr()?;
        self.skip_newlines();
        let filter = self.comprehension_filter()?;
        self.skip_newlines();
        self.expect(&Tok::RBrace)?;
        Ok(Expr::MapComp {
            key: Arc::new(key),
            value: Arc::new(value),
            pattern,
            iterable: Arc::new(iterable),
            filter: filter.map(Arc::new),
            span,
        })
    }

    fn atom_inner(&mut self) -> Result<Expr> {
        let span = self.span();
        let e = match self.at().clone() {
            Tok::Int(v) => {
                self.bump();
                Expr::Lit(Lit::Int(v), span)
            }
            Tok::IntMinMagnitude => {
                return Err(Diag::new(
                    codes::INVALID_NUMBER,
                    "integer out of range; `9223372036854775808` is only valid as part of `-9223372036854775808`",
                    span,
                ))
            }
            Tok::Float(v) => {
                self.bump();
                Expr::Lit(Lit::Float(v), span)
            }
            Tok::Str(s) => {
                self.bump();
                Expr::Lit(Lit::Str(s), span)
            }
            Tok::True => {
                self.bump();
                Expr::Lit(Lit::Bool(true), span)
            }
            Tok::False => {
                self.bump();
                Expr::Lit(Lit::Bool(false), span)
            }
            Tok::None => {
                self.bump();
                Expr::Lit(Lit::None, span)
            }
            Tok::FStr(raw) => {
                self.bump();
                let parts = self.fstring(&raw, span)?;
                Expr::FStr(parts.into(), span)
            }
            Tok::Ident(name) => {
                // A `::`-qualified path names an item in another module. Join
                // the segments into one canonical name so the checker and
                // runtime resolve it exactly like a flat declaration.
                let name = if matches!(
                    self.toks.get(self.pos + 1).map(|t| &t.tok),
                    Some(Tok::ColonColon)
                ) {
                    let (segments, _) = self.path_segments()?;
                    segments.join("::")
                } else {
                    self.bump();
                    name
                };
                let last = name.rsplit("::").next().unwrap_or(name.as_str());
                let is_type_name = last.chars().next().is_some_and(char::is_uppercase);
                // Explicit generic type arguments: `identity<int>(x)` or
                // `Box<int> { value: 1 }`. A balanced `<...>` followed by `(`
                // or `{` is a generic application; every other `<` stays a
                // comparison (see `at_type_args`).
                if self.at_type_args() {
                    let targs = self.ty_args()?;
                    // A generic head may continue through `::` to a qualified
                    // variant: `Result<int, string>::Ok(1)` (§4.9,
                    // CONF-GRAM-4). The type arguments stay with the head; the
                    // trailing segment is the variant tag. This continues the
                    // existing qualified-path semantics and does not introduce
                    // associated functions or other associated items.
                    let qualified_variant = if self.eat(&Tok::ColonColon) {
                        Some(self.ident("variant name")?)
                    } else {
                        None
                    };
                    if let Some(variant) = qualified_variant {
                        let full = format!("{name}::{variant}");
                        if matches!(self.at(), Tok::LBrace) {
                            let args = self.construct_fields(full.clone(), span)?;
                            return Ok(Expr::Construct(full, args.into(), targs, span));
                        }
                        let args = self.construct_positional(span)?;
                        return Ok(Expr::Construct(full, args.into(), targs, span));
                    }
                    if matches!(self.at(), Tok::LBrace) {
                        let args = self.construct_fields(name.clone(), span)?;
                        return Ok(Expr::Construct(name, args.into(), targs, span));
                    }
                    // The only other suffix `at_type_args` accepts is `(`.
                    self.expect(&Tok::LParen)?;
                    let args = self.call_args()?;
                    return Ok(Expr::Call(Arc::new(Expr::Name(name, span)), args.into(), targs, span));
                }
                if matches!(self.at(), Tok::LBrace) && is_type_name {
                    // struct literal `Point { x: 1 }`
                    let args = self.construct_fields(name.clone(), span)?;
                    return Ok(Expr::Construct(name, args.into(), Vec::new(), span));
                }
                if matches!(self.at(), Tok::LParen) && is_type_name {
                    // variant constructor `Ok(x)`
                    let args = self.construct_positional(span)?;
                    return Ok(Expr::Construct(name, args.into(), Vec::new(), span));
                }
                Expr::Name(name, span)
            }
            Tok::LParen => {
                self.bump();
                // lambda?
                if let Some(params) = self.try_lambda_params() {
                    let body = self.expr()?;
                    return Ok(Expr::Lambda(params, Arc::new(body), span));
                }
                let first = self.expr()?;
                if self.eat(&Tok::Comma) {
                    // A parenthesized comma-list is list sugar and one AST
                    // level; bound its nesting during parsing. Pure grouping
                    // parentheses (no comma) are not counted, matching
                    // `check_expr_depth` (see `enter_container`).
                    self.enter_container()?;
                    let mut items = vec![first];
                    self.skip_newlines();
                    // `(x,)` is a one-element list (§21); the grammar's
                    // optional tail after the comma may be empty.
                    if !matches!(self.at(), Tok::RParen) {
                        loop {
                            self.skip_newlines();
                            items.push(self.expr()?);
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                break;
                            }
                            self.skip_newlines();
                            if matches!(self.at(), Tok::RParen) {
                                break;
                            }
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    self.leave_container();
                    Expr::Tuple(items.into(), span)
                } else {
                    self.expect(&Tok::RParen)?;
                    first
                }
            }
            Tok::LBracket => {
                // A list literal is one AST level; bound its nesting during
                // parsing (see `enter_container`).
                self.enter_container()?;
                self.bump();
                self.skip_newlines();
                if self.eat(&Tok::RBracket) {
                    self.leave_container();
                    return Ok(Expr::List(Arc::from([]), span));
                }
                self.skip_newlines();
                let first = self.expr()?;
                // `[value for pattern in iterable (if filter)?]` — one
                // generator clause and at most one filter (§24). The clauses
                // may be split across lines inside the brackets, so a newline
                // before `for`/`if` is insignificant here. `for` is a statement
                // keyword and cannot begin an expression, so it is unambiguous.
                //
                // The comprehension body is parsed in a non-inlined helper:
                // `atom_inner` is on the grouping recursion path
                // (`[` → expr → unary → postfix → atom → expr), and keeping its
                // frame small preserves the substrate stack margin that lets
                // over-limit grouping report `E1015` instead of trapping on
                // wasm (§31.2, §31.5).
                self.skip_newlines();
                let result = if self.eat(&Tok::For) {
                    self.finish_list_comprehension(first, span)
                } else {
                    let mut items = vec![first];
                    loop {
                        self.skip_newlines();
                        if !self.eat(&Tok::Comma) {
                            self.expect(&Tok::RBracket)?;
                            break;
                        }
                        self.skip_newlines();
                        // A trailing comma before `]` is allowed (§4.5).
                        if matches!(self.at(), Tok::RBracket) {
                            self.bump();
                            break;
                        }
                        items.push(self.expr()?);
                    }
                    Ok(Expr::List(items.into(), span))
                };
                self.leave_container();
                result?
            }
            Tok::LBrace => {
                if self.map_ahead() {
                    // A map literal is one AST level; bound its nesting during
                    // parsing (see `enter_container`).
                    self.enter_container()?;
                    self.bump();
                    let mut entries = Vec::new();
                    self.skip_newlines();
                    // `{:}` is the empty-map literal (§20.3): `{` `:` `}`.
                    // Whitespace and newlines around the colon are already
                    // skipped by the lexer and `skip_newlines`, so `{ : }`
                    // and the multi-line form are identical. `{}` never
                    // reaches this branch (`map_ahead` routes it to a block).
                    let result = if self.eat(&Tok::Colon) {
                        self.skip_newlines();
                        self.expect(&Tok::RBrace)?;
                        Ok(Expr::Map(entries.into(), span))
                    } else {
                        loop {
                            if self.eat(&Tok::RBrace) {
                                break Ok(Expr::Map(entries.into(), span));
                            }
                            let k = self.expr()?;
                            self.expect(&Tok::Colon)?;
                            self.skip_newlines();
                            let v = self.expr()?;
                            // `{key: value for pattern in iterable (if filter)?}`
                            // — one generator clause and at most one filter
                            // (§25). Parsed in a non-inlined helper so
                            // `atom_inner`'s frame stays small on the grouping
                            // recursion path.
                            self.skip_newlines();
                            if entries.is_empty() && self.eat(&Tok::For) {
                                break self.finish_map_comprehension(k, v, span);
                            }
                            entries.push((k, v));
                            self.skip_newlines();
                            if !self.eat(&Tok::Comma) {
                                self.skip_newlines();
                                self.expect(&Tok::RBrace)?;
                                break Ok(Expr::Map(entries.into(), span));
                            }
                            self.skip_newlines();
                        }
                    };
                    self.leave_container();
                    result?
                } else {
                    let body = self.block()?;
                    Expr::Block(body, span)
                }
            }
            Tok::If => {
                self.bump();
                let cond = self.expr()?;
                let then = self.block()?;
                let els = if self.eat(&Tok::Else) {
                    // `else` accepts an expression, and `if` is an expression,
                    // so `else if B { … }` parses as the nested `Expr::If`
                    // `else { if B { … } }` with no special handling.
                    Some(Arc::new(self.expr()?))
                } else {
                    None
                };
                Expr::If(Arc::new(cond), then, els, span)
            }
            Tok::Match => {
                self.bump();
                let subject = self.expr()?;
                self.expect(&Tok::LBrace)?;
                let mut arms = Vec::new();
                loop {
                    self.skip_newlines();
                    if self.eat(&Tok::RBrace) {
                        break;
                    }
                    let pat = self.pattern()?;
                    let guard = if self.eat(&Tok::If) {
                        Some(self.expr()?)
                    } else {
                        None
                    };
                    self.expect(&Tok::Arrow)?;
                    let body: Arc<[Stmt]> = if matches!(self.at(), Tok::LBrace) {
                        self.block()?
                    } else {
                        let e = self.expr()?;
                        // A match arm's expression body is terminated by `,`,
                        // `;`, a newline, or the arm block's closing `}`
                        // (CONF-PARSE-8; `,` separates arms and is not a
                        // statement separator elsewhere).
                        match self.at() {
                            Tok::Comma | Tok::Semi | Tok::Newline | Tok::RBrace | Tok::Eof => {}
                            other => {
                                return Err(Diag::new(
                                    codes::EXPECTED,
                                    format!(
                                        "expected `,`, `;`, or a newline between match arms, found {}",
                                        other.describe()
                                    ),
                                    self.span(),
                                ))
                            }
                        }
                        Arc::from([Stmt::Expr(e, span)])
                    };
                    arms.push(Arm {
                        pattern: pat,
                        guard,
                        body,
                    });
                    if matches!(self.at(), Tok::Comma | Tok::Semi) {
                        self.bump();
                    }
                }
                Expr::Match(Arc::new(subject), arms.into(), span)
            }
            Tok::Fn => {
                self.bump();
                let params = if self.eat(&Tok::LParen) {
                    self.params(false)?
                } else {
                    let pspan = self.span();
                    let mutable = self.eat(&Tok::Mut);
                    let name = self.ident("lambda parameter")?;
                    vec![Param {
                        name,
                        ty: None,
                        mutable,
                        span: pspan,
                    }]
                };
                self.expect(&Tok::Arrow)?;
                let body = self.expr()?;
                Expr::Lambda(params, Arc::new(body), span)
            }
            other => {
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("expected an expression, found {}", other.describe()),
                    span,
                ))
            }
        };
        Ok(e)
    }

    fn cons_arg(&mut self) -> Result<Arg> {
        if let Tok::Ident(name) = self.at().clone() {
            let save = self.pos;
            self.bump();
            if self.eat(&Tok::Colon) {
                let value = self.expr()?;
                return Ok(Arg {
                    name: Some(name),
                    value,
                });
            }
            self.pos = save;
        }
        let value = self.expr()?;
        Ok(Arg { name: None, value })
    }

    /// If the tokens form `(x, y) ->` or `(x: T, mut y) ->`, return the
    /// parameters and consume through the arrow. Uses the same parameter model
    /// as ordinary functions (`Param`: name, annotation, `mut`), so lambdas and
    /// functions share one signature model (`LANGUAGE_SPEC.md` §15.4).
    fn try_lambda_params(&mut self) -> Option<Vec<Param>> {
        let save = self.pos;
        let mut params = Vec::new();
        self.skip_newlines();
        // A zero-parameter lambda `() -> body` is allowed.
        if self.eat(&Tok::RParen) {
            return if self.eat(&Tok::Arrow) {
                Some(params)
            } else {
                self.pos = save;
                None
            };
        }
        loop {
            let pspan = self.span();
            let mutable = self.eat(&Tok::Mut);
            let name = match self.at().clone() {
                Tok::Ident(n) => {
                    self.bump();
                    n
                }
                _ => {
                    self.pos = save;
                    return None;
                }
            };
            let ty = if self.eat(&Tok::Colon) {
                match self.ty() {
                    Ok(t) => Some(t),
                    Err(_) => {
                        self.pos = save;
                        return None;
                    }
                }
            } else {
                None
            };
            params.push(Param {
                name,
                ty,
                mutable,
                span: pspan,
            });
            self.skip_newlines();
            if self.eat(&Tok::Comma) {
                self.skip_newlines();
                // A trailing comma before `)` is allowed.
                if self.eat(&Tok::RParen) {
                    break;
                }
                continue;
            }
            if self.eat(&Tok::RParen) {
                break;
            }
            self.pos = save;
            return None;
        }
        if self.eat(&Tok::Arrow) {
            Some(params)
        } else {
            self.pos = save;
            None
        }
    }

    fn map_ahead(&self) -> bool {
        let mut i = self.pos + 1;
        let mut depth = 0i32;
        while i < self.toks.len() {
            match &self.toks[i].tok {
                Tok::Newline => i += 1,
                Tok::RParen | Tok::RBracket | Tok::RBrace if depth == 0 => return false,
                Tok::LParen | Tok::LBracket | Tok::LBrace => {
                    depth += 1;
                    i += 1;
                }
                Tok::RParen | Tok::RBracket | Tok::RBrace => {
                    depth -= 1;
                    i += 1;
                }
                Tok::Colon if depth == 0 => return true,
                Tok::Let
                | Tok::Return
                | Tok::Throw
                | Tok::Break
                | Tok::Continue
                | Tok::While
                | Tok::Loop
                | Tok::For
                | Tok::Try
                | Tok::Semi
                    if depth == 0 =>
                {
                    return false
                }
                _ => i += 1,
            }
        }
        false
    }

    fn fstring(&mut self, raw: &str, span: Span) -> Result<Vec<FPart>> {
        let mut parts = Vec::new();
        let mut lit = String::new();
        // The raw body is a verbatim substring of the source, beginning just
        // after the `f`/`F` prefix and its opening quote. Tracking a byte
        // offset lets an interpolation's diagnostics point at the real file
        // location instead of the start of the source.
        let base = span.start + 2;
        let mut offset = 0usize;
        let mut chars = raw.chars().peekable();
        while let Some(c) = chars.next() {
            offset += c.len_utf8();
            match c {
                '{' => {
                    if chars.peek() == Some(&'{') {
                        chars.next();
                        offset += 1;
                        lit.push('{');
                        continue;
                    }
                    if !lit.is_empty() {
                        parts.push(FPart::Lit(std::mem::take(&mut lit)));
                    }
                    let inner_start = offset;
                    let mut depth = 1usize;
                    let mut closing = None;
                    for (i, c) in expression_syntax(&raw[inner_start..]) {
                        match c {
                            '}' => {
                                depth -= 1;
                                if depth == 0 {
                                    closing = Some(inner_start + i);
                                    break;
                                }
                            }
                            '{' => depth += 1,
                            _ => {}
                        }
                    }
                    let Some(end) = closing else {
                        return Err(Diag::new(
                            codes::UNTERMINATED_STRING,
                            "unterminated `{` in f-string",
                            span,
                        ));
                    };
                    let inner = &raw[inner_start..end];
                    // The outer iterator resumes just after the closing brace.
                    for _ in raw[inner_start..=end].chars() {
                        chars.next();
                    }
                    offset = end + 1;
                    if inner.trim().is_empty() {
                        return Err(Diag::new(
                            codes::EXPECTED,
                            "empty `{}` in f-string",
                            Span::new(base + inner_start, base + offset),
                        ));
                    }
                    // A top-level `:` separates the expression from its format
                    // specification. A `:` inside brackets/parens belongs to a
                    // slice or call, so split only at depth zero.
                    let (expr_src, spec_src) = split_format_spec(inner);
                    let e = parse_expr_at(expr_src, base + inner_start)?;
                    let spec = match spec_src {
                        Some((spec_text, spec_off)) => {
                            Some(parse_format_spec(spec_text, base + inner_start + spec_off)?)
                        }
                        None => None,
                    };
                    parts.push(FPart::Expr(e, spec));
                }
                '}' => {
                    if chars.peek() == Some(&'}') {
                        chars.next();
                        offset += 1;
                        lit.push('}');
                    } else {
                        // A lone `}` is not silently accepted (§3.6.4): the only
                        // way to write a literal `}` is the `}}` escape.
                        return Err(Diag::new(
                            codes::EXPECTED,
                            "a lone `}` in an f-string is not allowed; write `}}` for a literal `}`",
                            Span::new(base + offset - 1, base + offset),
                        ));
                    }
                }
                _ => lit.push(c),
            }
        }
        if !lit.is_empty() {
            parts.push(FPart::Lit(lit));
        }
        Ok(parts)
    }
}

/// Desugar `lhs |> rhs` into a call with `lhs` as the first argument:
///
/// * `lhs |> f`        -> `f(lhs)`
/// * `lhs |> f(a, b)`  -> `f(lhs, a, b)`
/// * `lhs |> r.m(a)`   -> `r.m(lhs, a)`
///
/// A bare name on the right is `f(lhs)`: desugaring it to a call (rather than
/// keeping it a function value) is what lets a piped value resolve an
/// overloaded function by its type (`LANGUAGE_SPEC.md` §23). Any other
/// right-hand side is a callable value, so it becomes `rhs(lhs)`.
fn desugar_pipe(lhs: Expr, rhs: Expr, span: Span) -> Expr {
    match rhs {
        Expr::Call(callee, args, ty_args, cspan) => {
            // The piped value becomes the first *positional* argument; any
            // named arguments follow (§23).
            let mut args = args.to_vec();
            args.insert(
                0,
                Arg {
                    name: None,
                    value: lhs,
                },
            );
            Expr::Call(callee, args.into(), ty_args, cspan)
        }
        Expr::Method(recv, name, args, ty_args, mspan) => {
            let mut args = args.to_vec();
            args.insert(
                0,
                Arg {
                    name: None,
                    value: lhs,
                },
            );
            Expr::Method(recv, name, args.into(), ty_args, mspan)
        }
        Expr::Name(..) => Expr::Call(
            Arc::new(rhs),
            Arc::from([Arg {
                name: None,
                value: lhs,
            }]),
            Vec::new(),
            span,
        ),
        other => Expr::Pipe(Arc::new(lhs), Arc::new(other), span),
    }
}

/// Syntax characters outside strings and comments, with byte offsets.
/// Shared by interpolation balancing and the format separator scanner.
fn expression_syntax(s: &str) -> impl Iterator<Item = (usize, char)> + '_ {
    let mut chars = s.char_indices();
    let mut quote = None;
    let mut escaped = false;
    let mut line_comment = false;
    let mut block_comment = false;
    let mut depth = 0i32;
    let mut format = false;
    std::iter::from_fn(move || {
        while let Some((i, c)) = chars.next() {
            if format {
                return Some((i, c));
            }
            if line_comment {
                if c == '\n' {
                    line_comment = false;
                }
                continue;
            }
            if block_comment {
                if s[i..].starts_with("--!>") {
                    for _ in 0..3 {
                        chars.next();
                    }
                    block_comment = false;
                }
                continue;
            }
            if let Some(q) = quote {
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
                continue;
            }
            if s[i..].starts_with("<!--") {
                for _ in 0..3 {
                    chars.next();
                }
                block_comment = true;
            } else if c == '#' {
                line_comment = true;
            } else if c == '\'' || c == '"' {
                quote = Some(c);
            } else {
                match c {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => depth -= 1,
                    ':' if depth == 0
                        && s.as_bytes().get(i.wrapping_sub(1)) != Some(&b':')
                        && s.as_bytes().get(i + 1) != Some(&b':') =>
                    {
                        format = true;
                    }
                    _ => {}
                }
                return Some((i, c));
            }
        }
        None
    })
}

/// Split at a top-level format colon, preserving paths, strings and comments.
fn split_format_spec(s: &str) -> (&str, Option<(&str, usize)>) {
    let mut depth = 0i32;
    for (i, c) in expression_syntax(s) {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            ':' if depth == 0
                && s.as_bytes().get(i.wrapping_sub(1)) != Some(&b':')
                && s.as_bytes().get(i + 1) != Some(&b':') =>
            {
                let spec_start = i + 1;
                return (&s[..i], Some((&s[spec_start..], spec_start)));
            }
            _ => {}
        }
    }
    (s, None)
}

/// Parse an f-string format specification (`LANGUAGE_SPEC.md` §3.6.4).
///
/// Grammar, in order:
///
/// ```text
/// spec  = [ [fill] align ] [ sign ] [ width ] [ "." precision ] [ type ]
/// align = "<" | ">" | "^"
/// sign  = "+" | "-" | " "
/// type  = "d" | "b" | "o" | "x" | "X" | "f" | "F" | "e" | "E" | "%"
/// ```
fn parse_format_spec(spec: &str, base: usize) -> Result<FormatSpec> {
    let span = Span::new(base, base + spec.len());
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0usize;
    let mut out = FormatSpec {
        span,
        ..FormatSpec::default()
    };
    // Fill and alignment: either `align`, or `fill` followed by `align`.
    if let Some(&c) = chars.get(i) {
        if let Some(a) = align_of(c) {
            out.align = Some(a);
            i += 1;
        } else if let Some(&next) = chars.get(i + 1) {
            if let Some(a) = align_of(next) {
                out.fill = Some(c);
                out.align = Some(a);
                i += 2;
            }
        }
    }
    // Sign.
    if let Some(&c) = chars.get(i) {
        match c {
            '+' => {
                out.sign = Some(Sign::Plus);
                i += 1;
            }
            '-' => {
                out.sign = Some(Sign::Minus);
                i += 1;
            }
            ' ' => {
                out.sign = Some(Sign::Space);
                i += 1;
            }
            _ => {}
        }
    }
    // Width. A leading `0` before the digits is the zero-padding flag: it
    // sets fill `0` and (for numbers) right alignment, as in Python's `06d`.
    if chars.get(i) == Some(&'0') && chars.get(i + 1).is_some_and(char::is_ascii_digit) {
        out.fill = Some('0');
        out.align.get_or_insert(Align::Right);
        i += 1;
    }
    let (w, ni) = take_digits(&chars, i);
    if let Some(w) = w {
        out.width = Some(w);
        i = ni;
    }
    // Precision.
    if chars.get(i) == Some(&'.') {
        let (p, ni) = take_digits(&chars, i + 1);
        let Some(p) = p else {
            return Err(Diag::new(
                codes::EXPECTED,
                "expected digits after `.` in f-string format specification",
                span,
            ));
        };
        out.precision = Some(p);
        i = ni;
    }
    // Type.
    if let Some(&c) = chars.get(i) {
        out.ty = Some(match c {
            'd' => FormatType::Dec,
            'b' => FormatType::Binary,
            'o' => FormatType::Octal,
            'x' => FormatType::Hex { upper: false },
            'X' => FormatType::Hex { upper: true },
            'f' | 'F' => FormatType::Fixed,
            'e' | 'E' => FormatType::Exp,
            '%' => FormatType::Percent,
            _ => {
                return Err(Diag::new(
                    codes::EXPECTED,
                    format!("unknown format type `{c}` in f-string"),
                    span,
                ))
            }
        });
        i += 1;
    }
    if i != chars.len() {
        return Err(Diag::new(
            codes::EXPECTED,
            format!("unexpected `{}` in f-string format specification", chars[i]),
            span,
        ));
    }
    Ok(out)
}

fn align_of(c: char) -> Option<Align> {
    match c {
        '<' => Some(Align::Left),
        '>' => Some(Align::Right),
        '^' => Some(Align::Center),
        _ => None,
    }
}

/// Consume a run of ASCII digits, returning the value and the next index.
fn take_digits(chars: &[char], start: usize) -> (Option<usize>, usize) {
    let mut i = start;
    let mut value = 0usize;
    let mut any = false;
    while let Some(&c) = chars.get(i) {
        let Some(d) = c.to_digit(10) else { break };
        value = value.saturating_mul(10).saturating_add(d as usize);
        any = true;
        i += 1;
    }
    (any.then_some(value), i)
}

fn infix(t: &Tok) -> Option<(u8, u8, Option<BinOp>)> {
    Some(match t {
        Tok::Pipe => (1, 2, None),
        Tok::Or => (3, 4, Some(BinOp::Or)),
        Tok::And => (5, 6, Some(BinOp::And)),
        Tok::EqEq => (7, 8, Some(BinOp::Eq)),
        Tok::Ne => (7, 8, Some(BinOp::Ne)),
        Tok::Lt => (9, 10, Some(BinOp::Lt)),
        Tok::Le => (9, 10, Some(BinOp::Le)),
        Tok::Gt => (9, 10, Some(BinOp::Gt)),
        Tok::Ge => (9, 10, Some(BinOp::Ge)),
        // Bitwise sits above comparison and below shift, so `a | b == c`
        // groups as `a | (b == c)` is avoided by keeping comparison weakest;
        // here comparison is weaker (lower binding power), so `a == b | c`
        // groups as `a == (b | c)`, matching Python's ordering.
        Tok::Bar => (11, 12, Some(BinOp::BitOr)),
        Tok::Amp => (13, 14, Some(BinOp::BitAnd)),
        Tok::Shl => (15, 16, Some(BinOp::Shl)),
        Tok::Shr => (15, 16, Some(BinOp::Shr)),
        Tok::Plus => (20, 21, Some(BinOp::Add)),
        Tok::Minus => (20, 21, Some(BinOp::Sub)),
        Tok::Star => (22, 23, Some(BinOp::Mul)),
        Tok::Slash => (22, 23, Some(BinOp::Div)),
        Tok::Percent => (22, 23, Some(BinOp::Rem)),
        // Right-associative: the left binding power is higher than the right.
        Tok::Caret => (25, 24, Some(BinOp::Pow)),
        _ => return None,
    })
}

/// Whether a token is a reserved keyword (cannot be a name).
fn is_keyword(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Let
            | Tok::Mut
            | Tok::Fn
            | Tok::If
            | Tok::Else
            | Tok::Match
            | Tok::While
            | Tok::Loop
            | Tok::For
            | Tok::In
            | Tok::Return
            | Tok::Break
            | Tok::Continue
            | Tok::Use
            | Tok::As
            | Tok::Struct
            | Tok::Enum
            | Tok::Type
            | Tok::And
            | Tok::Or
            | Tok::Not
            | Tok::Try
            | Tok::Catch
            | Tok::Finally
            | Tok::Throw
            | Tok::Pub
            | Tok::True
            | Tok::False
            | Tok::None
    )
}

fn starts_expr(t: &Tok) -> bool {
    matches!(
        t,
        Tok::Int(_)
            | Tok::Float(_)
            | Tok::Str(_)
            | Tok::FStr(_)
            | Tok::True
            | Tok::False
            | Tok::None
            | Tok::Ident(_)
            | Tok::LParen
            | Tok::LBracket
            | Tok::LBrace
            | Tok::Minus
            | Tok::Tilde
            | Tok::Not
            | Tok::If
            | Tok::Match
            | Tok::Fn
    )
}

fn span_of(e: &Expr) -> Span {
    match e {
        Expr::Lit(_, s)
        | Expr::Name(_, s)
        | Expr::FStr(_, s)
        | Expr::Unary(_, _, s)
        | Expr::Binary(_, _, _, s)
        | Expr::Call(_, _, _, s)
        | Expr::Method(_, _, _, _, s)
        | Expr::Field(_, _, s)
        | Expr::Index(_, _, s)
        | Expr::List(_, s)
        | Expr::Map(_, s)
        | Expr::Construct(_, _, _, s)
        | Expr::Tuple(_, s)
        | Expr::Lambda(_, _, s)
        | Expr::Pipe(_, _, s)
        | Expr::Range(_, _, s)
        | Expr::If(_, _, _, s)
        | Expr::Match(_, _, s)
        | Expr::Block(_, s) => *s,
        Expr::ListComp { span, .. } | Expr::MapComp { span, .. } => *span,
    }
}
