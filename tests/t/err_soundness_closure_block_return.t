// The same with a block.
struct Fd { n: int }

fn main() {
    let k = || {
        return 1;
    };
    let _fd = Fd { n: 1 };
    let _x = k();
}
// error: `_fd` is never used up
