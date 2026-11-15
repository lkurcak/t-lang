//! A deterministic family for facts that can't all be true
//! (docs/81-contradictions-and-fn-types.md): a function of three `u8`s,
//! `x` alone and `y` with `z`, whose parameters' refinements and the
//! conditions of the branches around a call of `absurd(())`, or of a
//! function needing `z > 100`, say things of them, linear comparisons with
//! small constants. The host decides, by trying every value from 0 to 40
//! (enough, as every constant is at most 15 and a solution can be moved
//! down while it keeps its differences), whether what's known where the
//! call is can all be true. If not, the program must be accepted, and its
//! calls run under the interpreter's oracle, which stops a run that gets
//! to `absurd`; if so, it must be refused. Some programs have a
//! contradiction among `x`'s facts and none among `y`'s and `z`'s, or the
//! other way round, so a group the goal doesn't share variables with must
//! be found contradictory on its own; some join the groups with `||`.
//! They're about what programs mean, so they run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and runs it: it must print `output`.
fn accepted(src: &str, output: &str) {
    let checked = t_lang::check_source(src);
    let program = checked
        .result
        .unwrap_or_else(|e| panic!("rejected:\n{src}\n{}", e.render_in(&checked.sources)));
    let mut terminal = t_lang::interp::Captured::default();
    let result = t_lang::interp::run(&program.lower(), &mut terminal)
        .unwrap_or_else(|e| panic!("{src}\n{}", e.render_in(&checked.sources)));
    assert!(
        matches!(result, t_lang::interp::Value::Variant(_, ref v, _) if v == "Ok"),
        "{src}"
    );
    assert_eq!(String::from_utf8(terminal.out).unwrap(), output, "{src}");
}

/// Checks `src`: it's refused, with `message` in the error.
fn rejected(src: &str, message: &str) {
    let checked = t_lang::check_source(src);
    let error = checked
        .result
        .expect_err(&format!("accepted an invalid program:\n{src}"));
    assert_eq!(error.phase, Phase::Type, "{src}");
    assert!(
        error.message.contains(message),
        "{src}\n{}",
        error.render_in(&checked.sources)
    );
}

/// The three variables, by index: `x`, `y`, `z`.
const NAMES: [&str; 3] = ["x", "y", "z"];

/// The largest value tried for each variable.
const TOP: u32 = 40;

/// A comparison: `v op c`, or `v op w + c`.
#[derive(Clone, Copy)]
enum Atom {
    Const(usize, Op, u32),
    Diff(usize, Op, usize, u32),
}

#[derive(Clone, Copy)]
enum Op {
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
}

impl Op {
    fn text(self) -> &'static str {
        match self {
            Op::Lt => "<",
            Op::Gt => ">",
            Op::Le => "<=",
            Op::Ge => ">=",
            Op::Eq => "==",
            Op::Ne => "!=",
        }
    }

    fn holds(self, a: u32, b: u32) -> bool {
        match self {
            Op::Lt => a < b,
            Op::Gt => a > b,
            Op::Le => a <= b,
            Op::Ge => a >= b,
            Op::Eq => a == b,
            Op::Ne => a != b,
        }
    }
}

impl Atom {
    fn text(self) -> String {
        match self {
            Atom::Const(v, op, c) => format!("{} {} {c}", NAMES[v], op.text()),
            Atom::Diff(v, op, w, 0) => format!("{} {} {}", NAMES[v], op.text(), NAMES[w]),
            Atom::Diff(v, op, w, c) => format!("{} {} {} + {c}", NAMES[v], op.text(), NAMES[w]),
        }
    }

    fn holds(self, vals: [u32; 3]) -> bool {
        match self {
            Atom::Const(v, op, c) => op.holds(vals[v], c),
            Atom::Diff(v, op, w, c) => op.holds(vals[v], vals[w] + c),
        }
    }

    /// The variables it's of.
    fn vars(self) -> Vec<usize> {
        match self {
            Atom::Const(v, ..) => vec![v],
            Atom::Diff(v, _, w, _) => vec![v, w],
        }
    }
}

/// A condition: atoms joined by `&&`, or by `||`.
#[derive(Clone)]
struct Cond {
    atoms: Vec<Atom>,
    any: bool,
}

impl Cond {
    fn text(&self) -> String {
        let sep = if self.any { " || " } else { " && " };
        let parts: Vec<String> = self.atoms.iter().map(|a| a.text()).collect();
        parts.join(sep)
    }

    fn holds(&self, vals: [u32; 3]) -> bool {
        match self.any {
            true => self.atoms.iter().any(|a| a.holds(vals)),
            false => self.atoms.iter().all(|a| a.holds(vals)),
        }
    }
}

/// A deterministic stream of numbers (SplitMix64).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }

    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

fn op(rng: &mut Rng) -> Op {
    [Op::Lt, Op::Gt, Op::Le, Op::Ge, Op::Eq, Op::Ne][rng.below(6) as usize]
}

/// An atom of `x` alone, or of `y` and `z`.
fn atom(rng: &mut Rng, group: usize) -> Atom {
    match group {
        0 => Atom::Const(0, op(rng), rng.below(16) as u32),
        _ => match rng.below(3) {
            0 => Atom::Const(1 + rng.below(2) as usize, op(rng), rng.below(16) as u32),
            1 => Atom::Diff(1, op(rng), 2, rng.below(4) as u32),
            _ => Atom::Diff(2, op(rng), 1, rng.below(4) as u32),
        },
    }
}

/// A function: its parameters' refinements (one condition of `x`, one of
/// `y` and `z`), then the conditions it branches on, each taken by an
/// `if` around what follows, or by returning early where it's false; and
/// whether the call at the end is `absurd(())`, or `need(z)`.
struct Case {
    x: Option<Cond>,
    y: Option<Cond>,
    branches: Vec<(Cond, bool)>,
    absurd: bool,
}

