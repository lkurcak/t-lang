// Code after something that never finishes never runs. Like rustc, T warns
// about the first statement of it in each block.

fn after_return() -> i64 {
    return 1;
    print(2i64);
    3
}

fn after_both_branches(c: bool) {
    if c { return; } else { return; }
    print(1i64);
}

fn after_break() {
    while true {
        break;
        print(1i64);
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

fn main() {
    print(after_return());          // expect: 1
    after_both_branches(true);
    after_break();
    print(only_one_branch(false));  // expect: 2
    print(*dead_borrow(&3, &4));    // expect: 3
    print(allowed());               // expect: 1
}
// warning: unreachable code: what comes before it never finishes, so this never runs
// warning: unreachable code
// warning: unreachable code
// warning: unreachable code
