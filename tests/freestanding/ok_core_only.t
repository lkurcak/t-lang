// With no `t.toml`, a program built for the freestanding target has core
// and alloc.
fn main() {
    let b = alloc::boxed::Box::new(3u8);
}
