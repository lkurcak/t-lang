//! Failed proofs retain their conservative meaning and explain what the
//! programmer would need to establish, using current source values.

use t_lang::error::Error;

fn rejected(src: &str) -> (Error, String) {
    let checked = t_lang::check_source(src);
    let error = checked.result.unwrap_err();
    let rendered = error.render_in(&checked.sources);
    (error, rendered)
}

#[test]
fn arithmetic_explains_the_obligation_and_available_facts() {
    let (_, rendered) =
        rejected("fn inc(n: i64) -> i64 with {} { n + 1 }\nfn main() { let _ = inc(0); }");
    for text in [
        "`+` may panic",
        "required condition:",
        "n <= 9223372036854775806",
        "known here:",
        "n <= 9223372036854775807",
        "this does not show that it is false",
    ] {
        assert!(rendered.contains(text), "missing {text:?}:\n{rendered}");
    }
    assert!(!rendered.contains("Lin {"), "{rendered}");
    assert_eq!(rendered.matches("-->").count(), 1, "{rendered}");
    assert!(
        rendered.contains("= note: required condition:"),
        "{rendered}"
    );
}

#[test]
fn indexing_identifies_the_missing_upper_bound() {
    let (_, rendered) = rejected(
        "fn at(s: &[i64], i: usize) -> i64 { s[i] }\nfn main() { let a: [i64; 1] = [1]; let _ = at(a.as_slice(), 0); }",
    );
    assert!(
        rendered.contains("required condition: i < s.len()"),
        "{rendered}"
    );
    assert!(rendered.contains("known here: 0 <= i"), "{rendered}");
    assert!(!rendered.contains("counterexample"), "{rendered}");
}

#[test]
fn unsupported_reasoning_differs_from_solver_arithmetic_overflow() {
    let (_, nonlinear) =
        rejected("fn square(n: i64) -> i64 with {} { n * n }\nfn main() { let _ = square(0); }");
    assert!(
        nonlinear.contains("supported linear reasoning"),
        "{nonlinear}"
    );
    assert!(
        !nonlinear.contains("solver's checked integer arithmetic range"),
        "{nonlinear}"
    );

    let (_, arithmetic) = rejected(include_str!("t/err_refine_solver_coefficient_min.t"));
    assert!(
        arithmetic.contains("solver's checked integer arithmetic range"),
        "{arithmetic}"
    );
    assert!(
        !arithmetic.contains("this does not show that it is false"),
        "{arithmetic}"
    );
}

#[test]
fn overflow_while_building_the_obligation_keeps_its_reason() {
    let (_, rendered) = rejected(
        "\
fn huge(x: i64) -> { r: i64 | r * 9223372036854775807 * 9223372036854775807 * 4 > 0 } { x }
fn main() { let _ = huge(0); }
",
    );
    assert!(
        rendered.contains("solver's checked integer arithmetic range"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("supported linear reasoning"),
        "{rendered}"
    );
}

#[test]
fn mutation_and_mutable_lending_explain_lost_facts_at_the_source_event() {
    for (src, cause) in [
        (
            include_str!("t/err_soundness_refine_place_assign.t"),
            "assignment",
        ),
        (
            include_str!("t/err_soundness_refine_place_pass_mut.t"),
            "mutable lending",
        ),
        (
            include_str!("t/err_soundness_measure_pop.t"),
            "mutable lending",
        ),
    ] {
        let (error, rendered) = rejected(src);
        let note = error.notes.iter().find(|(_, n)| n.starts_with(cause));
        assert!(note.is_some(), "{rendered}");
        let (span, message) = note.unwrap();
        assert_ne!(*span, error.span, "{rendered}");
        assert!(
            message.contains("invalidated facts about the previous value"),
            "{rendered}"
        );
        if cause == "assignment" {
            assert!(message.contains("`p.x`"), "{rendered}");
            // The old branch fact must not be described as true of p.x now.
            assert!(
                !error
                    .notes
                    .iter()
                    .any(|(_, n)| n.starts_with("known here:") && n.contains("0 < p.x")),
                "{rendered}"
            );
        }
    }
}

#[test]
fn invalidation_in_a_nonreaching_branch_is_not_reported() {
    let (_, rendered) = rejected(
        "\
fn zero(x: &mut i64) { *x = 0; }
fn f(mut x: i64, take: bool) -> i64 with {} {
    if take { zero(&mut x); return 0; }
    10 / x
}
fn main() { let _ = f(0, false); }
",
    );
    assert!(rendered.contains("required condition:"), "{rendered}");
    assert!(
        !rendered.contains("mutable lending invalidated"),
        "{rendered}"
    );
}

#[test]
fn shadowing_does_not_present_a_fact_about_the_old_binding_as_current() {
    let (error, rendered) = rejected(
        "\
fn positive(x: { x: i64 | x > 0 }) { let _ = x; }
fn f(x: { x: i64 | x > 0 }) { let x = x - 1; positive(x); }
fn main() { f(1); }
",
    );
    assert!(rendered.contains("required condition: 0 < x"), "{rendered}");
    assert!(
        !error
            .notes
            .iter()
            .any(|(_, n)| n.starts_with("known here:") && n.contains("0 < x")),
        "{rendered}"
    );
}

#[test]
fn proof_details_survive_closure_effect_checking() {
    let (_, rendered) = rejected(
        "\
fn use_it<F: Fn(i64) -> i64 + Discard>(f: F) { let _ = f(1); }
fn main() { use_it(|n| n + 1); }
",
    );
    assert!(rendered.contains("required condition:"), "{rendered}");
    assert!(rendered.contains("n <= 9223372036854775806"), "{rendered}");
}

#[test]
fn established_proofs_still_accept_and_run_under_the_soundness_oracle() {
    let run = t_lang::run_source(
        "\
fn inc(n: { n: i64 | n < 9223372036854775807 }) -> i64 { n + 1 }
fn at(s: &[i64], i: { i: usize | i < s.len() }) -> i64 { s[i] }
fn main() { let a: [i64; 1] = [41]; let _ = inc(41); let _ = at(a.as_slice(), 0); }
",
    );
    assert!(run.result.is_ok(), "{:?}", run.result);
}
