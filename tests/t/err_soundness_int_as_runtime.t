// The run-time check of `as`: -1 isn't a `u64`.
fn f(n: i64) -> u64 with panic { n as u64 }

fn main() with panic {
    print(f(-1));
}
// error: runtime error: `as` overflowed: -1 isn't a `u64`
