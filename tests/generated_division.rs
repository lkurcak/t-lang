//! Deterministic families for `/` and `%` by a constant in predicates
//! (docs/72-division.md), with host `i128` arithmetic as the oracle: a
//! body's quotient is a predicate's, rounding towards zero, by either
//! sign of divisor; a claim one off is refused; and what a predicate
//! assumes of a remainder pins a value down exactly when the integers do.
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

/// Checks `src`: each proof it has fails, and the first is of `message`.
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

/// `n` as T writes it in a predicate or a body: the least `i64` is out of
/// a literal's range, so it's one less than its negation.
fn lit(n: i128) -> String {
    if n == i128::from(i64::MIN) {
        "(-9223372036854775807 - 1)".to_string()
    } else {
        n.to_string()
    }
}

const DIVISORS: [i128; 10] = [1, -1, 2, -2, 3, -3, 7, -7, 10, -10];

fn dividends() -> Vec<i128> {
    let mut all: Vec<i128> = (-9..=9).collect();
    all.extend([
        i128::from(i64::MIN),
        i128::from(i64::MIN) + 1,
        i128::from(i64::MAX) - 1,
        i128::from(i64::MAX),
    ]);
    all
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn a_body_divides_as_a_predicate_does() {
    // Every function in one program: each must be proved, and run, its
    // result checked against its refinement by the interpreter.
    for op in ["/", "%"] {
        let mut items = String::new();
        let mut calls = String::new();
        let mut output = String::new();
        let mut n = 0;
        for a in dividends() {
            for k in DIVISORS {
                // The least `i64` by -1 overflows: its proof fails.
                if a == i128::from(i64::MIN) && k == -1 {
                    continue;
                }
                let q = if op == "/" { a / k } else { a % k };
                let (a_lit, k_lit, q_lit) = (lit(a), lit(k), lit(q));
                items.push_str(&format!(
                    "fn f{n}(a: {{ v: i64 | v == {a_lit} }}) -> \
                     {{ r: i64 | r == {q_lit} && r == a {op} {k_lit} }} {{ a {op} {k_lit} }}\n"
                ));
                calls.push_str(&format!("    out.line(&f{n}({a_lit}))?;\n"));
                output.push_str(&format!("{q}\n"));
                n += 1;
            }
        }
        let src = format!(
            "{items}\nfn main(os: std::Os) -> Result<(), std::io::Error> {{\n    \
             let mut out = os.stdout;\n{calls}    Ok(())\n}}\n"
        );
        accepted(&src, &output);
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn a_quotient_one_off_is_refused() {
    // Fewer dividends than above, each case a program of its own: one
    // below or one above, of the predicate's quotient or the body's, in
    // turn.
    let mut dividends: Vec<i128> = vec![-9, -7, -4, -1, 0, 1, 3, 8, 9];
    dividends.extend([i128::from(i64::MIN), i128::from(i64::MAX)]);
    let mut turn = 0;
    for op in ["/", "%"] {
        for &a in &dividends {
            for k in DIVISORS {
                if a == i128::from(i64::MIN) && k == -1 {
                    continue;
                }
                turn += 1;
                let q = if op == "/" { a / k } else { a % k };
                let wrong = if turn % 2 == 0 { q - 1 } else { q + 1 };
                let (a_lit, k_lit, w_lit) = (lit(a), lit(k), lit(wrong));
                let src = if turn % 4 < 2 || i64::try_from(wrong).is_err() {
                    format!(
                        "fn f(a: {{ v: i64 | v == {a_lit} }}) -> \
                         {{ r: bool | a {op} {k_lit} == {w_lit} }} {{ true }}\n\
                         fn main() {{}}\n"
                    )
                } else {
                    format!(
                        "fn f(a: {{ v: i64 | v == {a_lit} }}) -> \
                         {{ r: i64 | r == {w_lit} }} {{ a {op} {k_lit} }}\n\
                         fn main() {{}}\n"
                    )
                };
                rejected(&src, "can't prove");
            }
        }
    }
}

#[test]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "the same on every platform: run on Linux"
)]
fn a_remainder_assumed_pins_down_what_the_integers_do() {
    // `v % k == r` with `lo <= v <= hi`: if one integer is that, it's
    // proved to be it; if more are, the least isn't proved to be it.
    let mut items = String::new();
    let mut calls = String::new();
    let mut output = String::new();
    let mut n = 0;
    for k in [2i128, 3, 4, -4, 5] {
        let m = k.abs();
        for r in -(m - 1)..m {
            for lo in [-11i128, -4, -2, 1, 6] {
                for width in [1i128, 3, 8] {
                    let hi = lo + width;
                    let s: Vec<i128> = (lo..=hi).filter(|v| v % k == r).collect();
                    let pred = format!("v % {k} == {r} && v >= {lo} && v <= {hi}");
                    match s.as_slice() {
                        [] => {}
                        [one] => {
                            items.push_str(&format!(
                                "fn f{n}(a: {{ v: i64 | {pred} }}) -> {{ r: i64 | r == {one} }} \
                                 {{ a }}\n"
                            ));
                            calls.push_str(&format!("    out.line(&f{n}({one}))?;\n"));
                            output.push_str(&format!("{one}\n"));
                            n += 1;
                        }
                        [least, ..] => rejected(
                            &format!(
                                "fn f(a: {{ v: i64 | {pred} }}) -> {{ r: i64 | r == {least} }} \
                                 {{ a }}\nfn main() {{}}\n"
                            ),
                            "can't prove",
                        ),
                    }
                }
            }
        }
    }
    assert!(n > 50, "too few cases pinned down: {n}");
    let src = format!(
        "{items}\nfn main(os: std::Os) -> Result<(), std::io::Error> {{\n    \
         let mut out = os.stdout;\n{calls}    Ok(())\n}}\n"
    );
    accepted(&src, &output);
}
