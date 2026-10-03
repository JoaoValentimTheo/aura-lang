#!/bin/sh
# B-1R2 mutation-sensitivity harness.
#
# Applies each deliberate semantic mutation to `src/run/mod.rs` one at a time,
# runs the differential oracle, records whether the oracle detected it, and
# restores the file. Every mutation must be detected; the file must end
# byte-identical to its starting state.
#
# This script MUTATES a production source file and then restores it. It never
# commits. Run from the repository root:
#
#   sh tests/oracle/mutation_experiments.sh
#
# It is intentionally not part of `cargo test`: it edits a tracked file, so it
# is a manual/CI-optional evidence generator, not a test.
#
# Exit status 0 means every mutation was detected AND the file was restored.

set -eu

REPO=$(git rev-parse --show-toplevel)
cd "$REPO"

SRC=src/run/mod.rs
BAK=$(mktemp)
cp "$SRC" "$BAK"
BEFORE=$(shasum -a 256 "$SRC" | awk '{print $1}')

# Always restore, even on failure.
trap 'cp "$BAK" "$SRC"; rm -f "$BAK"' EXIT INT TERM

pass=0
fail=0

# apply <name> <old> <new>  — performs a literal replacement via python3.
apply() {
    python3 - "$SRC" "$2" "$3" <<'PY'
import sys
path, old, new = sys.argv[1], sys.argv[2], sys.argv[3]
s = open(path).read()
if old not in s:
    sys.stderr.write("ANCHOR NOT FOUND\n")
    sys.exit(3)
open(path, "w").write(s.replace(old, new, 1))
PY
}

check() {
    name=$1
    if cargo test --locked --test evaluator_oracle oracle_corpus_matches_golden -- --exact >/tmp/oracle_mut.log 2>&1; then
        echo "MISS  $name: oracle did NOT detect the mutation"
        fail=$((fail + 1))
    else
        echo "DETECT $name"
        pass=$((pass + 1))
    fi
    cp "$BAK" "$SRC"
}

# ---- M1: reverse function-argument evaluation order -----------------------
apply "M1 arg order" \
  'let mut vals = Vec::with_capacity(args.len());
        for a in args {
            match self.eval(&a.value, env)? {
                Ctl::Val(v) => vals.push(v),
                other => return Ok(other),
            }
        }' \
  'let mut vals = vec![Value::None; args.len()];
        for (i, a) in args.iter().enumerate().rev() {
            match self.eval(&a.value, env)? {
                Ctl::Val(v) => vals[i] = v,
                other => return Ok(other),
            }
        }'
check "M1 reversal of argument evaluation order"

# ---- M2: do not short-circuit `and` ---------------------------------------
apply "M2 short-circuit" \
  'let lv = val!(self.eval(l, env));
                    if !lv.truthy() {
                        return Ok(Ctl::Val(Value::Bool(false)));
                    }
                    let rv = val!(self.eval(r, env));
                    return Ok(Ctl::Val(Value::Bool(rv.truthy())));' \
  'let lv = val!(self.eval(l, env));
                    let rv = val!(self.eval(r, env));
                    if !lv.truthy() {
                        return Ok(Ctl::Val(Value::Bool(false)));
                    }
                    return Ok(Ctl::Val(Value::Bool(rv.truthy())));'
check "M2 and evaluates RHS"

# ---- M3: wrong returned value (+1 on int addition) ------------------------
apply "M3 wrong value" \
  'Ok(Value::Int(a.checked_add(*b).ok_or_else(|| {
                        self.error(codes::OVERFLOW, "integer overflow", span)
                    })?))' \
  'Ok(Value::Int(a.checked_add(*b).ok_or_else(|| {
                        self.error(codes::OVERFLOW, "integer overflow", span)
                    })? + 1))'
check "M3 integer addition off by one"

# ---- M4: wrong diagnostic code --------------------------------------------
apply "M4 wrong code" \
  'return Err(self.error(codes::RECURSION, "call depth limit exceeded", span));' \
  'return Err(self.error(codes::OVERFLOW, "call depth limit exceeded", span));'
check "M4 E4011 -> E4013"

# ---- M5: wrong source attribution -----------------------------------------
apply "M5 source" \
  '            .get(&(Rc::as_ptr(closure) as usize))
            .copied()
        {
            self.current_source = Some(source);
        }' \
  '            .get(&(Rc::as_ptr(closure) as usize))
            .copied()
        {
            let _ = source;
        }'
