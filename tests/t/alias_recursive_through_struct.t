// A struct or enum breaks the cycle: this is the recursive type the
// error of `err_alias_cycle` asks for.
type Link = Option<alloc::boxed::Box<Node>>;
#[derive(Discard)]
struct Node { n: i64, next: Link }

fn sum(l: &Link) -> i64 {
    match l {
        Some(b) => b.n + sum(&b.next),
        None => 0,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let l: Link = Some(alloc::boxed::Box::new(Node { n: 2, next: None }));
    out.line(&sum(&l))?; // expect: 2
    Ok(())
}
