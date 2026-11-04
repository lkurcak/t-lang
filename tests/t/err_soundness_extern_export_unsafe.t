// C calls an export, and keeps no obligations: it can't say `with unsafe`.
extern fn reset() with unsafe {}
// error: an exported `extern fn` can't say `with unsafe`

fn main() {}
