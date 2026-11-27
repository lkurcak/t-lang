//! A generated family of programs that try to break a refined place
//! through an *abstraction point*: code that sees it only as a type
//! parameter, an opaque type, an impl's parameter, a pattern's binding, an
//! aggregate's field or a drop (docs/68-fail-closed.md). Each is the
//! product of
//!
//! - a **position**: where the refined value is, which what's given lets
//!   code write (behind a `&mut`, in a `Cell`);
//! - a **trait**: what the code reaching it may do with what it's given (a
//!   marker whose `discard` writes, a consumer, a maker, an equality bound
//!   that gives it back);
//! - a **point**: how the program reaches that code (a generic function,
//!   an opaque result, an impl generic in it);
//!
//! and the pattern, aggregate and drop points on their own, each writing a
//! value that breaks the refinement (`99`) or keeps it (`3`). The oracle is
//! the interpreter's: every program is refused, or runs without "type
//! soundness violated". Retrospective 7 found holes in exactly such
//! products of features each tested on its own.
//!
//! They're about what programs mean, the same on every platform, so they run
//! on Linux alone.

use std::panic::catch_unwind;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Position {
    /// A refined list's element, given as `&mut` it.
    Mut,
    /// A refined `Cell`, given as `&` it.
    Cell,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Trait {
    /// `Discard`, of a wrapper whose `discard` writes.
    Marker,
    /// `fn go(self)`, whose impl writes.
    Consumer,
    /// `fn re(self) -> Self`, whose impl writes.
    Maker,
    /// `Give<Out = ..>`, which gives back what it holds.
    EqBinding,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Point {
    /// `fn run<G: Bound>(g: G)`.
    Generic,
    /// `fn hide(..) -> Option<impl Bound>`.
    Opaque,
    /// `impl<G: Bound> Run for Hold<G>`, through `fn start<R: Run>`.
    Impl,
}

const POSITIONS: [Position; 2] = [Position::Mut, Position::Cell];
const TRAITS: [Trait; 4] = [
    Trait::Marker,
    Trait::Consumer,
    Trait::Maker,
    Trait::EqBinding,
];
const POINTS: [Point; 3] = [Point::Generic, Point::Opaque, Point::Impl];

impl Position {
    /// The type of what's given, with `'a`.
    fn ty(self) -> &'static str {
        match self {
            Position::Mut => "&'a mut usize",
            Position::Cell => "&'a Cell<usize>",
        }
    }

    /// A statement writing `v` through `x`, of `ty()`.
    fn write(self, x: &str, v: u32) -> String {
        match self {
            Position::Mut => format!("*{x} = {v};"),
            Position::Cell => format!("{x}.set({v});"),
        }
    }

    /// The refined value's declaration, in `main`.
    fn setup(self) -> &'static str {
        match self {
            Position::Mut => "let mut l: List<{ t: usize | t < 5 }> = List::new();\n    l.push(1);",
            Position::Cell => "let c: Cell<{ t: usize | t < 5 }> = Cell::new(1);",
        }
    }

    /// What `hide` takes, and how its body reaches what's given.
    fn hide_param(self) -> &'static str {
        match self {
            Position::Mut => "l: &'a mut List<{ t: usize | t < 5 }>",
            Position::Cell => "c: &'a Cell<{ t: usize | t < 5 }>",
        }
    }

    fn hide_arg(self) -> &'static str {
        match self {
            Position::Mut => "&mut l",
            Position::Cell => "&c",
        }
    }

    /// `Some(wrap(given))` in `hide`, if there's something to give.
    fn hide_body(self, wrap: &dyn Fn(&str) -> String) -> String {
        match self {
            Position::Mut => format!(
                "if l.len() > 0 {{ Some({}) }} else {{ None }}",
                wrap("&mut l[0]")
            ),
            Position::Cell => format!("Some({})", wrap("c")),
        }
    }

    /// `body` run only where what's given can be made, in `main`, with
    /// `given` for it.
    fn guarded(self, body: &dyn Fn(&str) -> String) -> String {
        match self {
            Position::Mut => format!("if l.len() > 0 {{ {} }}", body("&mut l[0]")),
            Position::Cell => format!("{{ {} }}", body("&c")),
        }
    }

    /// Reads the refined value, which the interpreter checks.
    fn check(self) -> &'static str {
        match self {
            Position::Mut => "if l.len() > 0 { let _x: { t: usize | t < 5 } = l[0]; }",
            Position::Cell => "let _x: { t: usize | t < 5 } = c.get();",
        }
    }
}

/// Text made of text: what's given, wrapped; how code uses a value.
type Text = Box<dyn Fn(&str) -> String>;

