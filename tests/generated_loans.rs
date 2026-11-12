//! A generated family of programs that change a place through a *loan*:
//! each way a place can change other than by assignment, as MIR has it, a
//! `&mut` of it (docs/69-loans.md). Each is the product of
//!
//! - a **loan**: how the program lends the place and writes a value
//!   through the loan (an explicit `&mut`, a call it's lent to, a
//!   closure's capture, by `&mut`, by `move`, given to a function, nested,
//!   or a pattern's binding into a field, a variant or an array);
//! - a **site**: where the program reads the place, relative to the loan
//!   (straight after it, at the head of a loop that makes it, in what the
//!   function ensures);
//! - a **value**: one that breaks what the read needs (`0`, divided by) or
//!   keeps it (`1`).
//!
//! The oracle is the interpreter's: every program is refused, or runs
//! without "type soundness violated". And the loan forgets no more than it
//! must: what was read before it, and what's assigned after it, are known,
//! and a closure's loan keeps the measures its body keeps (its frame).
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::panic::catch_unwind;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Target {
    /// `c.n`, a field.
    Field,
    /// What's in `c.o`, a variant's field.
    Variant,
    /// `c.a[0]`, an array's element.
    Element,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Loan {
    /// `let r = &mut ..; *r = v;`
    Borrow,
    /// `set(&mut .., v);`
    Call,
    /// `let mut t = || .. = v; t();`
    Capture,
    /// `let mut t = move || .. = v; t();`
    MoveCapture,
    /// `apply(|| .. = v);`
    Given,
    /// `apply(|| apply(|| .. = v));`
    Nested,
    /// `match c { C { n, .. } => *n = v }`, and the like for a variant
    /// and an array.
    Pattern,
    /// `let C { n, .. } = c; *n = v;`
    LetPattern,
    /// `let mut t = || { c.touch(); .. = v; }; t();`: a closure whose
    /// body keeps `n()` by a call, but writes it too.
    FrameBroken,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Site {
    /// After the loan.
    Straight,
    /// At the head of a loop whose body makes the loan, after it.
    Loop,
    /// What `f` ensures.
    Ensures,
}

const TARGETS: [Target; 3] = [Target::Field, Target::Variant, Target::Element];
const LOANS: [Loan; 9] = [
    Loan::Borrow,
    Loan::Call,
    Loan::Capture,
    Loan::MoveCapture,
    Loan::Given,
    Loan::Nested,
    Loan::Pattern,
    Loan::LetPattern,
    Loan::FrameBroken,
];
const SITES: [Site; 3] = [Site::Straight, Site::Loop, Site::Ensures];

/// What every program has: `C`, whose `touch` keeps `n()`, and functions
/// to lend things to.
const ITEMS: &str = "#[derive(Copy, Discard)]
struct C {
    n: u8,
    m: u8,
    o: Option<u8>,
    a: [u8; 2],
}

impl C {
    #[measure]
    fn n(&self) -> u8 { self.n }

    fn touch(&mut self) ensures self.n() == old(self.n()) { self.m = self.m / 2; }
}

fn set(r: &mut u8, v: u8) { *r = v; }

fn apply<F: FnOnce() + Discard>(f: F) { f() }

";

impl Target {
    /// The place, as an expression of `c: &mut C`.
    fn place(self) -> &'static str {
        match self {
            Target::Field => "c.n",
            Target::Variant => "c.o",
            Target::Element => "c.a[0]",
        }
    }

    /// A statement giving it what the read needs.
    fn reset(self) -> &'static str {
        match self {
            Target::Field => "c.n = 1;",
            Target::Variant => "c.o = Some(1);",
            Target::Element => "c.a[0] = 1;",
        }
    }

    /// An expression dividing by it, a `u8`.
    fn read(self) -> &'static str {
        match self {
            Target::Field => "10 / c.n",
            Target::Variant => "match c.o { Some(k) => 10 / k, None => 0 }",
            Target::Element => "10 / c.a[0]",
        }
    }

    /// Statements writing `v` to it, through `x`, a `&mut` of the place,
    /// as a pattern binds it from `c`.
    fn pattern(self, v: u32) -> String {
        match self {
            Target::Field => format!("match c {{ C {{ n, .. }} => *n = {v} }}"),
            Target::Variant => {
                format!("match c {{ C {{ o: Some(k), .. }} => *k = {v}, _ => {{}} }}")
            }
            Target::Element => format!("match c {{ C {{ a: [k, _], .. }} => *k = {v} }}"),
        }
    }

    /// The same, by `let`.
    fn let_pattern(self, v: u32) -> String {
        match self {
            Target::Field => format!("let C {{ n, .. }} = c; *n = {v};"),
            Target::Variant => {
                format!("let C {{ o, .. }} = c; if let Some(k) = o {{ *k = {v}; }}")
            }
            Target::Element => format!("let C {{ a: [k, _], .. }} = c; *k = {v};"),
        }
    }

    /// An assignment of `v` to it.
    fn assign(self, v: u32) -> String {
        match self {
            Target::Field => format!("c.n = {v}"),
            Target::Variant => format!("c.o = Some({v})"),
            Target::Element => format!("c.a[0] = {v}"),
        }
    }
}

