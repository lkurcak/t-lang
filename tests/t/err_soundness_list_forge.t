// A list's fields are private, so nothing outside `alloc::list` can make
// one whose `len` says slots hold values they don't.
use alloc::list::List;
use alloc::raw::RawBuf;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let l: List<i64> = List { buf: RawBuf::empty(), len: 3 };
    out.line(&(*l.get(0).unwrap()))?;
    l.discard();
    Result::Ok(())
}
// error: field `buf` of `List` is private to `alloc::list`
