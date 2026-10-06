#![allow(dead_code)]
#![allow(clippy::format_push_string)]
//! Deterministic, dependency-free Aura program generator.
//!
//! Shared by the property tests (`tests/property_hardening.rs`) and the fuzz
//! targets (`fuzz/fuzz_targets/*.rs`) via `#[path]` inclusion, so a single
//! generator definition is exercised by both. It introduces **no** dependency:
//! it is plain source, included where needed.
//!
//! ## Guarantee
//!
//! Every generated program parses and **passes the checker**. The generator
//! tracks the type of every binding it introduces and only emits calls,
//! operations, and field/index accesses that the conservative checker accepts,
//! so a generated program exercises the *runtime*, not the checker's rejection
//! paths. (If a generated program were rejected, that is a generator defect and
//! the property/fuzz harness treats it as a failure — it is never silently
//! discarded.)
//!
//! Generated programs deliberately include deliberate runtime diagnostics
//! (out-of-range index, division by zero) because a structured `E4xxx` is a
//! *success* for the runtime invariant; only a panic/abort/timeout is a
//! failure.

/// A tiny deterministic PRNG (splitmix64). Reproducible from a single `u64`
/// seed, which is exactly what `proptest` shrinks and what a fuzz target derives
/// from its input bytes.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        // Avoid the all-zero state.
        Rng(seed ^ 0x9E37_79B9_7F4A_7C15)
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A uniform value in `0..n` (`n > 0`).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        (self.next_u64() % (n as u64)) as usize
    }

    pub fn pick<'a, T>(&mut self, xs: &'a [T]) -> &'a T {
        &xs[self.below(xs.len())]
    }

    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }
}

/// A binding tracked by the generator: its Aura name and its checker type.
#[derive(Clone)]
enum Var {
    Int(String),
    Str(String),
    Bool(String),
    /// A `[int]` list that is `let mut` (so `push` is allowed).
    MutIntList(String),
    /// An untyped `let mut []` that accepts any element (used for cycles).
    AnyList(String),
    Point(String),
    Color(String),
}

/// The fixed, always-valid preamble every generated program shares. It defines
/// a struct + method, enums, a trait + impl, a generic function, and plain
/// functions, matching the frozen OOP/generics surface.
const PREAMBLE: &str = "\
struct Point { x: int, y: int }\n\
impl Point { fn sum(self) -> int { return self.x + self.y } }\n\
enum Color { Red, Green, Blue }\n\
trait Show { fn show(self) -> int }\n\
struct Num { n: int }\n\
impl Show for Num { fn show(self) -> int { return self.n } }\n\
fn id<T>(x: T) -> T { return x }\n\
fn add(a: int, b: int) -> int { return a + b }\n\
fn describe(c: Color) -> int { return match c { Red() -> 0\n Green() -> 1\n Blue() -> 2 } }\n\
fn on_show<T: Show>(x: T) -> int { return x.show() }\n";

/// Maximum statements per generated `main` body (keeps programs bounded and
/// deterministic in cost).
pub const MAX_STMTS: usize = 10;

/// Generate a complete, checker-passing Aura program from `seed`.
#[must_use]
pub fn generate(seed: u64) -> String {
    let mut rng = Rng::new(seed);
    let mut vars: Vec<Var> = Vec::new();
    let mut body = String::new();
    let n = 2 + rng.below(MAX_STMTS);
    for _ in 0..n {
        gen_stmt(&mut rng, &mut vars, &mut body, 0);
    }
    // Always end with a deterministic observable, so two substrates must agree.
    body.push_str("    print(0)\n");
    // Keystone unused analysis: every generated local is emitted as an
    // explicit discard (`_name`). A `_`-prefixed binding is an ordinary,
    // fully readable binding that the unused analysis does not track, so the
    // generator's contract — every program checks — holds by construction
    // rather than by a post-pass that would need to reason about scopes.
    format!("{PREAMBLE}fn main() {{\n{body}}}\n")
}

fn int_expr(rng: &mut Rng, vars: &[Var]) -> String {
    let mut opts: Vec<String> = vec![
        rng.below(20).to_string(),
        "len([1, 2, 3])".into(),
        "abs(-3)".into(),
        "to_int(\"7\")".into(),
        "on_show(Num { n: 4 })".into(),
        format!("add({}, 1)", rng.below(9)),
    ];
    for v in vars {
        match v {
            Var::Int(n) => opts.push(n.clone()),
            Var::MutIntList(n) | Var::AnyList(n) => opts.push(format!("len({n})")),
            Var::Point(n) => opts.push(format!("{n}.sum()")),
            Var::Color(n) => opts.push(format!("describe({n})")),
            Var::Str(n) => opts.push(format!("len({n})")),
            Var::Bool(_) => {}
        }
    }
    let a = opts[rng.below(opts.len())].clone();
    if rng.chance(40) {
        let op = *rng.pick(&["+", "-", "*", "%", "/"]);
        // A divisor of zero is a *runtime* `E4007`, which is a permitted
        // outcome, but avoiding it keeps generated programs exercising the
        // value model rather than a single fatal diagnostic. `1..9` never
        // divides by zero.
        let b = (1 + rng.below(8)).to_string();
        format!("({a} {op} {b})")
    } else {
        a
    }
}