check "M5 drop frame source attribution"

# ---- M6: wrong span --------------------------------------------------------
apply "M6 span" \
  'return Err(self.error(codes::DIV_ZERO, "division by zero", span));
                        }
                        a.checked_div(*b)' \
  'return Err(self.error(codes::DIV_ZERO, "division by zero", Span::default()));
                        }
                        a.checked_div(*b)'
check "M6 division span -> Span::default"

# ---- M7: captured mutation differs ----------------------------------------
apply "M7 capture" \
  'if slot.1 {
                        slot.0 = value;
                        return Ok(());
                    }
                    return Err(AssignError::Immutable);' \
  'if slot.1 {
                        if let Value::Int(i) = value {
                            slot.0 = Value::Int(i + 1);
                        } else {
                            slot.0 = value;
                        }
                        return Ok(());
                    }
                    return Err(AssignError::Immutable);'
check "M7 captured assignment writes i+1"

# ---- M8: finally precedence differs ---------------------------------------
apply "M8 finally" \
  'let fin = self.exec_block(f, env, true)?;
                    if !matches!(fin, Ctl::Val(_)) {
                        result = Ok(fin);
                    }' \
  'let _fin = self.exec_block(f, env, true)?;'
check "M8 finally no longer overrides pending"

# ---- M9: method/self binding differs --------------------------------------
apply "M9 self" \
  'let mut full = Vec::with_capacity(vals.len() + 1);
                        full.push(subject.clone());
                        full.extend(vals);' \
  'let mut full = Vec::with_capacity(vals.len() + 1);
                        full.push(Value::None);
                        full.extend(vals);'
check "M9 method receiver -> none"

# ---- M10: module initialization order differs -----------------------------
apply "M10 init order" \
  'for (item, source) in module.items.iter().zip(item_sources.iter().copied()) {
            self.current_source = Some(source);
            self.last_error_source = None;' \
  'let rev_items: Vec<_> = module.items.iter().zip(item_sources.iter().copied()).collect();
        for (item, source) in rev_items.into_iter().rev() {
            self.current_source = Some(source);
            self.last_error_source = None;'
check "M10 reverse module init order"

# ---- M11: value type erasure (proves the type-tagged observable) ----------
# Erase the runtime type of `string` values. Stdout is unaffected
# (`print("1")` and `print(1)` both render `1`); only the type-tagged value
# observable can catch this, which is exactly the blind spot the type tag
# closes.
VSRC=src/run/value.rs
VBAK=$(mktemp)
cp "$VSRC" "$VBAK"
VBEFORE=$(shasum -a 256 "$VSRC" | awk '{print $1}')
python3 - "$VSRC" <<'PY'
import sys
path = sys.argv[1]
s = open(path).read()
old = '            Value::Str(_) => "string",'
new = '            Value::Str(_) => "int",'
if old not in s:
    sys.stderr.write("VALUE ANCHOR NOT FOUND\n")
    sys.exit(3)
open(path, "w").write(s.replace(old, new, 1))
PY
if cargo test --locked --test evaluator_oracle oracle_corpus_matches_golden -- --exact >/tmp/oracle_mut11.log 2>&1; then
    echo "MISS  M11 value type erasure: oracle did NOT detect the mutation"
    fail=$((fail + 1))
else
    echo "DETECT M11 value type erasure (string reported as int)"
    pass=$((pass + 1))
fi
cp "$VBAK" "$VSRC"
rm -f "$VBAK"
VAFTER=$(shasum -a 256 "$VSRC" | awk '{print $1}')

AFTER=$(shasum -a 256 "$SRC" | awk '{print $1}')
echo
echo "detected: $pass  missed: $fail"
echo "src/run/mod.rs before=$BEFORE after=$AFTER"

if [ "$BEFORE" != "$AFTER" ]; then
    echo "RESTORATION FAILED: $SRC changed" >&2
    exit 2
fi
if [ "$VBEFORE" != "$VAFTER" ]; then
    echo "RESTORATION FAILED: $VSRC changed" >&2
    exit 2
fi
if [ "$fail" -ne 0 ]; then
    echo "MUTATION SENSITIVITY INCOMPLETE" >&2
    exit 1
fi
echo "all mutations detected and reverted"
