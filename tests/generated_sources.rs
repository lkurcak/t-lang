//! Where generic code gets values of a type parameter
//! (docs/83-sources.md): a family of signatures that give a function, an
//! adapter or a closure another way to get a value the checker might count
//! on being one it was given, each tried with a value that breaks what's
//! known (an iterator of `100..101` where one of `0..8` was given, a `100`
//! where an item below 8 was): every one is refused or runs with the
//! interpreter's oracle quiet. Beside them, the ways that only give back
//! what was given, which must still be accepted.
//!
//! The channels: what a closure gives (its result, by value, in an
//! `Option` or a tuple), what it can write back through a parameter
//! (behind a `&mut`, in an `Option` or a tuple behind one, in a `Cell`,
//! behind two `&mut`s), another argument of the type, a bound that fixes
//! what an iterator's items are, an impl of `DoubleEndedIterator` with
//! bounds of its own, a second closure giving the same type, a closure
//! given to the callee that makes the one it keeps; and a closure the
//! callee keeps, returned, put in an `Option`, given to another closure,
//! or held by an adapter whose module can get it out.
//!
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone.

use std::panic::catch_unwind;

const R: &str = "core::ops::Range<usize>";
const CELL: &str = "core::cell::Cell";

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

/// `template` with each `@K@` replaced by its value, and then `$R` and
/// `$CELL` by the types they name.
fn fill(template: &str, with: &[(&str, &str)]) -> String {
    let mut s = template.to_string();
    for (k, v) in with {
        s = s.replace(&format!("@{k}@"), v);
    }
    s.replace("$R", R).replace("$CELL", CELL)
}

/// A function generic in iterator `X` that gives back an `X`, with one
/// more way in: `(bound, param, body, argument)`.
const MAKER: &[(&str, &str, &str, &str, &str)] = &[
    (
        "x_ret",
        "F: FnOnce() -> X",
        "f: F",
        "let _ = x; f()",
        "|| 100..101",
    ),
    (
        "x_ret_opt",
        "F: FnOnce() -> Option<X>",
        "f: F",
        "match f() { Some(y) => { let _ = x; y } None => x }",
        "|| Some(100..101)",
    ),
    (
        "x_ret_tuple",
        "F: FnOnce() -> (X, u8)",
        "f: F",
        "let _ = x; f().0",
        "|| (100..101, 0)",
    ),
    (
        "x_mut",
        "F: FnOnce(&mut X)",
        "f: F",
        "let mut y = x; f(&mut y); y",
        "|it: &mut $R| *it = 100..101",
    ),
    (
        "x_mut_opt",
        "F: FnOnce(&mut Option<X>)",
        "f: F",
        "let mut o: Option<X> = None; f(&mut o); match o { Some(y) => { let _ = x; y } None => x }",
        "|o: &mut Option<$R>| *o = Some(100..101)",
    ),
    (
        "x_mut_tuple",
        "F: FnOnce(&mut (X, u8))",
        "f: F",
        "let mut t = (x, 0u8); f(&mut t); t.0",
        "|t: &mut ($R, u8)| t.0 = 100..101",
    ),
    (
        "x_cell",
        "F: FnOnce(&$CELL<Option<X>>)",
        "f: F",
        "let c = $CELL::new(None); f(&c); match c.into_inner() { Some(y) => { let _ = x; y } None => x }",
        "|c: &$CELL<Option<$R>>| { let _ = c.replace(Some(100..101)); }",
    ),
    (
        "x_mut_mut",
        "F: FnOnce(&mut &mut X)",
        "f: F",
        "let mut y = x; { let mut r = &mut y; f(&mut r); } y",
        "|r: &mut &mut $R| **r = 100..101",
    ),
    (
        "x_fnmut_ret",
        "F: FnMut(X) -> X + Discard",
        "f: F",
        "let mut f = f; f(x)",
        "|_y: $R| 100..101",
    ),
    (
        "x_param_opt",
        "",
        "o: Option<X>",
        "match o { Some(y) => { let _ = x; y } None => x }",
        "Some(100..101)",
    ),
    (
        "x_param_mut",
        "",
        "r: &mut X",
        "core::mem::replace(r, x)",
        "&mut (100..101)",
    ),
    ("x_two", "", "y: X", "let _ = x; y", "100..101"),
    ("x_fn_param", "", "g: fn() -> X", "let _ = x; g()", "far"),
    (
        "x_bound_make",
        "+ Make",
        "_u: ()",
        "let _ = x; X::make()",
        "()",
    ),
];

