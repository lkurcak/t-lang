//! Deterministic families for what's known where branches meet
//! (docs/61-proof-search.md, lemmas J1 and J2): each branch sets a value
//! and says which branch it was, `k`; after the branches meet, the value
//! is used where it must be small, under a condition on `k`. The value's
//! knowledge is in each place it can be: an array's elements (what's
//! said of positions), an `Option`'s field (an enum's parts), a tuple's
//! field, and a tuple in an `Option` (copied down from the enum to its
//! field). The branches meet as an `if`/`else`, a `match` of four arms,
//! nested `if`s one to three deep, and `if`s one after another. Where the
//! condition names only branches whose value is small, it's accepted and
//! run by the interpreter, which checks the refinement at the call;
//! naming one whose value isn't, it's refused. They're about what
//! programs mean, so they run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and, if it's accepted, runs it. Gives the error's message
/// if it's refused, or what went wrong running it.
fn verdict(src: &str) -> Result<(), String> {
    let checked = t_lang::check_source(src);
    let program = match checked.result {
        Ok(p) => p,
        Err(e) => {
            assert_eq!(
                e.phase,
                Phase::Type,
                "{src}\n{}",
                e.render_in(&checked.sources)
            );
            return Err(e.message);
        }
    };
    let run = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut terminal = t_lang::interp::Captured::default();
        t_lang::interp::run(&program.lower(), &mut terminal)
    }));
    match run {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(format!(
            "accepted, but it failed: {}",
            e.render_in(&checked.sources)
        )),
        Err(_) => Err("accepted, but the interpreter found a broken refinement".to_string()),
    }
}

/// Where what's known of the value is.
#[derive(Clone, Copy, Debug)]
enum Carrier {
    /// `[u8; 2]`: at its elements' position.
    Array,
    /// `Option<u8>`: in the enum's parts.
    Opt,
    /// `(u8, u8)`: in the tuple's first field.
    Pair,
    /// `Option<(u8, u8)>`: said of the enum and copied to its field.
    OptPair,
}

impl Carrier {
    /// The refined type `small` takes, the plain one, a value whose small
    /// part is `v`, and how `small` reads the small part of `l`.
    fn parts(self, v: u8) -> (&'static str, &'static str, String, &'static str) {
        match self {
            Carrier::Array => (
                "[{ x: u8 | x < 5 }; 2]",
                "[u8; 2]",
                format!("[{v}u8, {v}u8]"),
                "l[0]",
            ),
            Carrier::Opt => (
                "Option<{ x: u8 | x < 5 }>",
                "Option<u8>",
                format!("Some({v}u8)"),
                "match l {\n        Some(x) => x,\n        None => 0,\n    }",
            ),
            Carrier::Pair => (
                "({ x: u8 | x < 5 }, u8)",
                "(u8, u8)",
                format!("({v}u8, 9u8)"),
                "l.0",
            ),
            Carrier::OptPair => (
                "Option<({ x: u8 | x < 5 }, u8)>",
                "Option<(u8, u8)>",
                format!("Some(({v}u8, 9u8))"),
                "match l {\n        Some((x, _)) => x,\n        None => 0,\n    }",
            ),
        }
    }
}

/// How the branches meet; each has `branches()` of them, numbered from 0.
#[derive(Clone, Copy, Debug)]
enum Shape {
    IfElse,
    Match4,
    /// `if`s inside each other, this deep; branch 0 is none taken.
    Nested(usize),
    /// `if`s one after another, this many; branch `i` is the last taken.
    Sequence(usize),
}

impl Shape {
    fn branches(self) -> usize {
        match self {
            Shape::IfElse => 2,
            Shape::Match4 => 4,
            Shape::Nested(d) | Shape::Sequence(d) => d + 1,
        }
    }

