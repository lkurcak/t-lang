// A closure that borrows `n` can return a reference to it, which lasts as
// long as the borrow does: as long as `n`, not just the call. Its result
// type isn't written: `-> &i64` would say the result lives as long as the
// argument, as for a function (`err_soundness_closure_hr_return_capture.t`).
fn main() {
    let n: i64 = 5;
    let f = || &n;
    print(*f());                    // expect: 5
    let m = 2;
    let g = |_r: &i64| &n;
    print(*g(&m));                  // expect: 5
}
