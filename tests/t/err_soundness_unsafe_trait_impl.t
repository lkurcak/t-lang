// An impl's method can do no more than its trait's says: callers through
// the trait see no `unsafe`.
use alloc::raw::RawBuf;

trait Take {
    fn take(&mut self) -> i64;
}

impl Take for RawBuf<i64> {
    fn take(&mut self) -> i64 with unsafe { self.read(0) }
}

fn main() {}
// error: `take` is unsafe, but `Take`'s `take` can't, so its impls can't either