fn maker(bound: &str, param: &str, body: &str, arg: &str) -> String {
    let b = match bound {
        "" => String::new(),
        more if more.starts_with('+') => format!(" {more}"),
        other => format!(", {other}"),
    };
    fill(
        "fn mk<X: Iterator + Discard@B@>(x: X, @P@) -> X where X::Item: Discard {
    @BODY@
}

trait Make {
    fn make() -> Self;
}

impl Make for $R {
    fn make() -> $R { 100..101 }
}

fn far() -> $R { 100..101 }

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in mk(0..8, @ARG@) {
        let _ = a[r];
    }
}
",
        &[("B", &b), ("P", param), ("BODY", body), ("ARG", arg)],
    )
}

/// The same, lent `&mut X`: `(bound, param, body, argument)`.
const LENT: &[(&str, &str, &str, &str, &str)] = &[
    (
        "x_mut",
        "F: FnOnce(&mut X)",
        "f: F",
        "f(x)",
        "|it: &mut $R| *it = 100..101",
    ),
    (
        "x_mut_opt",
        "F: FnOnce(&mut Option<X>)",
        "f: F",
        "let mut o: Option<X> = None; f(&mut o); match o { Some(y) => { let _ = core::mem::replace(x, y); } None => {} }",
        "|o: &mut Option<$R>| *o = Some(100..101)",
    ),
    (
        "x_ret",
        "F: FnOnce() -> X",
        "f: F",
        "let _ = core::mem::replace(x, f());",
        "|| 100..101",
    ),
];

fn lent(bound: &str, param: &str, body: &str, arg: &str) -> String {
    fill(
        "fn mk<X: Iterator + Discard, @B@>(x: &mut X, @P@) where X::Item: Discard {
    @BODY@
}

fn main() {
    let a: [u8; 8] = [0; 8];
    let mut it = 0..8;
    mk(&mut it, @ARG@);
    for r in it {
        let _ = a[r];
    }
}
",
        &[("B", bound), ("P", param), ("BODY", body), ("ARG", arg)],
    )
}

/// An adapter over iterator `I` giving `I::Item`, with a closure that
/// has another way to them: `(closure bound, next's body, closure)`.
const ADAPTER: &[(&str, &str, &str, &str)] = &[
    (
        "item_mut",
        "FnMut(&mut I::Item)",
        "match self.iter.next() { Some(x) => { let mut v = x; (self.f)(&mut v); Some(v) } None => None }",
        "|x: &mut usize| *x = 100",
    ),
    (
        "item_mut_opt",
        "FnMut(&mut Option<I::Item>)",
        "let mut o = self.iter.next(); (self.f)(&mut o); o",
        "|o: &mut Option<usize>| *o = Some(100)",
    ),
    (
        "item_cell",
        "FnMut(&$CELL<Option<I::Item>>)",
        "let c = $CELL::new(self.iter.next()); (self.f)(&c); c.into_inner()",
        "|c: &$CELL<Option<usize>>| { let _ = c.replace(Some(100)); }",
    ),
    (
        "item_ret",
        "FnMut() -> I::Item",
        "match self.iter.next() { Some(_x) => Some((self.f)()), None => None }",
        "|| 100",
    ),
    (
        "item_ret_opt",
        "FnMut() -> Option<I::Item>",
        "match self.iter.next() { Some(_x) => (self.f)(), None => None }",
        "|| Some(100)",
    ),
    (
        "item_feedback",
        "FnMut(I::Item) -> I::Item",
        "match self.iter.next() { Some(x) => Some((self.f)(x)), None => None }",
        "|_x: usize| 100",
    ),
    (
        "iter_mut",
        "FnMut(&mut I)",
        "(self.f)(&mut self.iter); self.iter.next()",
        "|it: &mut $R| *it = 100..101",
    ),
    (
        "iter_mut_opt",
        "FnMut(&mut Option<I>)",
        "let mut o: Option<I> = None; (self.f)(&mut o); match o { Some(i) => { let _old = core::mem::replace(&mut self.iter, i); } None => {} } self.iter.next()",
        "|o: &mut Option<$R>| *o = Some(100..101)",
    ),
    (
        "iter_ret",
        "FnMut() -> I",
        "let _old = core::mem::replace(&mut self.iter, (self.f)()); self.iter.next()",
        "|| 100..101",
    ),
];

