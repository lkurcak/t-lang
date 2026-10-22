// `main`'s obligations would be its caller's to keep, and nothing calls it.
use alloc::raw::RawBuf;

fn main() with {alloc, panic, unsafe} {
    let mut b: RawBuf<i64> = RawBuf::new(1);
    b.write(0, 1);
    print(b.read(0));
    b.free();
}
// error: `main` can't say `with unsafe`: nothing calls it to keep its obligations
