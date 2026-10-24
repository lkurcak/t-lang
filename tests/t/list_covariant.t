// A list owns its values, as its buffer does, so it's covariant: a list of
// `&'static i64`s will do as one of shorter-lived references.
use alloc::list::List;
use alloc::string::String;
use core::fmt::Write;

fn shorten<'a>(l: List<&'static i64>) -> List<&'a i64> { l }

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let n = 7;
    let mut l: List<&i64> = shorten(List::new());
    l.push(&n);
    out.line(&(**l.get(0).unwrap())); // expect: 7
    l.discard();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