/// The adapter, with the closure written in the call or (`var`) made
/// before it.
fn adapter(name: &str, fb: &str, body: &str, arg: &str, var: bool) -> String {
    let discard = matches!(name, "item_mut_opt" | "iter_mut_opt" | "iter_ret");
    let i = if discard {
        "I: Iterator + Discard"
    } else {
        "I: Iterator"
    };
    let main = if var {
        "let f = @ARG@;\n    for r in ad(0..8, f) {"
    } else {
        "for r in ad(0..8, @ARG@) {"
    };
    fill(
        &format!(
            "#[derive(Discard)]
struct Ad<I, F> {{ iter: I, f: F }}

impl<@I@, F: @FB@> Iterator for Ad<I, F> where I::Item: Discard {{
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {{
        @BODY@
    }}
}}

fn ad<@I@, F: @FB@>(iter: I, f: F) -> Ad<I, F> where I::Item: Discard {{ Ad {{ iter, f }} }}

fn main() {{
    let a: [u8; 8] = [0; 8];
    {main}
        let _ = a[r];
    }}
}}
"
        ),
        &[("I", i), ("FB", fb), ("BODY", body), ("ARG", arg)],
    )
}

/// What `map`'s closure gives, through a maker with another way to the
/// closure: `(bound, param, body, call)`.
const MAP_CLOSURE: &[(&str, &str, &str, &str, &str)] = &[
    (
        "g_ret",
        "H: FnOnce() -> G",
        "h: H",
        "let _ = g; it.map(h())",
        "mk(0..8, g, || b)",
    ),
    (
        "g_mut",
        "H: FnOnce(&mut G)",
        "h: H",
        "let mut g = g; h(&mut g); it.map(g)",
        "mk(0..8, g, |q: &mut fn(usize) -> usize| *q = b)",
    ),
    (
        "g_opt",
        "",
        "o: Option<G>",
        "match o { Some(q) => { let _ = g; it.map(q) } None => it.map(g) }",
        "mk(0..8, g, Some(b))",
    ),
    (
        "g_two",
        "",
        "q: G",
        "let _ = g; it.map(q)",
        "mk(0..8, g, b)",
    ),
];

fn map_closure(bound: &str, param: &str, body: &str, call: &str) -> String {
    let b = if bound.is_empty() {
        String::new()
    } else {
        format!(", {bound}")
    };
    fill(
        "fn mk<I: Iterator, U, G: FnMut(I::Item) -> U + Discard@B@>(it: I, g: G, @P@) -> core::iter::Map<I, G> {
    @BODY@
}

fn half(x: usize) -> { r: usize | r < 8 } { x % 8 }

fn bad(_x: usize) -> usize { 100 }

fn main() {
    let a: [u8; 8] = [0; 8];
    let g: fn(usize) -> usize = half;
    let b: fn(usize) -> usize = bad;
    for r in @CALL@ {
        let _ = a[r];
    }
}
",
        &[("B", &b), ("P", param), ("BODY", body), ("CALL", call)],
    )
}

/// An adapter giving a closure's `U`, with a second closure that gives
/// one too: `(bounds, next's body, the second closure)`.
const SECOND: &[(&str, &str, &str, &str)] = &[
    (
        "u_two_fg",
        "F: FnMut(I::Item) -> U, G: FnMut() -> U",
        "match self.iter.next() { Some(x) => { let _u = (self.f)(x); Some((self.g)()) } None => None }",
        "|| 100",
    ),
    (
        "u_two_gf",
        "G: FnMut() -> U, F: FnMut(I::Item) -> U",
        "match self.iter.next() { Some(x) => { let _u = (self.f)(x); Some((self.g)()) } None => None }",
        "|| 100",
    ),
    (
        "u_mut",
        "F: FnMut(I::Item) -> U, G: FnMut(&mut U)",
        "match self.iter.next() { Some(x) => { let mut u = (self.f)(x); (self.g)(&mut u); Some(u) } None => None }",
        "|u: &mut usize| *u = 100",
    ),
    (
        "u_feedback",
        "F: FnMut(I::Item) -> U, G: FnMut(U) -> U",
        "match self.iter.next() { Some(x) => { let u = (self.f)(x); Some((self.g)(u)) } None => None }",
        "|_u: usize| 100",
    ),
    (
        "u_ret_opt",
        "F: FnMut(I::Item) -> U, G: FnMut() -> Option<U>",
        "match self.iter.next() { Some(x) => { let _u = (self.f)(x); (self.g)() } None => None }",
        "|| Some(100)",
    ),
];

