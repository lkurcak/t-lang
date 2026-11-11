// A refinement in an alias's argument goes where the parameter is in what
// the alias stands for: the `Err` of `R`'s `Result`, both elements of
// `Pair`'s tuple. (Up to 0.80.0 it went by where it was written: the `Ok`.)
type R<T> = Result<u8, T>;
type Pair<T> = (T, T);

fn need(_n: { n: u8 | n < 3 }) {}

fn f(x: R<{ v: u8 | v < 3 }>) -> u8 {
    match x {
        Ok(a) => a,
        Err(b) => {
            need(b);
            b
        },
    }
}

fn g(p: Pair<{ v: u8 | v < 3 }>) -> u8 {
    need(p.0);
    need(p.1);
    p.0 + p.1
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(Ok(200)))?; // expect: 200
    out.line(&f(Err(2)))?; // expect: 2
    out.line(&g((1, 2)))?; // expect: 3
    Ok(())
}
