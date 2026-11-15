// A closure's value with two returns, called straight: what it gives is
// either one's, the one after the first too.
fn main(os: std::Os) {
    let c = os.args.len() > 5;
    let f = || {
        if c {
            return 1u32;
        }
        0u32
    };
    let x: { v: u32 | v > 0 } = f();
    let _ = x;
}
// error: can't prove `v > 0` for `x`
