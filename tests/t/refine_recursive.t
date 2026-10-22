// A recursive call may be assumed to have the refinement its function
// promises: a call that never returns gives nothing to count on.
fn count_down(n: { v: i64 | v >= 0 }) -> { r: i64 | r == 0 } {
    if n == 0 { 0 } else { count_down(n - 1) }
}

fn main() {
    print(count_down(5)); // expect: 0
}
