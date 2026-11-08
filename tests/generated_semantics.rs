//! Deterministic program families with known results, paired rejection
//! cases, and declaration permutations. No external fuzz runner is needed.
//! They're about what programs mean, the same on every platform, so they
//! run on Linux alone (CI's other platforms are the slowest).

use std::panic::{UnwindSafe, catch_unwind};
use t_lang::error::Phase;

const MAX_BYTES: usize = 4_096;

fn case<T>(label: &str, src: &str, check: impl FnOnce() -> T + UnwindSafe) -> T {
    assert!(src.len() <= MAX_BYTES, "{label}: input exceeds its bound");
    match catch_unwind(check) {
        Ok(value) => value,
        Err(panic) => {
            let why = panic
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| panic.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            panic!("{label}\nsource:\n{src}\n{why}");
        }
    }
}

fn accepted(src: &str, output: &str) {
    let checked = t_lang::check_source(src);
    let program = checked
        .result
        .unwrap_or_else(|e| panic!("{}", e.render_in(&checked.sources)));
    let mut terminal = t_lang::interp::Captured::default();
    let result = t_lang::interp::run(&program.lower(), &mut terminal)
        .unwrap_or_else(|e| panic!("{}", e.render_in(&checked.sources)));
    assert!(matches!(result, t_lang::interp::Value::Variant(_, ref v, _) if v == "Ok"));
    assert_eq!(String::from_utf8(terminal.out).unwrap(), output);
    assert!(terminal.err.is_empty());
}

