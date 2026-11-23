//! Deterministic families for what a function was given, `old(..)`
//! (docs/30-postconditions.md, docs/92-where-a-bound-holds.md), at every
//! depth: a value behind one to seven references, plain, through structs
//! that hold the next reference, through boxes, or behind a reference in
//! a struct or a box that's behind them; named by a
//! postcondition, a loop's invariant, or a result's refinement; by a body
//! that writes a new value there first, which `old(..)` mustn't see, or
//! one that leaves it as it was. Each is refused, or accepted and run by
//! the interpreter, which checks every postcondition, invariant and
//! refinement with what was there at the call. What's known as it was at
//! the call is only so deep (`ENTRY_DEPTH`): deeper, `old(..)` is unknown,
//! never what's there when it's read (retrospective 18). They're about
//! what programs mean, so they run on Linux alone.

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

/// How the value is reached from the parameter `a`.
#[derive(Clone, Copy, Debug)]
enum Shape {
    /// `&mut &mut .. u8`.
    Refs,
    /// `&mut L1`, where each `Lk` holds `p: &mut L(k+1)`, and the last `n`.
    Structs,
    /// `&mut Box<Box<.. u8>>`.
    Boxes,
    /// `&mut &mut .. S`, where `S` holds `q: &mut u8`.
    RefsToStruct,
    /// `&mut &mut .. Box<&mut u8>`.
    RefsToBox,
}

/// Where `old(..)` is.
#[derive(Clone, Copy, Debug)]
enum Use {
    Ensures,
    Invariant,
    Result,
}

/// The parameter's type, the value's path from `a`, the items it needs,
/// and a `main` that calls `f` with a value 1 there, for `depth`
/// references (or a reference and boxes) to it, or to what holds its
/// last reference.
fn shaped(shape: Shape, depth: usize) -> (String, String, String, String) {
    match shape {
        Shape::Refs => {
            let ty = format!("{}u8", "&mut ".repeat(depth));
            let path = format!("{}a", "*".repeat(depth));
            let mut main = String::from("    let mut x0: u8 = 1;\n");
            for k in 1..depth {
                main += &format!("    let mut x{k} = &mut x{};\n", k - 1);
            }
            main += &format!("    f(&mut x{});\n", depth - 1);
            (ty, path, String::new(), main)
        }
        Shape::Structs => {
            let mut items = String::new();
            for k in 1..=depth {
                let next = if k + 1 == depth { "" } else { "<'a>" };
                let field = match k == depth {
                    true => "n: u8".to_string(),
                    false => format!("p: &'a mut L{}{next}", k + 1),
                };
                let params = if k == depth { "" } else { "<'a>" };
                items += &format!("#[derive(Discard)]\nstruct L{k}{params} {{ {field} }}\n\n");
            }
            let first = if depth == 1 { "L1" } else { "L1<'_>" };
            let ty = format!("&mut {first}");
            let path = format!("a{}.n", ".p".repeat(depth - 1));
            let mut main = format!("    let mut l{depth} = L{depth} {{ n: 1 }};\n");
            for k in (1..depth).rev() {
                main += &format!("    let mut l{k} = L{k} {{ p: &mut l{} }};\n", k + 1);
            }
            main += "    f(&mut l1);\n";
            (ty, path, items, main)
        }
        Shape::Boxes => {
            let boxes = depth - 1;
            let ty = format!("&mut {}u8{}", "Box<".repeat(boxes), ">".repeat(boxes));
            let path = format!("{}a", "*".repeat(depth));
            let mut value = "1u8".to_string();
            for _ in 0..boxes {
                value = format!("Box::new({value})");
            }
            let main = format!("    let mut b = {value};\n    f(&mut b);\n");
            (ty, path, String::new(), main)
        }
        Shape::RefsToStruct | Shape::RefsToBox => {
            let (inner, items, held) = match shape {
                Shape::RefsToStruct => (
                    "S<'_>",
                    "#[derive(Discard)]\nstruct S<'a> { q: &'a mut u8 }\n\n",
                    "S { q: &mut x }",
                ),
                _ => (
                    "Box<&mut u8>",
                    "use alloc::boxed::Box;\n\n",
                    "Box::new(&mut x)",
                ),
            };
            let ty = format!("{}{inner}", "&mut ".repeat(depth));
            let to = format!("{}a", "*".repeat(depth));
            let path = match shape {
                Shape::RefsToStruct => format!("*({to}).q"),
                _ => format!("**{to}"),
            };
            let mut main = format!("    let mut x: u8 = 1;\n    let mut s0 = {held};\n");
            for k in 1..depth {
                main += &format!("    let mut s{k} = &mut s{};\n", k - 1);
            }
            main += &format!("    f(&mut s{});\n", depth - 1);
            (ty, path, items.to_string(), main)
        }
    }
}

