// A variable a predicate in a closure names, a loop's invariant or a
// `let`'s refinement, is captured whole, by `&` if nothing else needs
// more: checking the predicate, and the interpreter checking the checker,
// read it there.
#[derive(Discard)]
struct P { n: u32, m: u32 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { n: 5, m: 2 };
    let count = || {
        let mut i: u32 = 0;
        while i < p.n invariant i <= p.n {
            i = i + 1;
        }
        i
    };
    out.line(&count())?; // expect: 5
    let q = P { n: 7, m: 3 };
    let small = || {
        let k: { v: u32 | v <= q.n } = q.m;
        k
    };
    out.line(&small()) // expect: 3
}