fn second(fb: &str, body: &str, garg: &str, var: bool) -> String {
    let main = if var {
        "let g = @G@;\n    for r in ad(0..100, |x: usize| x % 8, g) {"
    } else {
        "for r in ad(0..100, |x: usize| x % 8, @G@) {"
    };
    fill(
        &format!(
            "#[derive(Discard)]
struct Ad<I, F, G> {{ iter: I, f: F, g: G }}

impl<I: Iterator, U, @FB@> Iterator for Ad<I, F, G> where U: Discard {{
    type Item = U;
    effect e = I::e;

    fn next(&mut self) -> Option<U> with I::e {{
        @BODY@
    }}
}}

fn ad<I: Iterator, U, @FB@>(iter: I, f: F, g: G) -> Ad<I, F, G> where U: Discard {{ Ad {{ iter, f, g }} }}

fn main() {{
    let a: [u8; 8] = [0; 8];
    {main}
        let _ = a[r];
    }}
}}
"
        ),
        &[("FB", fb), ("BODY", body), ("G", garg)],
    )
}

const NEED8: &str = "fn need8(_r: { r: usize | r < 8 }) {}\n\n";

/// A closure the callee calls with what an iterator gives, and so is
/// walked knowing it, where the callee has another way to an item, or
/// keeps the closure to be called with anything later.
fn closures() -> Vec<(&'static str, String)> {
    let mut out = Vec::new();
    let mut add = |name: &'static str, body: &str| out.push((name, format!("{NEED8}{body}")));
    add(
        "eq_usize",
        "fn each<X: Iterator<Item = usize> + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) {
    let _ = it;
    let mut f = f;
    f(100);
}

fn main() {
    each(0..8, |x: usize| need8(x));
}
",
    );
    add(
        "eq_param",
        "fn each<T, X: Iterator<Item = T> + Discard, F: FnMut(X::Item) + Discard>(it: X, t: T, f: F) {
    let _ = it;
    let mut f = f;
    f(t);
}

fn main() {
    each(0..8, 100, |x: usize| need8(x));
}
",
    );
    add(
        "item_param",
        "fn each<X: Iterator + Discard, F: FnMut(X::Item) + Discard>(it: X, extra: X::Item, f: F) {
    let _ = it;
    let mut f = f;
    f(extra);
}

fn main() {
    each(0..8, 100, |x: usize| need8(x));
}
",
    );
    add(
        "kept_ret",
        "fn keep<X: Iterator + Discard, F: FnMut(X::Item)>(it: X, f: F) -> F {
    let _ = it;
    f
}

fn main() {
    let mut f = keep(0..8, |x: usize| need8(x));
    f(100);
}
",
    );
    add(
        "kept_after_call",
        "fn keep<X: Iterator + Discard, F: FnMut(X::Item)>(mut it: X, mut f: F) -> F where X::Item: Discard {
    match it.next() {
        Some(x) => f(x),
        None => {}
    }
    let _ = it;
    f
}

fn main() {
    let mut f = keep(0..8, |x: usize| need8(x));
    f(100);
}
",
    );
    add(
        "kept_opt",
        "fn keep<X: Iterator + Discard, F: FnMut(X::Item)>(it: X, f: F) -> Option<F> {
    let _ = it;
    Some(f)
}

fn main() {
    match keep(0..8, |x: usize| need8(x)) {
        Some(mut f) => f(100),
        None => {}
    }
}
",
    );
    add(
        "kept_slot",
        "fn keep<X: Iterator + Discard, F: FnMut(X::Item), H: FnOnce(F)>(it: X, f: F, h: H) {
    let _ = it;
    h(f)
}

fn main() {
    let mut slot = None;
    keep(0..8, |x: usize| need8(x), |f| slot = Some(f));
    match slot {
        Some(mut f) => f(100),
        None => {}
    }
}
",
    );
    add(
        "kept_into_f",
        "#[derive(Discard)]
struct Each<I, F> { iter: I, f: F }

impl<I: Iterator, F: FnMut(I::Item) -> bool> Iterator for Each<I, F> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}

