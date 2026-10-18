// `break 'a` out of two loops leaves `_a` behind.
struct Fd { n: int }

fn main() {
    let x = 'a: loop {
        let _a = Fd { n: 10 };
        loop {
            break 'a 1;
        }
    };
    print(x);
}
// error: `_a` is never used up, and its type, `Fd`, isn't `Discard`
