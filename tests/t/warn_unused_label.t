// A label nothing names does nothing. The fix removes it.
fn main() {
    let mut i: i64 = 0;
    'outer: while i < 3 {
        i += 1;
        if i == 2 { break; }
    }
    print(i); // expect: 2
}
// warning: unused label `'outer`: no `break` or `continue` names it