impl<I, F> Each<I, F> {
    fn into_f(self) -> F where I: Discard { self.f }
}

fn each<I: Iterator, F: FnMut(I::Item) -> bool>(iter: I, f: F) -> Each<I, F> where I::Item: Discard {
    Each { iter, f }
}

fn main() {
    let e = each(0..8, |x: usize| { need8(x); true });
    let mut f = e.into_f();
    let _ = f(100);
}
",
    );
    // An adapter in a module of its own, with a way in from its module.
    let adapter = "#[derive(Discard)]
    pub struct Each<I, F> { iter: I, f: F }

    impl<I: Iterator, F: FnMut(I::Item)> Iterator for Each<I, F> where I::Item: Discard {
        type Item = I::Item;
        effect e = I::e;

        fn next(&mut self) -> Option<I::Item> with I::e {
            match self.iter.next() {
                Some(x) => {
                    (self.f)(x);
                    None
                }
                None => None,
            }
        }
    }

    pub fn each<I: Iterator, F: FnMut(I::Item)>(iter: I, f: F) -> Each<I, F> where I::Item: Discard {
        Each { iter, f }
    }
";
    add(
        "kept_same_module",
        &format!(
            "{}
fn main() {{
    let mut e = each(0..8, |x: usize| need8(x));
    (e.f)(100);
    let _ = e;
}}
",
            adapter.replace("\n    ", "\n").replace("pub ", "")
        ),
    );
    add(
        "kept_poke",
        &format!(
            "mod m {{
    {adapter}
    pub fn poke<I: Iterator<Item = usize>, F: FnMut(usize)>(e: &mut Each<I, F>) {{
        (e.f)(100);
    }}
}}

fn main() {{
    let mut e = m::each(0..8, |x: usize| need8(x));
    m::poke(&mut e);
    let _ = e;
}}
"
        ),
    );
    add(
        "kept_swap_iter",
        &format!(
            "mod m {{
    {adapter}
    pub fn swap_iter<I: Iterator + Discard, F>(e: &mut Each<I, F>, it: I) {{
        e.iter = it;
    }}
}}

fn main() {{
    let mut e = m::each(0..8, |x: usize| need8(x));
    m::swap_iter(&mut e, 100..101);
    for _ in e {{}}
}}
"
        ),
    );
    add(
        "kept_wrapper",
        &format!(
            "mod m {{
    {adapter}
    #[derive(Discard)]
    pub struct W<I, F> {{ pub e: Each<I, F> }}

    pub fn poke<I: Iterator<Item = usize>, F: FnMut(usize)>(w: &mut W<I, F>) {{
        (w.e.f)(100);
    }}
}}

fn main() {{
    let mut w = m::W {{ e: m::each(0..8, |x: usize| need8(x)) }};
    m::poke(&mut w);
    let _ = w;
}}
"
        ),
    );
    add(
        "kept_pub_field",
        &format!(
            "mod m {{
    {}
}}

fn main() {{
    let mut e = m::each(0..8, |x: usize| need8(x));
    (e.f)(100);
    let _ = e;
}}
",
            adapter.replace("{ iter: I, f: F }", "{ iter: I, pub f: F }")
        ),
    );
    add(
        "kept_opaque",
        "fn keep<X: Iterator + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) -> impl FnMut(X::Item) + Discard {
    let _ = it;
    let mut f = f;
    move |v| f(v)
}

fn main() {
    let mut g = keep(0..8, |x: usize| need8(x));
    g(100);
}
",
    );
    add(
        "kept_sealed_eq",
        "mod m {
    #[derive(Discard)]
    pub struct Each<I, F> { iter: I, f: F }

    impl<I: Iterator<Item = usize>, F: FnMut(usize)> Iterator for Each<I, F> {
        type Item = usize;
        effect e = I::e;

        fn next(&mut self) -> Option<usize> with I::e {
            match self.iter.next() {
                Some(_x) => {
                    (self.f)(100);
                    None
                }
                None => None,
            }
        }
    }

    pub fn each<I: Iterator, F: FnMut(I::Item)>(iter: I, f: F) -> Each<I, F> { Each { iter, f } }
}

