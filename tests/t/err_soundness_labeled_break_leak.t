// `break 'a` out of two loops leaves `_a` behind.
struct Fd { n: i64 }

fn main() {
    let x = 'a: loop {
        let _a = Fd { n: 10 };
        loop {
            break 'a 1i64;
        }
    };
    print(x);
}
// error: `_a` is never used up, and its type, `Fd`, isn't `Discard`
