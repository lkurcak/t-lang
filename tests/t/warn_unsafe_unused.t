// An `unsafe` block around nothing unsafe takes on no obligations, and
// only hides where the unsafe operations aren't.
fn main() {
    // SAFETY: nothing to keep.
    let n = unsafe { 1 };
    print(n); // expect: 1
}
// warning: this `unsafe` block has nothing unsafe in it
