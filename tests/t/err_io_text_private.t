// The adapter that writes text into a byte sink is core's own: a program
// writes text with `put` and `line`.
fn main() {
    let mut bytes = alloc::list::List::<u8>::new();
    let _ = core::io::Text { sink: &mut bytes }; // error: private
}