/// The program for one product, writing `v`.
fn program(pos: Position, tr: Trait, point: Point, v: u32) -> String {
    let ty = pos.ty();
    let mut items = String::new();
    // What the trait is, and its impl for what's given (or a wrapper of
    // it); what a value of it is made of `given`; the bound; and how code
    // generic in it uses `g`.
    let (wrap, bound, uses): (Text, String, Text) = match tr {
        Trait::Marker => {
            items += "struct W<X> {\n    r: X,\n}\n\n";
            items += &format!(
                "impl<'a> Discard for W<{ty}> {{\n    fn discard(self) {{\n        {}\n    }}\n}}\n\n",
                pos.write("self.r", v)
            );
            (
                Box::new(|g: &str| format!("W {{ r: {g} }}")),
                "Discard".to_string(),
                Box::new(|g: &str| format!("let _d = {g};")),
            )
        }
        Trait::Consumer => {
            items += "trait Go {\n    fn go(self);\n}\n\n";
            items += &format!(
                "impl<'a> Go for {ty} {{\n    fn go(self) {{\n        {}\n    }}\n}}\n\n",
                pos.write("self", v)
            );
            (
                Box::new(|g: &str| g.to_string()),
                "Go".to_string(),
                Box::new(|g: &str| format!("{g}.go();")),
            )
        }
        Trait::Maker => {
            items += "trait Re {\n    fn re(self) -> Self;\n}\n\n";
            items += &format!(
                "impl<'a> Re for {ty} {{\n    fn re(self) -> Self {{\n        {}\n        self\n    }}\n}}\n\n",
                pos.write("self", v)
            );
            (
                Box::new(|g: &str| g.to_string()),
                "Re + Discard".to_string(),
                Box::new(|g: &str| format!("let _h = {g}.re();")),
            )
        }
        Trait::EqBinding => {
            items += "trait Give {\n    type Out;\n    fn give(self) -> Self::Out;\n}\n\n";
            items += "#[derive(Discard)]\nstruct Wg<X> {\n    r: X,\n}\n\n";
            items += "impl<X> Give for Wg<X> {\n    type Out = X;\n    fn give(self) -> X {\n        self.r\n    }\n}\n\n";
            let write = pos.write("x", v);
            (
                Box::new(|g: &str| format!("Wg {{ r: {g} }}")),
                format!("Give<Out = {ty}>"),
                Box::new(move |g: &str| format!("let x = {g}.give(); {write}")),
            )
        }
    };
    let main_body = match point {
        Point::Generic => {
            items += &format!("fn run<'a, G: {bound}>(g: G) {{\n    {}\n}}\n\n", uses("g"));
            pos.guarded(&|given| format!("run({});", wrap(given)))
        }
        Point::Opaque => {
            items += &format!(
                "fn hide<'a>({}) -> Option<impl {bound}> {{\n    {}\n}}\n\n",
                pos.hide_param(),
                pos.hide_body(&*wrap)
            );
            format!(
                "if let Some(g) = hide({}) {{ {} }}",
                pos.hide_arg(),
                uses("g")
            )
        }
        Point::Impl => {
            items += "trait Run {\n    fn run(self);\n}\n\n";
            items += "struct Hold<G> {\n    g: G,\n}\n\n";
            items += &format!(
                "impl<'a, G: {bound}> Run for Hold<G> {{\n    fn run(self) {{\n        let g = self.g;\n        {}\n    }}\n}}\n\n",
                uses("g")
            );
            items += "fn start<R: Run>(r: R) {\n    r.run();\n}\n\n";
            pos.guarded(&|given| format!("start(Hold {{ g: {} }});", wrap(given)))
        }
    };
    format!(
        "use alloc::list::List;\nuse core::cell::Cell;\n\n{items}fn main() with alloc {{\n    {}\n    {main_body}\n    {}\n}}\n",
        pos.setup(),
        pos.check()
    )
}