fn rejected(src: &str, message: &str) {
    let checked = t_lang::check_source(src);
    let error = checked.result.expect_err("accepted an invalid program");
    assert_eq!(error.phase, Phase::Type);
    assert!(
        error.message.contains(message),
        "{}",
        error.render_in(&checked.sources)
    );
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_integer_boundary_guards() {
    // Host i128 arithmetic is an independent oracle for all eight T
    // integer types here, including u64's upper half and signed minima.
    for (ty, min, max) in [
        ("i8", i8::MIN as i128, i8::MAX as i128),
        ("u8", 0, u8::MAX as i128),
        ("i16", i16::MIN as i128, i16::MAX as i128),
        ("u16", 0, u16::MAX as i128),
        ("i32", i32::MIN as i128, i32::MAX as i128),
        ("u32", 0, u32::MAX as i128),
        ("i64", i64::MIN as i128, i64::MAX as i128),
        ("u64", 0, u64::MAX as i128),
    ] {
        for step in [1, 2, 3] {
            for addition in [false, true] {
                let (op, cmp, guard) = if addition {
                    ("+", ">", max - step)
                } else {
                    ("-", "<", min + step)
                };
                for valid in [true, false] {
                    // Weaken the guard by exactly one, admitting the
                    // first value whose arithmetic result is out of range.
                    let bound = guard
                        + if valid {
                            0
                        } else if addition {
                            1
                        } else {
                            -1
                        };
                    let mut src = format!(
                        "fn calc(n: {ty}) -> {ty} with {{}} {{\n\
                         if n {cmp} {bound}{ty} {{ return 0{ty}; }}\n\
                         n {op} {step}{ty}\n}}\n\
                         fn main(os: std::Os) -> Result<(), std::io::Error> {{\n\
                         let mut out = os.stdout;\n"
                    );
                    let mut output = String::new();
                    for n in [min, min + step, max - step, max] {
                        src.push_str(&format!("out.line(&calc({n}{ty}))?;\n"));
                        let value = if addition && n > guard || !addition && n < guard {
                            0
                        } else if addition {
                            n + step
                        } else {
                            n - step
                        };
                        output.push_str(&format!("{value}\n"));
                    }
                    src.push_str("Result::Ok(())\n}\n");
                    case(&format!("{ty}, {op}{step}, valid {valid}"), &src, || {
                        if valid {
                            accepted(&src, &output);
                        } else {
                            rejected(&src, &format!("`{op}` may panic"));
                        }
                    });
                }
            }
        }
    }
}

/// Every permutation of four whole declarations, not their internals.
fn declaration_orders() -> Vec<[usize; 4]> {
    let mut orders = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let order = [a, b, c, d];
                    if (0..4).all(|i| !order[..i].contains(&order[i])) {
                        orders.push(order);
                    }
                }
            }
        }
    }
    assert_eq!(orders.len(), 24);
    orders
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_mutation_refinement_and_declaration_orders() {
    // Each route can set the previously positive divisor to zero. A
    // fresh guard must recover the proof; omitting it must be rejected.
    let mutations = [
        "s.divisor = 0;",
        "write(s);",
        "let r: &mut State = s; write(r);",
        "let mut update = || s.divisor = 0; update();",
        "let r: &mut State = s; let mut update = || write(r); update();",
    ];
    for (route, mutation) in mutations.into_iter().enumerate() {
        for guarded in [true, false] {
            let guard = if guarded {
                "if d <= 0 { return 0; }"
            } else {
                ""
            };
            let mut main = String::from(
                "fn main(os: std::Os) -> Result<(), std::io::Error> {\n\
                 let mut out = os.stdout;\n",
            );
            let mut output = String::new();
            for divisor in [-1, 0, 1, 3] {
                for change in [false, true] {
                    main.push_str(&format!(
                        "{{ let mut s = State {{ divisor: {divisor} }};\n\
                         out.line(&compute(&mut s, {change}))?; }}\n"
                    ));
                    let value = if divisor <= 0 || change {
                        0
                    } else {
                        12 / divisor
                    };
                    output.push_str(&format!("{value}\n"));
                }
            }
            main.push_str("Result::Ok(())\n}\n");
            let declarations = [
                "#[derive(Copy, Discard)] struct State { divisor: i64 }".into(),
                // Omit effects on the helper so permutations also exercise
                // inference through a callee declared after its caller.
                "fn write(s: &mut State) { s.divisor = 0; }".into(),
                format!(
                    "fn compute(s: &mut State, change: bool) -> i64 with {{}} {{\n\
                     if s.divisor <= 0 {{ return 0; }}\n\
                     if change {{ {mutation} }}\n\
                     let d = s.divisor;\n{guard}\n\
                     let positive: {{ v: i64 | v > 0 }} = d;\n\
                     12 / positive\n}}"
                ),
                main,
            ];
            for order in declaration_orders() {
                let src = order.map(|i| declarations[i].as_str()).join("\n");
                case(
                    &format!("mutation route {route}, guarded {guarded}, order {order:?}"),
                    &src,
                    || {
                        if guarded {
                            accepted(&src, &output);
                        } else {
                            rejected(&src, "can't prove `v > 0`");
                        }
                    },
                );
            }
        }
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_closure_facts_do_not_escape_early_returns() {
    for initial in [0, -1, 2] {
        for positive in [false, true] {
            let (condition, predicate) = if positive { ("<=", ">") } else { ("==", "!=") };
            for outside_guard in [true, false] {
                let guard = if outside_guard {
                    format!("if d {condition} 0 {{ return 0; }}")
                } else {
                    String::new()
                };
                let src = format!(
                    "fn refined(n: {{ v: i64 | v {predicate} 0 }}) -> i64 {{ n }}\n\
                     fn pick() -> i64 {{ {initial} }}\n\
                     fn compute() -> i64 {{\n\
                     let d = pick();\n\
                     let read = || {{ if d {condition} 0 {{ return 0; }} refined(d) }};\n\
                     let _ = read();\n{guard}\nrefined(d)\n}}\n\
                     fn main(os: std::Os) -> Result<(), std::io::Error> {{\n\
                     let mut out = os.stdout; out.line(&compute())?; Result::Ok(())\n}}"
                );
                case(
                    &format!("initial {initial}, positive {positive}, outer guard {outside_guard}"),
                    &src,
                    || {
                        if outside_guard {
                            let value = if initial == 0 || positive && initial < 0 {
                                0
                            } else {
                                initial
                            };
                            accepted(&src, &format!("{value}\n"));
                        } else {
                            rejected(&src, &format!("can't prove `v {predicate} 0`"));
                        }
                    },
                );
            }
        }
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_borrowed_closure_results_and_early_returns() {
    for initial in [-1, 0, 2] {
        for depth in 0..=2 {
            for inverted in [false, true] {
                for valid in [true, false] {
                    let reference = if valid { "outer" } else { "&local" };
                    let mut body = reference.to_string();
                    for _ in 0..depth {
                        body = format!("{{ let inner = || {body}; inner() }}");
                    }
                    let condition = if inverted { "!stop" } else { "stop" };
                    let src = format!(
                        "fn choose<'a>(outer: &'a i64, stop: bool) -> &'a i64 {{\n\
                         let local: i64 = {initial};\nlet get = || {body};\n\
                         if {condition} {{ return get(); }}\nouter\n}}\n\
                         fn main(os: std::Os) -> Result<(), std::io::Error> {{\n\
                         let mut out = os.stdout; let n = {initial};\n\
                         out.line(&(*choose(&n, true)))?;\n\
                         out.line(&(*choose(&n, false)))?; Result::Ok(())\n}}"
                    );
                    case(
                        &format!(
                            "borrow initial {initial}, depth {depth}, inverted {inverted}, valid {valid}"
                        ),
                        &src,
                        || {
                            if valid {
                                accepted(&src, &format!("{initial}\n{initial}\n"));
                            } else {
                                rejected(&src, "doesn't live long enough");
                            }
                        },
                    );
                }
            }
        }
    }
}

/// Every way the language has of lending a list to code that changes it,
/// one per line: what's known of `l.len()` before must be forgotten after
/// each, whatever its syntax. Step 53 found a call by path,
/// `List::push(&mut l, x)`, that learned what `push` ensures of the list
/// before the call (in every release since 0.43.0); the walk derives its
/// places and lends from the syntax, so each form is a separate way in.
const LEND_FORMS: &[&str] = &[
    "l.push(1u8);",
    "List::push(&mut l, 1u8);",
    "(&mut l).push(1u8);",
    "({ &mut l }).push(1u8);",
    "List::push({ &mut l }, 1u8);",
    "let r: &mut List<u8> = &mut l; List::push(r, 1u8);",
    "(if c { &mut l } else { &mut m }).push(1u8);",
    "grow(&mut l);",
    "grow_w(W { r: &mut l });",
    "match &mut l { r => r.push(1u8) }",
    "(|| l.push(1u8))();",
    "let mut f = || l.push(1u8); f();",
    "let g = |x: &mut List<u8>| x.push(1u8); g(&mut l);",
    "let s = f\"{ { l.push(1u8); 0u8 } }\"; let _ = s;",
    "let mut a = [&mut l]; a[0].push(1u8);",
    "pick(c, &mut l, &mut m).push(1u8);",
    "let r = &mut l; let q = &mut *r; q.push(1u8);",
    "let t = &mut l; let u = t; u.push(1u8);",
    "let mut w = W { r: &mut l }; w.r.push(1u8);",
    "let _ = core::mem::replace(&mut l, List::filled(1, 0u8));",
    "core::mem::swap(&mut l, &mut List::filled(1, 0u8));",
    "let mut o = Some(&mut l); if let Some(r) = &mut o { r.push(1u8); }",
    "for _ in 0..1u8 { l.push(1u8); }",
    "let mut k: u8 = 0; while k < 1 { List::push(&mut l, 1u8); k = k + 1; }",
    "let mut k: u8 = 0; loop { if k > 0 { break; } l.push(1u8); k = k + 1; }",
    "let mut k: u8 = 0; while let Some(_) = Some(k) { if k > 0 { break; } l.push(1u8); k = k + 1; }",
];

fn lend_program(change: &str) -> String {
    format!(
        "use alloc::list::List;\n\
         #[derive(Discard)]\n\
         struct W<'a> {{ r: &'a mut List<u8> }}\n\
         fn grow(l: &mut List<u8>) with alloc {{ l.push(1u8); }}\n\
         fn grow_w(w: W) with alloc {{ w.r.push(1u8); }}\n\
         fn pick<'a>(c: bool, x: &'a mut List<u8>, y: &'a mut List<u8>) -> &'a mut List<u8> {{\n\
         if c {{ x }} else {{ y }}\n}}\n\
         fn read(c: bool) -> u8 with alloc {{\n\
         let mut l: List<u8> = List::new();\n\
         let mut m: List<u8> = List::new();\n\
         let b: [u8; 1] = [7];\n\
         if l.len() == 0 && m.len() == 0 {{\n{change}\nb[l.len()]\n}} else {{ 0 }}\n}}\n\
         fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {{\n\
         let mut out = os.stdout;\nout.line(&read(true))?;\nOk(())\n}}\n"
    )
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn generated_lend_forms_forget_measures() {
    // Controls: what doesn't touch `l` keeps `l.len() == 0`.
    for keep in ["", "m.push(1u8);", "grow(&mut m);", "let _ = m.len();"] {
        let src = lend_program(keep);
        case(&format!("keep `{keep}`"), &src, || accepted(&src, "7\n"));
    }
    for form in LEND_FORMS {
        // Straight, in one arm of a branch, and in a loop's body.
        for change in [
            form.to_string(),
            format!("if c {{ {form} }}"),
            format!("let mut n: u8 = 0; while n < 1 {{ {form} n = n + 1; }}"),
        ] {
            let src = lend_program(&change);
            case(&format!("lend `{change}`"), &src, || {
                rejected(&src, "can't prove `i < self.len()`")
            });
        }
    }
}
