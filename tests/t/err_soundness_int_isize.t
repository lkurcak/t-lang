// The least `isize` is as unknown as the greatest: it's at most
// -2147483648, but that's all.
fn f(n: { n: isize | n > -2147483648 }) -> isize { n - 1 }

fn g(n: { n: isize | n > -2147483649 }) -> isize { n - 1 }

fn main() {}
// error: `-` may panic, but `g` doesn't say `with panic`
