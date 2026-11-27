//! A deterministic family for the order operands are worked out in
//! (docs/91-evaluation-order.md): an operand that reads a variable, in each
//! shape one can take — the variable, a field of a tuple or a struct, what's
//! behind a `&mut` — followed, in each place operands are — a call's
//! arguments, a method's receiver and arguments, an operator's sides, a
//! comparison's, a tuple's, a struct's, an array's and a variant's fields, a
//! closure's and a function value's arguments — by one that changes the
//! variable, by assigning in a block, through a `&mut` given to a call, in
//! a call's own argument, or in a `match`. The first operand is what the
//! variable was. Each program checks that as it runs, by a function that
//! panics if it isn't; where the checker follows the value, it's also given
//! to a function whose parameter's refinement says it, which must be
//! accepted, and to one that says it's what the variable is afterwards,
//! which must be refused. They're about what programs mean, so they run on
//! Linux alone.

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
        Err(_) => Err("accepted, but the interpreter found it unsound".to_string()),
    }
}

/// How the first operand reads the variable: what declares it as 5, the
/// operand, and the statement that makes it 7.
struct Form {
    name: &'static str,
    declare: &'static str,
    read: &'static str,
    write: &'static str,
    /// What a `&mut` to it is written as.
    lend: &'static str,
}

const FORMS: [Form; 4] = [
    Form {
        name: "variable",
        declare: "let mut v = 5u8;",
        read: "v",
        write: "v = 7;",
        lend: "&mut v",
    },
    Form {
        name: "tuple field",
        declare: "let mut t = (5u8, 1u8);",
        read: "t.0",
        write: "t = (7, 1);",
        lend: "&mut t.0",
    },
    Form {
        name: "struct field",
        declare: "let mut p = P { a: 5, b: 1 };",
        read: "p.a",
        write: "p.a = 7;",
        lend: "&mut p.a",
    },
    Form {
        name: "behind a `&mut`",
        declare: "let mut under = 5u8;\n    let r = &mut under;",
        read: "*r",
        write: "*r = 7;",
        lend: "&mut *r",
    },
];

/// What the later operand is, giving `k`, for a variable written by
/// `write` and lent as `lend`.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Writer {
    /// `{ v = 7; k }`
    Block,
    /// `set(&mut v, k)`
    Lent,
    /// `second(0, { v = 7; k })`
    Nested,
    /// `match true { true => { v = 7; k }, false => k }`
    Match,
}

const WRITERS: [Writer; 4] = [Writer::Block, Writer::Lent, Writer::Nested, Writer::Match];

fn writer(w: Writer, f: &Form, k: &str) -> String {
    let (write, lend) = (f.write, f.lend);
    match w {
        Writer::Block => format!("{{ {write} {k} }}"),
        Writer::Lent => format!("set({lend}, {k})"),
        Writer::Nested => format!("second(0, {{ {write} {k} }})"),
        Writer::Match => format!("match true {{ true => {{ {write} {k} }}, false => {k} }}"),
    }
}

/// Where the operands are: an expression of the first, `a`, and the later
/// one, `b`, whose value is what `a` was, 5, when `b` gives `k`; and
/// whether the checker follows that value to a refinement.
struct Context {
    name: &'static str,
    k: &'static str,
    expr: fn(&str, &str) -> String,
    followed: bool,
}

const CONTEXTS: [Context; 12] = [
    Context {
        name: "a call's arguments",
        k: "0u8",
        expr: |a, b| format!("first({a}, {b})"),
        followed: true,
    },
    Context {
        name: "a method's receiver",
        k: "0u8",
        expr: |a, b| format!("M {{ m: {a} }}.pick({b})"),
        followed: false,
    },
    Context {
        name: "an operator's sides",
        k: "0u8",
        expr: |a, b| format!("{a} + {b}"),
        followed: true,
    },
    Context {
        name: "a comparison's sides",
        k: "6u8",
        expr: |a, b| format!("five_if({a} < {b})"),
        followed: false,
    },
    Context {
        name: "an equality's sides",
        k: "5u8",
        expr: |a, b| format!("five_if({a} == {b})"),
        followed: false,
    },
    Context {
        name: "a tuple's fields",
        k: "0u8",
        expr: |a, b| format!("({a}, {b}).0"),
        followed: false,
    },
    Context {
        name: "a struct's fields",
        k: "0u8",
        expr: |a, b| format!("P {{ a: {a}, b: {b} }}.a"),
        followed: false,
    },
    Context {
        name: "an array's items",
        k: "0u8",
        expr: |a, b| format!("first_of([{a}, {b}])"),
        followed: false,
    },
    Context {
        name: "a closure's arguments",
        k: "0u8",
        expr: |a, b| format!("(|x: u8, _y: u8| x)({a}, {b})"),
        followed: false,
    },
    Context {
        name: "a function value's arguments",
        k: "0u8",
        expr: |a, b| format!("{{ let g: fn(u8, u8) -> u8 = plain; g({a}, {b}) }}"),
        followed: false,
    },
    // A chain's conditions, each worked out after the ones before
    // (docs/100-patterns.md).
    Context {
        name: "a chain's `let`s",
        k: "0u8",
        expr: |a, b| {
            format!("if let Some(x) = Some({a}) && let Some(_y) = Some({b}) {{ x }} else {{ 0 }}")
        },
        followed: false,
    },
    Context {
        name: "a chain's `let` and `bool`",
        k: "0u8",
        expr: |a, b| format!("if let Some(x) = Some({a}) && {b} < 200 {{ x }} else {{ 0 }}"),
        followed: false,
    },
];