fn case(rng: &mut Rng) -> Case {
    let cond = |rng: &mut Rng, group: usize, any: bool| {
        let n = 1 + rng.below(2) as usize;
        let atoms = (0..n)
            .map(|_| {
                let g = if any { rng.below(2) as usize } else { group };
                atom(rng, g)
            })
            .collect();
        Cond { atoms, any }
    };
    let x = (rng.below(3) > 0).then(|| cond(rng, 0, false));
    let y = (rng.below(3) > 0).then(|| cond(rng, 1, false));
    let branches = (0..1 + rng.below(3))
        .map(|_| {
            let any = rng.below(6) == 0;
            let group = rng.below(2) as usize;
            (cond(rng, group, any), rng.below(3) == 0)
        })
        .collect();
    Case {
        x,
        y,
        branches,
        absurd: rng.below(3) > 0,
    }
}

impl Case {
    /// The values each parameter may have, and every condition holds of.
    fn reached(&self, vals: [u32; 3]) -> bool {
        self.params(vals) && self.branches.iter().all(|(c, _)| c.holds(vals))
    }

    fn params(&self, vals: [u32; 3]) -> bool {
        self.x.as_ref().is_none_or(|c| c.holds(vals))
            && self.y.as_ref().is_none_or(|c| c.holds(vals))
    }

    /// Do the facts of variables `vars` alone hold of `vals`?
    fn group(&self, vals: [u32; 3], vars: &[usize]) -> bool {
        let only = |c: &Cond| (c.atoms.iter()).all(|a| a.vars().iter().all(|v| vars.contains(v)));
        let param = |c: &Option<Cond>| c.as_ref().is_none_or(|c| !only(c) || c.holds(vals));
        param(&self.x)
            && param(&self.y)
            && (self.branches.iter()).all(|(c, _)| c.any || !only(c) || c.holds(vals))
    }

    fn feasible(&self) -> bool {
        all_values().any(|v| self.reached(v))
    }

    /// The arguments of a call whose parameters' refinements hold, if
    /// there are any.
    fn args(&self) -> Option<[u32; 3]> {
        all_values().find(|&v| self.params(v))
    }

    fn function(&self, name: &str) -> String {
        let param = |v: usize, c: &Option<Cond>| match c {
            // A refinement of `x` alone: each atom is of `x`.
            Some(c) if c.atoms.iter().all(|a| a.vars() == [v]) => {
                format!("{0}: {{ {0}: u8 | {1} }}", NAMES[v], c.text())
            }
            _ => format!("{}: u8", NAMES[v]),
        };
        // `y`'s refinement may mention `z`, which comes after it: so it's
        // said of `z`, the last parameter, when it does.
        let (y, z) = match &self.y {
            Some(c) if c.atoms.iter().any(|a| a.vars().contains(&2)) => (
                "y: u8".to_string(),
                format!("z: {{ z: u8 | {} }}", c.text()),
            ),
            c => (param(1, c), "z: u8".to_string()),
        };
        let call = match self.absurd {
            true => "absurd(())",
            false => "need(z)",
        };
        let mut body = format!("{call};");
        for (c, early) in self.branches.iter().rev() {
            body = match early {
                true => format!("if !({}) {{\n        return;\n    }}\n    {body}", c.text()),
                false => format!("if {} {{\n        {body}\n    }}", c.text()),
            };
        }
        format!(
            "fn {name}({}, {y}, {z}) {{\n    {body}\n}}\n",
            param(0, &self.x)
        )
    }
}

fn all_values() -> impl Iterator<Item = [u32; 3]> {
    (0..=TOP).flat_map(|x| (0..=TOP).flat_map(move |y| (0..=TOP).map(move |z| [x, y, z])))
}

const HEADER: &str = "fn absurd(_u: { u: () | false }) -> ! {\n    loop {}\n}\n\n\
     fn need(_z: { z: u8 | z > 100 }) {}\n\n";

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn contradictions_prove_anything_and_only_they_do() {
    let mut rng = Rng(81);
    let mut items = String::from(HEADER);
    let mut calls = String::new();
    let (mut infeasible, mut feasible, mut split) = (0, 0, 0);
    for i in 0..400 {
        let c = case(&mut rng);
        let name = format!("k{i}");
        let f = c.function(&name);
        if c.feasible() {
            feasible += 1;
            // Every refusal is its own program, with the error it gets.
            let message = match c.absurd {
                true => "can't prove `false`",
                false => "can't prove `z > 100`",
            };
            rejected(&format!("{HEADER}{f}\nfn main() {{}}\n"), message);
        } else {
            infeasible += 1;
            // `need(z)`'s, contradictory only among `x`'s facts, which the
            // goal shares no variables with.
            if !c.absurd
                && !all_values().any(|v| c.group(v, &[0]))
                && all_values().any(|v| c.group(v, &[1, 2]))
            {
                split += 1;
            }
            items.push_str(&f);
            items.push('\n');
            if let Some([x, y, z]) = c.args() {
                calls.push_str(&format!("    {name}({x}, {y}, {z});\n"));
            }
        }
    }
    let src = format!(
        "{items}fn main(os: std::Os) -> Result<(), std::io::Error> with panic {{\n{calls}    \
         let mut out = os.stdout;\n    let n: u32 = {infeasible};\n    out.line(&n)\n}}\n"
    );
    accepted(&src, &format!("{infeasible}\n"));
    // The family has both kinds, and contradictions apart from the goal.
    assert!(
        feasible > 100 && infeasible > 50 && split > 20,
        "{feasible} {infeasible} {split}"
    );
}