fn str_expr(rng: &mut Rng, vars: &[Var]) -> String {
    let mut opts: Vec<String> = vec![
        "\"a\"".into(),
        "\"abc\".upper()".into(),
        "\" x \".trim()".into(),
    ];
    for v in vars {
        if let Var::Str(n) = v {
            opts.push(n.clone());
        }
        if let Var::Int(n) = v {
            opts.push(format!("to_string({n})"));
        }
    }
    let a = opts[rng.below(opts.len())].clone();
    if rng.chance(30) {
        format!("({a} + \"z\")")
    } else {
        a
    }
}

fn bool_expr(rng: &mut Rng, vars: &[Var]) -> String {
    let mut opts: Vec<String> = vec!["true".into(), "false".into(), "1 < 2".into()];
    for v in vars {
        match v {
            Var::Int(n) => opts.push(format!("{n} < 10")),
            Var::Str(n) => opts.push(format!("{n} == \"a\"")),
            Var::Bool(n) => opts.push(n.clone()),
            Var::MutIntList(n) | Var::AnyList(n) => {
                opts.push(format!("len({n}) > 0"));
                opts.push(format!("{n}.contains(1)"));
            }
            _ => {}
        }
    }
    let a = opts[rng.below(opts.len())].clone();
    if rng.chance(20) {
        let b = "true".to_string();
        format!("({a} and {b})")
    } else {
        a
    }
}