impl Loan {
    /// Statements lending `t`'s place and writing `v` through the loan.
    fn code(self, t: Target, v: u32) -> String {
        let assign = t.assign(v);
        match self {
            Loan::Borrow => match t {
                Target::Variant => format!("let r = &mut c.o; if let Some(k) = r {{ *k = {v}; }}"),
                _ => format!("let r = &mut {}; *r = {v};", t.place()),
            },
            Loan::Call => match t {
                Target::Variant => format!("if let Some(k) = &mut c.o {{ set(k, {v}); }}"),
                _ => format!("set(&mut {}, {v});", t.place()),
            },
            Loan::Capture => format!("let mut t = || {assign}; t();"),
            Loan::MoveCapture => format!("let mut t = move || {assign}; t();"),
            Loan::Given => format!("apply(|| {assign});"),
            Loan::Nested => format!("apply(|| apply(|| {assign}));"),
            Loan::Pattern => t.pattern(v),
            Loan::LetPattern => t.let_pattern(v),
            Loan::FrameBroken => format!("let mut t = || {{ c.touch(); {assign}; }}; t();"),
        }
    }
}

/// `f`, lending `t`'s place by `loan`, writing `v`, and reading it at
/// `site`; and `main`, calling it.
fn program(t: Target, loan: Loan, site: Site, v: u32) -> String {
    let code = loan.code(t, v);
    let (reset, read) = (t.reset(), t.read());
    let f = match site {
        Site::Straight => {
            format!("fn f(c: &mut C) -> u8 with {{}} {{\n    {reset}\n    {code}\n    {read}\n}}\n")
        }
        Site::Loop => format!(
            "fn f(c: &mut C) -> u8 with {{}} {{\n    {reset}\n    let mut i: u8 = 0;\n    while i < 2 {{\n        let _q = {read};\n        {code}\n        i = i + 1;\n    }}\n    0\n}}\n"
        ),
        Site::Ensures => format!(
            "fn f(c: &mut C) -> u8 with {{}} ensures {} {{\n    {reset}\n    {code}\n    0\n}}\n",
            match t {
                Target::Field => "c.n() == 1",
                Target::Variant => "c.o.is_some()",
                Target::Element => unreachable!("no measure names it"),
            }
        ),
    };
    format!(
        "{ITEMS}{f}\nfn main() {{\n    let mut c = C {{ n: 1, m: 1, o: Some(1), a: [1, 1] }};\n    let _x = f(&mut c);\n}}\n"
    )
}

#[derive(Debug, PartialEq)]
enum Verdict {
    Refused(String),
    Ran,
    Broke(String),
}

fn verdict(src: &str) -> Verdict {
    let checked = t_lang::check_source(src);
    let program = match checked.result {
        Ok(p) => p,
        Err(e) => return Verdict::Refused(e.render_in(&checked.sources)),
    };
    let lowered = program.lower();
    let run = catch_unwind(|| {
        let mut terminal = t_lang::interp::Captured::default();
        let _ = t_lang::interp::run(&lowered, &mut terminal);
    });
    match run {
        Ok(()) => Verdict::Ran,
        Err(panic) => {
            let why = (panic.downcast_ref::<String>().cloned())
                .or_else(|| panic.downcast_ref::<&str>().map(|s| s.to_string()))
                .unwrap_or_default();
            Verdict::Broke(why)
        }
    }
}