    /// The body that sets `l` and `k`, branch `i` to `vals[i]` and `i`.
    fn body(self, vals: &[String]) -> String {
        let set = |i: usize, ind: &str| format!("{ind}l = {};\n{ind}k = {i};\n", vals[i]);
        match self {
            Shape::IfElse => format!(
                "    if n % 2 == 0 {{\n{}    }} else {{\n{}    }}\n",
                set(0, "        "),
                set(1, "        ")
            ),
            Shape::Match4 => {
                let mut s = String::from("    match n {\n");
                for i in 0..4 {
                    let pat = if i == 3 {
                        "_".to_string()
                    } else {
                        i.to_string()
                    };
                    s += &format!(
                        "        {pat} => {{\n{}        }}\n",
                        set(i, "            ")
                    );
                }
                s + "    }\n"
            }
            Shape::Nested(d) => {
                let mut s = set(0, "    ");
                let mut ind = String::from("    ");
                for i in 1..=d {
                    s += &format!("{ind}if n > {} {{\n", i - 1);
                    ind += "    ";
                    s += &set(i, &ind);
                }
                for _ in 1..=d {
                    ind.truncate(ind.len() - 4);
                    s += &format!("{ind}}}\n");
                }
                s
            }
            Shape::Sequence(d) => {
                let mut s = set(0, "    ");
                for i in 1..=d {
                    s += &format!(
                        "    if n % {} == 0 {{\n{}    }}\n",
                        i + 1,
                        set(i, "        ")
                    );
                }
                s
            }
        }
    }
}

/// The program: `f(n)` sets the value as `shape` says, branch `i`'s small
/// part `vals[i]`, and gives it to `small` where `k` is one of `claim`;
/// `main` calls it with every `n` that takes a branch.
fn program(carrier: Carrier, shape: Shape, vals: &[u8], claim: &[usize]) -> String {
    let (refined, plain, _, read) = carrier.parts(0);
    let values: Vec<String> = vals.iter().map(|&v| carrier.parts(v).2).collect();
    let cond: Vec<String> = claim.iter().map(|i| format!("k == {i}")).collect();
    format!(
        "fn small(l: {refined}) -> u8 {{
    {read}
}}

fn f(n: usize) -> u8 {{
    let mut l: {plain} = {init};
    let mut k: u8 = 9;
    let _ = l;
    let _ = k;
{body}    if {cond} {{ small(l) }} else {{ 0 }}
}}

fn main() {{
    for n in 0..13 {{
        let _ = f(n);
    }}
}}
",
        init = carrier.parts(9).2,
        body = shape.body(&values),
        cond = cond.join(" || "),
    )
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn branches_meet_with_what_each_knew() {
    let carriers = [
        Carrier::Array,
        Carrier::Opt,
        Carrier::Pair,
        Carrier::OptPair,
    ];
    let shapes = [
        Shape::IfElse,
        Shape::Match4,
        Shape::Nested(1),
        Shape::Nested(2),
        Shape::Nested(3),
        Shape::Sequence(1),
        Shape::Sequence(2),
        Shape::Sequence(3),
    ];
    let mut wrong = Vec::new();
    let mut count = 0;
    for carrier in carriers {
        for shape in shapes {
            let b = shape.branches();
            // Which branches' values are small: the even ones, or the odd.
            for parity in [0, 1] {
                let vals: Vec<u8> = (0..b)
                    .map(|i| match i % 2 == parity {
                        true => 1 + (i % 4) as u8,
                        false => 7 + (i % 3) as u8,
                    })
                    .collect();
                let small: Vec<usize> = (0..b).filter(|i| vals[*i] < 5).collect();
                let big: Vec<usize> = (0..b).filter(|i| vals[*i] >= 5).collect();
                // The small ones: accepted, and runs. With a big one too:
                // refused.
                let mut claims = vec![(small.clone(), true)];
                if let Some(&x) = big.last() {
                    let mut c = small.clone();
                    c.push(x);
                    claims.push((c, false));
                }
                for (claim, ok) in claims {
                    if claim.is_empty() {
                        continue;
                    }
                    count += 1;
                    let src = program(carrier, shape, &vals, &claim);
                    let got = verdict(&src);
                    let right = match (&got, ok) {
                        (Ok(()), true) => true,
                        (Err(m), false) => m.contains("can't prove"),
                        _ => false,
                    };
                    if !right {
                        wrong.push(format!(
                            "{carrier:?} {shape:?} values {vals:?} claim {claim:?}: \
                             expected {}, got {got:?}\n{src}",
                            if ok { "accepted" } else { "refused" }
                        ));
                    }
                }
            }
        }
    }
    assert!(count >= 100, "only {count} programs");
    assert!(
        wrong.is_empty(),
        "{} of {count} wrong:\n{}",
        wrong.len(),
        wrong.join("\n")
    );
}
