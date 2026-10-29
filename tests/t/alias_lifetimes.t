// An alias can take lifetime parameters. Left out where it's used, they're
// elided, as a struct's are.
use alloc::arena::Arena;
use core::fmt::Write;

#[derive(Discard)]
enum Expr<'a> { Num(i64), Add(&'a Expr<'a>, &'a Expr<'a>) }
type Nodes<'a> = Arena<Expr<'a>>;
type Ref<'a, T> = &'a T;
type Forever = &'static str;

fn eval<'a>(e: Ref<'a, Expr<'a>>) -> i64 {
    match e {
        Expr::Num(n) => *n,
        Expr::Add(a, b) => eval(a) + eval(b),
    }
}

fn build<'a>(nodes: &'a Nodes<'a>) -> &'a Expr<'a> with alloc {
    let one = nodes.alloc(Expr::Num(1));
    let two = nodes.alloc(Expr::Num(2));
    nodes.alloc(Expr::Add(one, two))
}

fn label(n: Ref<i64>) -> Forever {
    match *n > 0 {
        true => "positive",
        false => "not",
    }
}

// Elided: the one lifetime of the parameters.
fn pick(a: Ref<i64>) -> Ref<i64> { a }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let nodes: Nodes = Arena::new();
    out.line(&eval(build(&nodes)))?; // expect: 3
    let s: Forever = "hi";
    out.write_str(s)?;
    out.line(&0i64)?; // expect: hi0
    out.write_str(label(&3))?;
    out.line(&0i64)?; // expect: positive0
    let n = 7i64;
    out.line(pick(&n))?; // expect: 7
    Result::Ok(())
}