fn all() -> Vec<(String, u32, String)> {
    let mut out = Vec::new();
    for v in [0, 1] {
        for t in TARGETS {
            for loan in LOANS {
                for site in SITES {
                    // `ensures` names a `&mut`'s target only by measures,
                    // and an array's element has none.
                    if (t, site) == (Target::Element, Site::Ensures) {
                        continue;
                    }
                    let label = format!("{t:?} × {loan:?} × {site:?}");
                    out.push((label, v, program(t, loan, site, v)));
                }
            }
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn no_loan_breaks_what_is_known() {
    let mut broke = Vec::new();
    for (label, v, src) in all() {
        if let Verdict::Broke(why) = verdict(&src) {
            broke.push(format!("{label}, writing {v}: {why}\n{src}"));
        }
    }
    assert!(broke.is_empty(), "{}", broke.join("\n\n"));
}

/// Every program writing `0` is refused for what the read needs: a family
/// that only made programs refused for some other reason would test
/// nothing.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn each_breaking_program_is_refused_for_the_read() {
    let mut off = Vec::new();
    for (label, v, src) in all() {
        if v != 0 {
            continue;
        }
        match verdict(&src) {
            Verdict::Refused(e) if e.contains("may panic") || e.contains("ensures") => {}
            other => off.push(format!("{label}: {other:?}\n{src}")),
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}

/// The loan forgets the place where it's made, and nothing more: what was
/// read of it before, and what's assigned to it after the loan is gone,
/// are known.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn before_and_after_the_loan_are_known() {
    let mut off = Vec::new();
    for t in [Target::Field, Target::Variant] {
        for loan in LOANS {
            let code = loan.code(t, 0);
            let (reset, read) = (t.reset(), t.read());
            let before = format!(
                "fn f(c: &mut C) -> u8 with {{}} {{\n    {reset}\n    let b = {read};\n    {code}\n    b\n}}\n"
            );
            let after = format!(
                "fn f(c: &mut C) -> u8 with {{}} {{\n    {code}\n    {reset}\n    {read}\n}}\n"
            );
            for (which, f) in [("before", before), ("after", after)] {
                let src = format!(
                    "{ITEMS}{f}\nfn main() {{\n    let mut c = C {{ n: 1, m: 1, o: Some(1), a: [1, 1] }};\n    let _x = f(&mut c);\n}}\n"
                );
                let got = verdict(&src);
                if got != Verdict::Ran {
                    off.push(format!("{t:?} × {loan:?}, {which}: {got:?}\n{src}"));
                }
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}

/// A closure whose only change to what it captures is a call keeping
/// `n()` keeps it, at each site, made once or in a loop, called or given
/// to a function, nested.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn a_closure_keeps_what_its_body_keeps() {
    let loans = [
        "let mut t = || c.touch(); t();",
        "let mut t = move || c.touch(); t(); t();",
        "apply(|| c.touch());",
        "apply(|| apply(|| c.touch()));",
        "apply(|| { c.touch(); let mut u = || c.touch(); u(); });",
    ];
    let mut off = Vec::new();
    for code in loans {
        for site in SITES {
            let f = match site {
                Site::Straight => format!(
                    "fn f(c: &mut C) -> u8 with {{}} {{\n    c.n = 1;\n    {code}\n    10 / c.n\n}}\n"
                ),
                Site::Loop => format!(
                    "fn f(c: &mut C) -> u8 with {{}} {{\n    c.n = 1;\n    let mut i: u8 = 0;\n    while i < 2 {{\n        let _q = 10 / c.n;\n        {code}\n        i = i + 1;\n    }}\n    0\n}}\n"
                ),
                Site::Ensures => format!(
                    "fn f(c: &mut C) -> u8 with {{}} ensures c.n() == old(c.n()) {{\n    {code}\n    0\n}}\n"
                ),
            };
            let src = format!(
                "{ITEMS}{f}\nfn main() {{\n    let mut c = C {{ n: 1, m: 1, o: Some(1), a: [1, 1] }};\n    let _x = f(&mut c);\n}}\n"
            );
            let got = verdict(&src);
            if got != Verdict::Ran {
                off.push(format!("`{code}` × {site:?}: {got:?}\n{src}"));
            }
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}
