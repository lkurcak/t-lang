// A `for` loop calls `next`, and so has its effects.
fn count<I: Iterator<Item = int> + Discard>(it: I) -> bool {
    for _x in it {
        return true;
    }
    false
}

fn main() {
    print_bool(count(0..3));
}
// error: `next` may have the effects `I::e`, but `count` doesn't say `with I::e`