fn main() {
    for _ in m::each(0..8, |x: usize| need8(x)) {}
}
",
    );
    add(
        "w7_slot",
        "fn h<T, F: FnMut(&mut T), H: FnOnce(F)>(_x: &mut T, f: F, g: H) {
    g(f)
}

fn main() {
    let mut x: { v: usize | v < 8 } = 5;
    let mut slot = None;
    h(&mut x, |r: &mut usize| need8(*r), |f| slot = Some(f));
    match slot {
        Some(mut f) => {
            let mut y = 100usize;
            f(&mut y)
        }
        None => {}
    }
}
",
    );
    add(
        "w7_slot_param",
        "fn h<T, F: Fn(&T) + Discard>(x: &T, f: F, slot: &mut Option<F>) {
    f(x);
    *slot = Some(f);
}

fn main() {
    let x: { v: usize | v < 8 } = 5;
    let mut slot = None;
    h(&x, |r: &usize| need8(*r), &mut slot);
    match slot {
        Some(f) => f(&100),
        None => {}
    }
}
",
    );
    add(
        "w7_ret",
        "fn h<T, F: Fn(&T)>(x: &T, f: F) -> F {
    f(x);
    f
}

fn main() {
    let x: { v: usize | v < 8 } = 5;
    let f = h(&x, |r: &usize| need8(*r));
    f(&100);
}
",
    );
    out
}

/// Iterators whose `DoubleEndedIterator` impl has bounds of its own.
fn back_impls() -> Vec<(&'static str, String)> {
    vec![
        (
            "adapter_dei_eq",
            "#[derive(Discard)]
struct Nine<I> { iter: I }

impl<I: Iterator> Iterator for Nine<I> {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e { self.iter.next() }
}

impl<I: DoubleEndedIterator<Item = usize>> DoubleEndedIterator for Nine<I> {
    fn next_back(&mut self) -> Option<usize> with I::e {
        match self.iter.next_back() {
            Some(_x) => Some(100),
            None => None,
        }
    }
}

fn nine<I: Iterator>(iter: I) -> Nine<I> { Nine { iter } }

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in nine(0..8).rev() {
        let _ = a[r];
    }
}
"
            .to_string(),
        ),
        (
            "carried_dei_make",
            "trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 100 }
}

#[derive(Discard)]
struct Once<T> { v: Option<T> }

impl<T: Discard> Iterator for Once<T> {
    type Item = T;
    effect e = {};

    fn next(&mut self) -> Option<T> { core::mem::replace(&mut self.v, None) }
}

impl<T: Discard + Make> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> {
        let _ = core::mem::replace(&mut self.v, None);
        Some(T::make())
    }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    let it = Once { v: Some(3usize) };
    for r in it.rev() {
        let _ = a[r];
        break;
    }
}
"
            .to_string(),
        ),
    ]
}

/// Every program that breaks what's known: `(family, name, program)`.
fn breaking() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, bound, param, body, arg) in MAKER {
        out.push((format!("maker {name}"), maker(bound, param, body, arg)));
    }
    for (name, bound, param, body, arg) in LENT {
        out.push((format!("lent {name}"), lent(bound, param, body, arg)));
    }
    for (name, fb, body, arg) in ADAPTER {
        for var in [false, true] {
            let label = format!("adapter {name}{}", if var { " var" } else { "" });
            out.push((label, adapter(name, fb, body, arg, var)));
        }
    }
    for (name, bound, param, body, call) in MAP_CLOSURE {
        out.push((
            format!("map closure {name}"),
            map_closure(bound, param, body, call),
        ));
    }
    for (name, fb, body, garg) in SECOND {
        for var in [false, true] {
            let label = format!("second {name}{}", if var { " var" } else { "" });
            out.push((label, second(fb, body, garg, var)));
        }
    }
    for (name, src) in closures() {
        out.push((format!("closure {name}"), src));
    }
    for (name, src) in back_impls() {
        out.push((format!("back {name}"), src));
    }
    out
}