/// The programs that aren't products: a pattern's `&` and `&mut`
/// bindings, two places in one aggregate, a drop where it's made.
fn singles(v: u32) -> Vec<(String, String)> {
    vec![
        (
            "& pattern into a variant's Cell".to_string(),
            format!(
                "use core::cell::Cell;\nfn main() {{\n    let o: Option<Cell<{{ t: usize | t < 5 }}>> = Some(Cell::new(1));\n    match &o {{\n        Some(c) => c.set({v}),\n        None => {{}}\n    }}\n    if let Some(c) = &o {{ let _x: {{ t: usize | t < 5 }} = c.get(); }}\n}}\n"
            ),
        ),
        (
            "& pattern into an array's Cell".to_string(),
            format!(
                "use core::cell::Cell;\nfn main() {{\n    let a: [Cell<{{ t: usize | t < 5 }}>; 2] = [Cell::new(1), Cell::new(2)];\n    let [c, _] = &a;\n    c.set({v});\n    let _x: {{ t: usize | t < 5 }} = a[0].get();\n}}\n"
            ),
        ),
        (
            "&mut pattern into a variant".to_string(),
            format!(
                "#[derive(Copy, Discard)]\nenum E {{\n    D({{ t: usize | t < 5 }}),\n    O,\n}}\nfn main() {{\n    let mut o: Option<E> = Some(E::D(1));\n    match &mut o {{\n        Some(E::D(r)) => *r = {v},\n        _ => {{}}\n    }}\n    if let Some(E::D(x)) = o {{ let _x: {{ t: usize | t < 5 }} = x; }}\n}}\n"
            ),
        ),
        (
            "struct of two &mut places".to_string(),
            format!(
                "use alloc::list::List;\nuse core::mem::swap;\n#[derive(Discard)]\nstruct W2<'a, T> {{\n    a: &'a mut T,\n    b: &'a mut T,\n}}\nfn sw<'a, T>(w: W2<'a, T>) {{\n    swap(w.a, w.b);\n}}\nfn main() with alloc {{\n    let mut l: List<{{ t: usize | t < 5 }}> = List::new();\n    l.push(1);\n    let mut y: usize = {v};\n    if l.len() > 0 {{ sw(W2 {{ a: &mut l[0], b: &mut y }}); }}\n    if l.len() > 0 {{ let _x: {{ t: usize | t < 5 }} = l[0]; }}\n}}\n"
            ),
        ),
        (
            "array of two &mut places".to_string(),
            format!(
                "use alloc::list::List;\nuse core::mem::swap;\nfn sw<'a, T>(p: [&'a mut T; 2]) {{\n    let [a, b] = p;\n    swap(a, b);\n}}\nfn main() with alloc {{\n    let mut l: List<{{ t: usize | t < 5 }}> = List::new();\n    l.push(1);\n    let mut y: usize = {v};\n    if l.len() > 0 {{ sw([&mut l[0], &mut y]); }}\n    if l.len() > 0 {{ let _x: {{ t: usize | t < 5 }} = l[0]; }}\n}}\n"
            ),
        ),
        (
            "struct of two &Cell places".to_string(),
            format!(
                "use core::cell::Cell;\n#[derive(Copy, Discard)]\nstruct P<'a, T> {{\n    a: &'a T,\n    b: &'a T,\n}}\nfn over<'a, U: Copy>(p: P<'a, Cell<U>>) {{\n    p.a.set(p.b.get());\n}}\nfn main() {{\n    let c1: Cell<{{ t: usize | t < 5 }}> = Cell::new(1);\n    let c2: Cell<usize> = Cell::new({v});\n    over(P {{ a: &c1, b: &c2 }});\n    let _x: {{ t: usize | t < 5 }} = c1.get();\n}}\n"
            ),
        ),
        (
            "drop where it's made".to_string(),
            format!(
                "use alloc::list::List;\nstruct W<'a, T> {{\n    r: &'a mut T,\n}}\nimpl<'a> Discard for W<'a, usize> {{\n    fn discard(self) {{\n        *self.r = {v};\n    }}\n}}\nfn main() with alloc {{\n    let mut l: List<{{ t: usize | t < 5 }}> = List::new();\n    l.push(1);\n    if l.len() > 0 {{ let _w = W {{ r: &mut l[0] }}; }}\n    if l.len() > 0 {{ let _x: {{ t: usize | t < 5 }} = l[0]; }}\n}}\n"
            ),
        ),
    ]
}

/// What happened to a program: refused (with the first error), or run,
/// and whether the interpreter found it broke a refinement.
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
    for v in [99, 3] {
        for pos in POSITIONS {
            for tr in TRAITS {
                for point in POINTS {
                    let label = format!("{pos:?} × {tr:?} × {point:?}");
                    out.push((label, v, program(pos, tr, point, v)));
                }
            }
        }
        for (label, src) in singles(v) {
            out.push((label, v, src));
        }
    }
    out
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn no_abstraction_point_breaks_a_refinement() {
    let mut broke = Vec::new();
    for (label, v, src) in all() {
        if let Verdict::Broke(why) = verdict(&src) {
            broke.push(format!("{label}, writing {v}: {why}\n{src}"));
        }
    }
    assert!(broke.is_empty(), "{}", broke.join("\n\n"));
}

/// Writing what keeps the refinement, through what can be checked, is
/// accepted: the rules refuse what could break it, not everything.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn keeping_the_refinement_is_accepted_where_it_can_be_checked() {
    let checked = [
        "& pattern into a variant's Cell",
        "& pattern into an array's Cell",
        "&mut pattern into a variant",
    ];
    for (label, src) in singles(3) {
        if !checked.contains(&label.as_str()) {
            continue;
        }
        assert_eq!(verdict(&src), Verdict::Ran, "{label}\n{src}");
    }
    for (label, src) in singles(99) {
        if !checked.contains(&label.as_str()) {
            continue;
        }
        assert!(
            matches!(verdict(&src), Verdict::Refused(_)),
            "{label}, writing 99\n{src}"
        );
    }
}

/// Every product is a program the checker gets past parsing and typing
/// to the refinement it's about: a family that only made programs it
/// refused for some other reason would test nothing.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn each_program_is_about_the_refinement() {
    let mut off = Vec::new();
    for (label, v, src) in all() {
        if let Verdict::Refused(e) = verdict(&src)
            && !e.contains("can't prove `t < 5`")
        {
            off.push(format!("{label}, writing {v}: {e}\n{src}"));
        }
    }
    assert!(off.is_empty(), "{}", off.join("\n\n"));
}
