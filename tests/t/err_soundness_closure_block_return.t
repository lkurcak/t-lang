// The same with a block.
struct Fd { n: i64 }

fn main() {
    let k = || {
        return 1i64;
    };
    let _fd = Fd { n: 1 };
    let _x = k();
}
// error: `_fd` is never used up
