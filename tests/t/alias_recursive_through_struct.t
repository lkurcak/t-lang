// A struct or enum breaks the cycle: this is the recursive type the
// error of `err_alias_cycle` asks for.
type Link = Option<alloc::boxed::Box<Node>>;
struct Node { n: i64, next: Link }

fn sum(l: &Link) -> i64 {
    match l {
        Option::Some(b) => b.n + sum(&b.next),
        Option::None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let l: Link = Option::Some(alloc::boxed::Box::new(Node { n: 2, next: Option::None }));
    out.line(&sum(&l))?; // expect: 2
    let Option::Some(b) = l else { return Result::Ok(()) };
    let Node { n: _, next: _ } = *b;
    Result::Ok(())
}