/// What `need` asks of the value, if anything.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Need {
    /// Nothing: only the run checks it.
    Run,
    /// That it's what the variable was, which is so.
    Was,
    /// That it's what the variable is afterwards, which isn't.
    Is,
}

fn program(f: &Form, c: &Context, w: Writer, need: Need) -> String {
    let later = writer(w, f, c.k);
    let expr = (c.expr)(f.read, &later);
    let declare = f.declare;
    let use_it = match need {
        Need::Run => "same(got, 5);",
        Need::Was => "same(was(got), 5);",
        Need::Is => "same(is(got), 5);",
    };
    format!(
        "#[derive(Copy, Discard)]
struct P {{ a: u8, b: u8 }}

#[derive(Copy, Discard)]
struct M {{ m: u8 }}

impl M {{
    fn pick(self, _b: u8) -> u8 {{ self.m }}
}}

fn first(a: u8, _b: u8) -> {{ r: u8 | r == a }} {{ a }}

fn plain(a: u8, _b: u8) -> u8 {{ a }}

fn second(_a: u8, b: u8) -> {{ r: u8 | r == b }} {{ b }}

fn set(c: &mut u8, k: u8) -> {{ r: u8 | r == k }} ensures *c == 7 {{
    *c = 7;
    k
}}

fn five_if(b: bool) -> u8 {{
    match b {{
        true => 5,
        false => 0,
    }}
}}

fn first_of(a: [u8; 2]) -> u8 {{ a[0] }}

fn was(x: {{ x: u8 | x == 5 }}) -> u8 {{ x }}

fn is(x: {{ x: u8 | x == 7 }}) -> u8 {{ x }}

fn same(a: u8, b: u8) with panic {{
    if a != b {{
        panic(\"an operand was read late\");
    }}
}}

fn main() with panic {{
    {declare}
    let got = {expr};
    {use_it}
}}
"
    )
}

/// Every form, in every context, before every writer.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn operands_are_read_in_order() {
    let mut failures = Vec::new();
    let mut count = 0;
    let mut followed = 0;
    for f in &FORMS {
        for c in &CONTEXTS {
            for w in WRITERS {
                let what = format!("{} in {}, then {w:?}", f.name, c.name);
                count += 1;
                let src = program(f, c, w, Need::Run);
                if let Err(m) = verdict(&src) {
                    failures.push(format!("{what}: {m}\n{src}"));
                }
                // The checker forgets what's behind a `&mut` it has just
                // made of a variable, so only the run says what's read.
                if !c.followed || f.read == "*r" {
                    continue;
                }
                followed += 1;
                let src = program(f, c, w, Need::Was);
                if let Err(m) = verdict(&src) {
                    failures.push(format!("{what}, as it was: {m}\n{src}"));
                }
                let src = program(f, c, w, Need::Is);
                match verdict(&src) {
                    Err(m) if m.contains("can't prove `x == 7`") => {}
                    got => failures.push(format!("{what}, as it is: {got:?}\n{src}")),
                }
            }
        }
    }
    assert!(count >= 160, "only {count} programs");
    assert!(followed >= 24, "only {followed} followed");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

/// A receiver taken by value is worked out before the arguments, and what's
/// called before what it's called with.
#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn receivers_and_callees_come_first() {
    let src = "#[derive(Copy, Discard)]
struct M { m: u8 }

impl M {
    fn pick(self, _b: u8) -> u8 { self.m }
}

fn one(_x: u8) -> u8 { 1 }

fn two(_x: u8) -> u8 { 2 }

fn same(a: u8, b: u8) with panic {
    if a != b {
        panic(\"read late\");
    }
}

fn main() with panic {
    let mut r = M { m: 5 };
    same(r.pick({ r = M { m: 7 }; 0 }), 5);
    let mut f: fn(u8) -> u8 = one;
    same(f({ f = two; 0 }), 1);
    let mut c = 5u8;
    c += { c = 7; 1 };
    same(c, 8);
}
";
    assert_eq!(verdict(src), Ok(()));
}
