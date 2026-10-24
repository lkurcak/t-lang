// Code after something that never finishes never runs. Like rustc, T warns
// about the first statement of it in each block.

fn note(_x: i64) {}

fn after_return() -> i64 {
    return 1;
    note(2);
    3
}

fn after_both_branches(c: bool) {
    if c { return; } else { return; }
    note(1);
}

fn after_break() {
    while true {
        break;
        note(1);
    }
}

// A branch that may not run doesn't count.
fn only_one_branch(c: bool) -> i64 {
    if c { return 1; }
    2
}

// Dead code is type checked, but not borrow checked: it never runs. Here
// it would need `'b` to outlive `'a`.
fn dead_borrow<'a, 'b>(x: &'a i64, y: &'b i64) -> &'a i64 {
    return x;
    y
}

#[allow(unreachable_code)]
fn allowed() -> i64 {
    return 1;
    2
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&after_return())?;          // expect: 1
    after_both_branches(true);
    after_break();
    out.line(&only_one_branch(false))?;  // expect: 2
    out.line(&(*dead_borrow(&3, &4)))?;    // expect: 3
    out.line(&allowed())?;               // expect: 1
    Result::Ok(())
}
// warning: unreachable code: what comes before it never finishes, so this never runs
// warning: unreachable code
// warning: unreachable code
// warning: unreachable code
