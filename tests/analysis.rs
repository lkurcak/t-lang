//! Semantic analysis and executable lowering are separate public phases.

#[test]
fn syntax_errors_cannot_produce_a_checked_program() {
    let src = "fn before() {}\nfn broken(x:) {}\nfn main() {}\n";
    let analysis = t_lang::check_source(src);
    let error = analysis.result.unwrap_err();
    assert_eq!(error.phase, t_lang::error::Phase::Syntax);
    assert!(analysis.info.notes.is_empty());
    let run = t_lang::run_source(src);
    assert_eq!(run.result.unwrap_err().phase, t_lang::error::Phase::Syntax);
    assert!(run.output.is_empty());
}

#[test]
fn lowering_preserves_the_checked_program() {
    let src = "fn id<T: Copy>(x: T) -> T { x }\nfn main() { fn nested() -> i64 { 42 } let _n = id(nested()); }\n";
    let analysis = t_lang::check_source(src);
    let checked = analysis.result.as_ref().unwrap();
    let before = checked.program().to_string();
    assert!(checked.program().fns.iter().any(|f| f.name.name == "id"));
    assert!(
        !checked
            .program()
            .fns
            .iter()
            .any(|f| f.name.name.contains("nested"))
    );
    let executable = checked.lower();
    assert!(
        executable
            .program()
            .fns
            .iter()
            .all(|f| f.generics.is_empty())
    );
    assert!(
        executable
            .program()
            .fns
            .iter()
            .any(|f| f.name.name == "main::nested")
    );
    let mut terminal = t_lang::interp::Captured::default();
    assert!(t_lang::interp::run(&executable, &mut terminal).is_ok());
    assert_eq!(checked.program().to_string(), before);
    assert!(std::ptr::eq(checked.info(), analysis.info.as_ref()));
}
