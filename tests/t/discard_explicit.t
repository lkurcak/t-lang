// `x.discard()` throws `x` away now, rather than at the end of its scope:
// for any type that's `Discard`, built in, derived, or with a `discard` of
// its own, and through a bound.
use alloc::list::List;
use alloc::string::String;

#[derive(Discard)]
struct Pair { a: String, b: i64 }

fn toss<T: Discard>(x: T) { x.discard() }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let s = String::from("a");
    let n = s.len();
    s.discard();
    let five: u8 = 5;
    five.discard();
    Pair { a: String::from("b"), b: 1 }.discard();
    let mut l: List<String> = List::new();
    l.push(String::from("c"));
    toss(l);
    toss(String::from("d"));
    out.line(&n) // expect: 1
}
