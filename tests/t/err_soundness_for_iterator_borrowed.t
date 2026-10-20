// `for x in &mut it` borrows `it` for the whole loop: it can't be looked at
// in the body.
fn main() {
    let mut it = 0..3;
    for i in &mut it {
        print(it.start());
        print(i);
    }
}
// error: cannot borrow `it` while `it` is mutably borrowed
