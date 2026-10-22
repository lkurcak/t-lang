// A list's fields are private, so nothing outside `alloc::list` can make
// one whose `len` says slots hold values they don't.
use alloc::list::List;
use alloc::raw::RawBuf;

fn main() {
    let l: List<i64> = List { buf: RawBuf::empty(), len: 3 };
    print(*l.get(0).unwrap());
    l.free();
}
// error: field `buf` of `List` is private to `alloc::list`
