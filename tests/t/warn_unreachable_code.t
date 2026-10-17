// Code after something that never finishes never runs. Like rustc, T warns
// about the first statement of it in each block.

fn after_return() -> int {
    return 1;
    print(2);
    3
}

fn after_both_branches(c: bool) {
    if c { return; } else { return; }
    print(1);
}

fn after_break() {
    while true {
        break;
        print(1);
    }
}

// A branch that may not run doesn't count.
fn only_one_branch(c: bool) -> int {
    if c { return 1; }
    2
}

// Dead code is type checked, but not borrow checked: it never runs. Here
// it would need `'b` to outlive `'a`.
fn dead_borrow<'a, 'b>(x: &'a int, y: &'b int) -> &'a int {
    return x;
    y
}

#[allow(unreachable_code)]
fn allowed() -> int {
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
