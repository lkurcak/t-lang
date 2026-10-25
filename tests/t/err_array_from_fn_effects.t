// `from_fn` does what its closure does: here, panic, which `main` doesn't
// say it may.
fn main() {
    let _a: [i64; 2] = core::array::from_fn(|i| if i == 0 { panic("no") } else { 1 });
}
// error: `core::array::from_fn` may panic, but `main` doesn't say `with panic`
