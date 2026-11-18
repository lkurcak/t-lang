//! Deterministic families for values in predicates
//! (docs/85-values-in-predicates.md): functions that say which value they
//! leave behind a `&mut`, or give back (`*a == old(*b)`, `r == old(*a)`),
//! of structs, lists, enums, boxes, tuples and type parameters, with
//! bodies that keep their word, break it, or change the value after; and
//! callers that lend them a local, a field, what a parameter points to, or
//! what a box owns, and then count on what was known of each value. Each
//! is refused, or accepted and run by the interpreter, which checks every
//! postcondition, comparing the values whole, and every refinement. They're
//! about what programs mean, so they run on Linux alone.

use t_lang::error::Phase;

/// Checks `src` and, if it's accepted, runs it: it must finish, having
/// broken no refinement. Gives the error's message if it's refused.
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
        Ok(Ok(t_lang::interp::Value::Variant(_, v, _))) if v == "Ok" => Ok(()),
        Ok(Ok(v)) => panic!("{src}\nfinished with {v:?}"),
        Ok(Err(e)) => panic!("{src}\n{}", e.render_in(&checked.sources)),
        Err(_) => Err("the interpreter found a broken refinement".to_string()),
    }
}

/// A type of values to move around, with how to make the two values
/// (`1` and `2`), what's known of each, and how to change one in place.
struct Ty {
    name: &'static str,
    items: &'static str,
    ty: &'static str,
    make: [&'static str; 2],
    /// Something known of a value at `{x}`, of type `obs_ty`, which is
    /// `obs[0]` of the first and `obs[1]` of the second.
    obs: &'static str,
    obs_ty: &'static str,
    seen: [&'static str; 2],
    /// A statement that changes the value at place `{p}`.
    change: &'static str,
    /// Is it `Copy`?
    copy: bool,
}

fn tys() -> Vec<Ty> {
    vec![
        Ty {
            name: "struct",
            items: "#[derive(Copy, Discard)]\nstruct Pair { lo: u8, hi: u8 }\n",
            ty: "Pair",
            make: ["Pair { lo: 1, hi: 0 }", "Pair { lo: 2, hi: 0 }"],
            obs: "{x}.lo",
            obs_ty: "u8",
            seen: ["1", "2"],
            change: "{p}.lo = 9;",
            copy: true,
        },
        Ty {
            name: "list",
            items: "use alloc::list::List;\n\
                    fn list(n: usize) -> { l: List<u8> | l.len() == n } with alloc {\n\
                    \x20   let mut l: List<u8> = List::new();\n\
                    \x20   let mut i: usize = 0;\n\
                    \x20   while i < n invariant l.len() == i && i <= n {\n\
                    \x20       l.push(0);\n\
                    \x20       i = i + 1;\n\
                    \x20   }\n\
                    \x20   l\n\
                    }\n",
            ty: "List<u8>",
            make: ["list(1)", "list(2)"],
            obs: "{x}.len()",
            obs_ty: "usize",
            seen: ["1", "2"],
            change: "{p}.push(9);",
            copy: false,
        },
        Ty {
            name: "enum",
            items: "",
            ty: "Option<u8>",
            make: ["Some(1)", "None"],
            obs: "{x}.is_some()",
            obs_ty: "bool",
            seen: ["true", "false"],
            change: "{p} = Some(9);",
            copy: true,
        },
        Ty {
            name: "box",
            items: "use alloc::boxed::Box;\n",
            ty: "Box<u8>",
            make: ["Box::new(1)", "Box::new(2)"],
            obs: "*{x}",
            obs_ty: "u8",
            seen: ["1", "2"],
            change: "*{p} = 9;",
            copy: false,
        },
        Ty {
            name: "tuple",
            items: "",
            ty: "(u8, u8)",
            make: ["(1, 0)", "(2, 0)"],
            obs: "{x}.0",
            obs_ty: "u8",
            seen: ["1", "2"],
            change: "{p}.0 = 9;",
            copy: true,
        },
    ]
}

/// What the function says.
#[derive(Clone, Copy, PartialEq)]
enum Spec {
    /// `*a == old(*b), *b == old(*a)`.
    Exchange,
    /// `*a == old(*b)`.
    OneWay,
    /// `-> { r | r == old(*a) } ensures *a == v`.
    Replace,
}

/// A body of the function, of `a` and `b` (or `v`), with `{change_a}` and
/// `{change_b}` for changing what they point to, `{change_r}` a local `r`.
struct Body {
    name: &'static str,
    code: &'static str,
    /// Does it keep its word, for each spec it's for? `None`: not for it.
    keeps: fn(Spec) -> Option<bool>,
    /// Only for `Copy` types, or only for those with a `change`.
    copy: bool,
    changes: bool,
}

fn bodies() -> Vec<Body> {
    use Spec::*;
    vec![
        Body {
            name: "swaps",
            code: "core::mem::swap(a, b);",
            keeps: |s| (s != Replace).then_some(true),
            copy: false,
            changes: false,
        },
        Body {
            name: "does nothing",
            code: "",
            keeps: |s| (s != Replace).then_some(false),
            copy: false,
            changes: false,
        },
        Body {
            name: "swaps twice",
            code: "core::mem::swap(a, b); core::mem::swap(a, b);",
            keeps: |s| (s != Replace).then_some(false),
            copy: false,
            changes: false,
        },
        Body {
            name: "swaps, then changes `*a`",
            code: "core::mem::swap(a, b); {change_a}",
            keeps: |s| (s != Replace).then_some(false),
            copy: false,
            changes: true,
        },
        Body {
            name: "swaps, then changes `*b`",
            code: "core::mem::swap(a, b); {change_b}",
            keeps: |s| match s {
                Exchange => Some(false),
                OneWay => Some(true),
                Replace => None,
            },
            copy: false,
            changes: true,
        },
        Body {
            name: "copies through a temporary",
            code: "let t = *a; *a = *b; *b = t;",
            keeps: |s| (s != Replace).then_some(true),
            copy: true,
            changes: false,
        },
        Body {
            name: "copies one way",
            code: "*a = *b;",
            keeps: |s| match s {
                Exchange => Some(false),
                OneWay => Some(true),
                Replace => None,
            },
            copy: true,
            changes: false,
        },
        Body {
            name: "replaces",
            code: "core::mem::replace(a, v)",
            keeps: |s| (s == Replace).then_some(true),
            copy: false,
            changes: false,
        },
        Body {
            name: "gives `v` back",
            code: "let mut t = v; core::mem::swap(a, &mut t); core::mem::swap(a, &mut t); t",
            keeps: |s| (s == Replace).then_some(false),
            copy: false,
            changes: false,
        },
        Body {
            name: "replaces, then changes `*a`",
            code: "let r = core::mem::replace(a, v); {change_a} r",
            keeps: |s| (s == Replace).then_some(false),
            copy: false,
            changes: true,
        },
        Body {
            name: "replaces, then changes what it gives",
            code: "let mut r = core::mem::replace(a, v); {change_r} r",
            keeps: |s| (s == Replace).then_some(false),
            copy: false,
            changes: true,
        },
    ]
}

/// Where the caller's two values are: `{x}` and `{y}` are their places,
/// after `{setup}`, in a function with `{params}`.
struct Shape {
    name: &'static str,
    /// Items it needs, with `{T}` for the type.
    items: &'static str,
    /// How `main` makes them, with `{one}` and `{two}` for the values.
    setup: &'static str,
    x: &'static str,
    y: &'static str,
    /// Whether the call and what's checked after are in `go`, of `x: &mut
    /// {T}` and `y: &mut {T}`, which `main` calls.
    by_param: bool,
}

fn shapes() -> Vec<Shape> {
    vec![
        Shape {
            name: "locals",
            items: "",
            setup: "let mut x: {T} = {one}; let mut y: {T} = {two};",
            x: "x",
            y: "y",
            by_param: false,
        },
        Shape {
            name: "fields",
            items: "#[derive(Discard)]\nstruct H { x: {T}, y: {T} }\n",
            setup: "let mut h = H { x: {one}, y: {two} };",
            x: "h.x",
            y: "h.y",
            by_param: false,
        },
        Shape {
            name: "parameters",
            items: "",
            setup: "let mut x: {T} = {one}; let mut y: {T} = {two};",
            x: "(*x)",
            y: "(*y)",
            by_param: true,
        },
        Shape {
            name: "a box",
            items: "",
            setup: "let mut x: alloc::boxed::Box<{T}> = alloc::boxed::Box::new({one}); \
                    let mut y: {T} = {two};",
            x: "(*x)",
            y: "y",
            by_param: false,
        },
    ]
}

/// The program: `f` with `spec` and `body`, generic if `generic`, called
/// on `shape`'s places of type `t`, then what the call says checked.
fn program(t: &Ty, spec: Spec, body: &Body, shape: &Shape, generic: bool) -> String {
    let ty = if generic { "T" } else { t.ty };
    let g = if generic { "<T>" } else { "" };
    let change = |p: &str| t.change.replace("{p}", p);
    let code = (body.code)
        .replace("{change_a}", &change("(*a)"))
        .replace("{change_b}", &change("(*b)"))
        .replace("{change_r}", &change("r"));
    let f = match spec {
        Spec::Exchange => {
            format!(
                "fn f{g}(a: &mut {ty}, b: &mut {ty}) ensures *a == old(*b), *b == old(*a) {{ {code} }}\n"
            )
        }
        Spec::OneWay => {
            format!("fn f{g}(a: &mut {ty}, b: &mut {ty}) ensures *a == old(*b) {{ {code} }}\n")
        }
        Spec::Replace => format!(
            "fn f{g}(a: &mut {ty}, v: {ty}) -> {{ r: {ty} | r == old(*a) }} ensures *a == v {{ {code} }}\n"
        ),
    };
    let obs = |place: &str, i: usize| {
        format!(
            "let _ = {{ let o: {{ v: {} | v == {} }} = {}; o }};",
            t.obs_ty,
            t.seen[i],
            t.obs.replace("{x}", place)
        )
    };
    let (x, y) = (shape.x, shape.y);
    let call = match spec {
        Spec::Exchange => {
            let (ax, ay) = match shape.by_param {
                true => ("x".to_string(), "y".to_string()),
                false => (format!("&mut {x}"), format!("&mut {y}")),
            };
            format!("f({ax}, {ay}); {} {}", obs(x, 1), obs(y, 0))
        }
        Spec::OneWay => {
            let (ax, ay) = match shape.by_param {
                true => ("x".to_string(), "y".to_string()),
                false => (format!("&mut {x}"), format!("&mut {y}")),
            };
            format!("f({ax}, {ay}); {}", obs(x, 1))
        }
        Spec::Replace => {
            let ax = match shape.by_param {
                true => "x".to_string(),
                false => format!("&mut {x}"),
            };
            format!(
                "let r = f({ax}, {}); {} {}",
                t.make[1],
                obs("r", 0),
                obs(x, 1)
            )
        }
    };
    let setup = (shape.setup)
        .replace("{T}", t.ty)
        .replace("{one}", t.make[0])
        .replace("{two}", t.make[1]);
    let items = format!("{}{}", t.items, shape.items.replace("{T}", t.ty));
    // What `go` is given: the two values, as its refinements say.
    let given = |name: &str, i: usize| {
        format!(
            "{{ {name}: &mut {} | {} == {} }}",
            t.ty,
            t.obs.replace("{x}", &format!("(*{name})")),
            t.seen[i]
        )
    };
    let main = match shape.by_param {
        true => format!(
            "fn go(x: {}, y: {}) {{ {call} }}\n\
             fn main() -> Result<(), std::io::Error> {{ {setup} go(&mut x, &mut y); Ok(()) }}\n",
            given("x", 0),
            given("y", 1)
        ),
        false => format!("fn main() -> Result<(), std::io::Error> {{ {setup} {call} Ok(()) }}\n"),
    };
    format!("{items}{f}{main}")
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn values() {
    let mut failures = Vec::new();
    let mut count = 0;
    for t in tys() {
        for body in bodies() {
            if body.copy && !t.copy {
                continue;
            }
            for spec in [Spec::Exchange, Spec::OneWay, Spec::Replace] {
                let Some(keeps) = (body.keeps)(spec) else {
                    continue;
                };
                for generic in [false, true] {
                    // Generic code can't change or copy a `T`.
                    if generic && (body.changes || body.copy) {
                        continue;
                    }
                    for shape in shapes() {
                        count += 1;
                        let src = program(&t, spec, &body, &shape, generic);
                        let what = format!(
                            "{} {} that {}{}, by {}",
                            t.name,
                            match spec {
                                Spec::Exchange => "exchange",
                                Spec::OneWay => "one-way",
                                Spec::Replace => "replace",
                            },
                            body.name,
                            if generic { " (generic)" } else { "" },
                            shape.name
                        );
                        match (keeps, verdict(&src)) {
                            (true, Ok(())) => {}
                            (false, Err(m))
                                if m.contains("of `f`") || m.contains("`f` ensures") => {}
                            (true, Err(m)) => failures.push(format!("{what}: refused: {m}\n{src}")),
                            (false, Err(m)) => failures
                                .push(format!("{what}: refused, but not for `f`: {m}\n{src}")),
                            (false, Ok(())) => {
                                failures.push(format!("{what}: accepted a broken promise\n{src}"))
                            }
                        }
                    }
                }
            }
        }
    }
    assert!(count > 300, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
