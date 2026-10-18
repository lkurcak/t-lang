// A linear variable in a loop body, left behind by `continue` and by the
// end of the body.
struct Fd { n: int }

fn main() {
    let mut i = 0;
    while i < 3 {
        let _f = Fd { n: i };
        i += 1;
        if i == 2 {
            continue;
        }
    }
}
// error: `_f` is never used up, and its type, `Fd`, isn't `Discard`