fn gen_stmt(rng: &mut Rng, vars: &mut Vec<Var>, out: &mut String, depth: usize) {
    let choice = rng.below(14);
    match choice {
        0 => {
            let name = format!("_i{}", vars.len());
            let e = int_expr(rng, vars);
            out.push_str(&format!("    let mut {name} = {e}\n"));
            vars.push(Var::Int(name));
        }
        1 => {
            let name = format!("_s{}", vars.len());
            let e = str_expr(rng, vars);
            out.push_str(&format!("    let {name} = {e}\n"));
            vars.push(Var::Str(name));
        }
        2 => {
            let name = format!("_b{}", vars.len());
            let e = bool_expr(rng, vars);
            out.push_str(&format!("    let {name} = {e}\n"));
            vars.push(Var::Bool(name));
        }
        3 => {
            let name = format!("_l{}", vars.len());
            let n = 1 + rng.below(4);
            let mut items = Vec::new();
            for _ in 0..n {
                items.push(rng.below(50).to_string());
            }
            out.push_str(&format!("    let mut {name} = [{}]\n", items.join(", ")));
            vars.push(Var::MutIntList(name));
        }
        4 => {
            // A cycle seed: an untyped mutable empty list accepts any element.
            let name = format!("_c{}", vars.len());
            out.push_str(&format!("    let mut {name} = []\n"));
            vars.push(Var::AnyList(name));
        }
        5 => {
            let name = format!("_p{}", vars.len());
            let x = rng.below(50).to_string();
            let y = rng.below(50).to_string();
            out.push_str(&format!("    let {name} = Point {{ x: {x}, y: {y} }}\n"));
            vars.push(Var::Point(name));
        }
        6 => {
            let name = format!("_k{}", vars.len());
            let c = *rng.pick(&["Red()", "Green()", "Blue()"]);
            out.push_str(&format!("    let {name} = {c}\n"));
            vars.push(Var::Color(name));
        }
        7 => {
            let e = int_expr(rng, vars);
            out.push_str(&format!("    print({e})\n"));
        }
        8 => {
            // Aliasing / cycle construction, then a cycle-safe observation.
            // The cases below deliberately build **shared subgraphs with
            // fan-out >= 2** (`push(c, c)` twice, `[c, c]`, `x = [x, x]`), the
            // exact shape AUDIT-4 showed depth-only cycle detection does not
            // bound: cycle detection != work-bounded traversal. Ordinary
            // single-cycle cases alone do not exercise combinatorial
            // re-traversal, so they are not sufficient coverage here.
            if let Some(Var::AnyList(n)) =
                vars.iter().find(|v| matches!(v, Var::AnyList(_))).cloned()
            {
                match rng.below(6) {
                    // Single self-cycle (terminates trivially).
                    0 => out.push_str(&format!("    push({n}, {n})\n")),
                    1 => out.push_str(&format!("    push({n}, {})\n", rng.below(9))),
                    // Fan-out 2: the cycle is reachable from two positions.
                    2 => {
                        out.push_str(&format!("    push({n}, {n})\n"));
                        out.push_str(&format!("    push({n}, {n})\n"));
                        out.push_str(&format!("    print(len(to_string({n})))\n"));
                    }
                    3 => {
                        out.push_str(&format!("    push({n}, {n})\n"));
                        out.push_str(&format!("    push({n}, {n})\n"));
                        out.push_str(&format!("    print(len(json_encode({n})))\n"));
                    }
                    // A shared DAG: an inner reference duplicated in an outer
                    // list, so the same subgraph is traversed twice.
                    4 => {
                        out.push_str(&format!("    push({n}, {n})\n"));
                        let m = format!("_m{}", vars.len());
                        out.push_str(&format!("    let {m} = [{n}, {n}]\n"));
                        out.push_str(&format!("    print(len(to_string({m})))\n"));
                        out.push_str(&format!("    print(len(json_encode({m})))\n"));
                    }
                    // Deep fan-out amplification: `x = [x, x]` repeated.
                    _ => {
                        let g = format!("_g{}", vars.len());
                        out.push_str(&format!("    let mut {g} = {n}\n"));
                        let reps = 1 + rng.below(4);
                        for _ in 0..reps {
                            out.push_str(&format!("    {g} = [{g}, {g}]\n"));
                        }
                        out.push_str(&format!("    print(len(to_string({g})))\n"));
                        out.push_str(&format!("    print(len(json_encode({g})))\n"));
                        out.push_str(&format!("    print({n} == {n})\n"));
                    }
                }
            } else {
                out.push_str("    print(1)\n");
            }
        }
        9 => {
            if let Some(Var::MutIntList(n)) = vars
                .iter()
                .find(|v| matches!(v, Var::MutIntList(_)))
                .cloned()
            {
                out.push_str(&format!("    push({n}, {})\n", rng.below(99)));
                out.push_str(&format!("    print(len({n}))\n"));
            } else {
                out.push_str("    print(2)\n");
            }
        }
        10 => {
            if depth < 3 && rng.chance(60) {
                let cond = bool_expr(rng, vars);
                out.push_str(&format!("    if {cond} {{\n"));
                // Bindings introduced inside a block are scoped to it, so the
                // inner generation works on a copy and its additions are
                // discarded — a later statement must never reference a name
                // that only exists inside the branch.
                let mut inner_vars = vars.clone();
                let inner = 1 + rng.below(2);
                for _ in 0..inner {
                    gen_stmt(rng, &mut inner_vars, out, depth + 1);
                }
                out.push_str("    } else {\n        print(9)\n    }\n");
            } else {
                out.push_str("    print(3)\n");
            }
        }
        _ => {
            if let Some(Var::MutIntList(n)) = vars
                .iter()
                .find(|v| matches!(v, Var::MutIntList(_)))
                .cloned()
            {
                out.push_str(&format!("    for x in {n} {{ print(x) }}\n"));
            } else {
                out.push_str("    for x in 0..3 { print(x) }\n");
            }
        }
    }
}

/// The canonical seed corpus for the generated-program layer: a fixed set of
/// seeds whose programs are committed as permanent corpus fixtures.
///
/// The first twelve seeds (`0..12`) are the original spread chosen so the
/// corpus covers each statement kind at least once. The remaining seeds are
/// **curated for the shared-subgraph / fan-out surface** AUDIT-4 exposed:
/// seeds whose generated program builds a cycle with reference fan-out >= 2
/// (`push(c, c)` twice, `x = [x, x]`, or a duplicated inner reference) and
/// observes it through display, JSON, and equality. A generator that only ever
/// produced simple single cycles was what let AUDIT-4's combinatorial
/// re-traversal escape the earlier fuzzing round, so those shapes are pinned
/// in the committed corpus rather than left to chance.
#[must_use]
pub fn seed_corpus() -> Vec<(u64, String)> {
    const SEEDS: &[u64] = &[
        // Original statement-kind spread.
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11,
        // Curated cycle / shared-fan-out coverage (AUDIT-4 surface).
        14, 105, 123, 161, 264, 265, 398,
    ];
    SEEDS.iter().map(|&s| (s, generate(s))).collect()
}
