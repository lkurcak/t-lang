enum Shape { Dot, Line }
type S = Shape;

fn main() {
    let _s = S::Dot;
}
// error: `S` is a type alias, so it can only be used as a type; here, write what it stands for: `Shape`