/// What only gives back what it was given: each must be accepted.
fn keeping() -> Vec<(&'static str, String)> {
    vec![
        (
            "maker reads it",
            maker("F: FnOnce(&X) + Discard", "f: F", "f(&x); x", "|_it: &$R| {}"),
        ),
        (
            "maker given two",
            fill(
                "fn mk<X: Iterator + Discard>(x: X, y: X, first: bool) -> X {
    if first { let _ = y; x } else { let _ = x; y }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in mk(0..8, 3..5, false) {
        let _ = a[r];
    }
}
",
                &[],
            ),
        ),
        (
            "adapter reads items",
            adapter(
                "item_ref",
                "FnMut(&I::Item)",
                "match self.iter.next() { Some(x) => { (self.f)(&x); Some(x) } None => None }",
                "|_x: &usize| {}",
                false,
            ),
        ),
        (
            "core map closure",
            "fn main() {
    let a: [u8; 4] = [1, 2, 3, 4];
    let b: [u8; 4] = [0; 4];
    for v in (0..4).rev().map(|x| a[x]).skip(2) {
        let _ = b[v as usize % 4];
    }
}
"
            .to_string(),
        ),
        (
            "core filter closure",
            format!(
                "{NEED8}fn main() {{
    for v in (0..8).filter(|x| {{ need8(*x); *x > 2 }}) {{
        need8(v);
    }}
}}
"
            ),
        ),
        (
            "sealed adapter of a module",
            format!(
                "{NEED8}mod m {{
    #[derive(Discard)]
    pub struct Each<I, F> {{ iter: I, f: F }}

    impl<I: Iterator, F: FnMut(I::Item) -> usize> Iterator for Each<I, F> {{
        type Item = usize;
        effect e = I::e;

        fn next(&mut self) -> Option<usize> with I::e {{
            match self.iter.next() {{
                Some(x) => Some((self.f)(x)),
                None => None,
            }}
        }}
    }}

    pub fn each<I: Iterator, F: FnMut(I::Item) -> usize>(iter: I, f: F) -> Each<I, F> {{
        Each {{ iter, f }}
    }}
}}

fn main() {{
    for _ in m::each(0..8, |x: usize| {{ need8(x); x }}) {{}}
}}
"
            ),
        ),
        (
            "closure not kept",
            format!(
                "{NEED8}fn each<X: Iterator + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) {{
    let mut f = f;
    for x in it {{
        f(x);
    }}
}}

fn main() {{
    each(0..8, |x: usize| need8(x));
}}
"
            ),
        ),
        (
            "closure given an item fixed",
            "fn each<X: Iterator<Item = usize> + Discard, F: FnMut(X::Item) + Discard>(it: X, f: F) {
    let _ = it;
    let mut f = f;
    f(3);
}

fn main() {
    each(0..8, |_x: usize| {});
}
"
            .to_string(),
        ),
        (
            "carried items",
            "#[derive(Discard)]
struct Once<T> { v: Option<T> }

impl<T: Discard> Iterator for Once<T> {
    type Item = T;
    effect e = {};

    fn next(&mut self) -> Option<T> { core::mem::replace(&mut self.v, None) }
}

impl<T: Discard> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> { core::mem::replace(&mut self.v, None) }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    for r in (Once { v: Some(3usize) }).rev() {
        let _ = a[r];
    }
}
"
            .to_string(),
        ),
    ]
}

/// No program given another way to a value breaks a type at run time:
/// each is refused, or runs with the oracle quiet.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn every_other_source_is_refused() {
    let mut off = Vec::new();
    // To look at them: `T_DUMP_SOURCES=dir cargo test --test
    // generated_sources`.
    if let Ok(dir) = std::env::var("T_DUMP_SOURCES") {
        let all = breaking().into_iter().map(|(l, s)| (format!("bad {l}"), s));
        let good = keeping().into_iter().map(|(l, s)| (format!("ok {l}"), s));
        for (label, src) in all.chain(good) {
            let name = label.replace(' ', "_");
            std::fs::write(format!("{dir}/{name}.t"), &src).unwrap();
        }
    }
    for (label, src) in breaking() {
        if let Verdict::Broke(why) = verdict(&src) {
            off.push(format!("{label}: broke: {why}\n{src}"));
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}

/// What gives back only what it was given still counts.
#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn what_was_given_is_kept() {
    let mut off = Vec::new();
    for (label, src) in keeping() {
        match verdict(&src) {
            Verdict::Ran => {}
            Verdict::Broke(why) => off.push(format!("{label}: broke: {why}\n{src}")),
            Verdict::Refused(e) => off.push(format!("{label}: refused: {e}\n{src}")),
        }
    }
    assert!(off.is_empty(), "{} off:\n\n{}", off.len(), off.join("\n\n"));
}
