// `+` panics when it overflows, so a function that adds must say it may
// panic: `fn main()` with no `with` promises the program never does.
fn next(n: int) -> int { n + 1 }

fn main() {
    print(next(9223372036854775807));
}
// error: `+` may panic, but `next` doesn't say `with panic`
