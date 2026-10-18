// A closure that borrows `n` can return a reference to it, which lasts as
// long as the borrow does: as long as `n`, not just the call.
fn main() {
    let n = 5;
    let f = || &n;
    print(*f());                    // expect: 5
    let m = 2;
    let g = |_r: &int| -> &int { &n };
    print(*g(&m));                  // expect: 5
}