/// `f`, saying with `old(..)` that the value is what it was, as `use_`
/// says, and writing 3 there first if `lie`.
fn program(shape: Shape, depth: usize, use_: Use, lie: bool) -> String {
    let (ty, path, items, main) = shaped(shape, depth);
    let write = match lie {
        true => format!("    {path} = 3;\n"),
        false => format!("    let v = {path};\n    {path} = v;\n"),
    };
    let f = match use_ {
        Use::Ensures => {
            format!("fn f(a: {ty}) with {{}}\nensures\n    {path} == old({path}),\n{{\n{write}}}\n")
        }
        Use::Invariant => format!(
            "fn f(a: {ty}) with {{}} {{\n{write}    let mut i: u8 = 0;\n    \
             while i < 2 invariant {path} == old({path}) {{\n        i = i + 1;\n    }}\n}}\n"
        ),
        Use::Result => format!(
            "fn f(a: {ty}) -> {{ r: u8 | r == old({path}) }} with {{}} {{\n{write}    {path}\n}}\n"
        ),
    };
    let main = match use_ {
        Use::Result => main.replace("    f(", "    let _r = f("),
        _ => main,
    };
    let alloc = match shape {
        Shape::Boxes if depth > 1 => "alloc",
        Shape::RefsToBox => "alloc",
        _ => "{}",
    };
    let import = match shape {
        Shape::Boxes if depth > 1 => "use alloc::boxed::Box;\n\n",
        _ => "",
    };
    format!("{import}{items}{f}\nfn main() with {alloc} {{\n{main}}}\n")
}

#[test]
#[cfg_attr(not(target_os = "linux"), ignore)]
fn old_at_every_depth() {
    let mut failures = Vec::new();
    let mut count = 0;
    for shape in [
        Shape::Refs,
        Shape::Structs,
        Shape::Boxes,
        Shape::RefsToStruct,
        Shape::RefsToBox,
    ] {
        for depth in 1..=7 {
            for use_ in [Use::Ensures, Use::Invariant, Use::Result] {
                for lie in [false, true] {
                    count += 1;
                    let src = program(shape, depth, use_, lie);
                    let got = verdict(&src);
                    // How deep what was given is known: a reference and a
                    // struct each count one, and so does a box.
                    let levels = match shape {
                        Shape::Structs => 2 * depth - 1,
                        Shape::RefsToStruct | Shape::RefsToBox => depth + 2,
                        _ => depth,
                    };
                    let ok = match (lie, &got) {
                        // `old(..)` of what was written over is refused,
                        // however deep.
                        (true, Err(m)) => !m.starts_with("accepted"),
                        (true, Ok(())) => false,
                        // What was given, unchanged, is known as it was
                        // while it's within the depth; deeper, it may not
                        // be proved, but what's accepted runs.
                        (false, r) if levels <= 4 => r.is_ok(),
                        (false, Ok(())) => true,
                        (false, Err(m)) => !m.starts_with("accepted"),
                    };
                    if !ok {
                        let what = format!(
                            "{shape:?}, {depth} deep, in {use_:?}, {}",
                            if lie { "written first" } else { "unchanged" }
                        );
                        failures.push(format!("{what}: {got:?}\n{src}"));
                    }
                }
            }
        }
    }
    assert!(count >= 210, "only {count} programs");
    assert!(
        failures.is_empty(),
        "{} of {count} failed:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}
