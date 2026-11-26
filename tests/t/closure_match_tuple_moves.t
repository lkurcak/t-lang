// A closure that takes apart a tuple or an array it captures, moving a
// value out of it that isn't `Copy`, captures it by value: before step
// 100, only a variant's or a struct's pattern said so, and a tuple's was
// taken to borrow, which refused the move.
#[derive(Discard)]
struct D { n: u8 }

fn take(d: D) -> u8 {
    let D { n } = d;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let t = (D { n: 4 }, 2u8);
    let c = || match t {
        (a, _) => take(a),
    };
    out.line(&c())?; // expect: 4
    let w = [D { n: 6 }];
    let e = || match w {
        [a] => take(a),
    };
    out.line(&e())?; // expect: 6
    Ok(())
}
