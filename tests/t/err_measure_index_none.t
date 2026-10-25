// `x[i]` is `*x.index(i)`: a type without an `index` method can't be indexed.
#[derive(Copy, Discard)]
struct W { n: usize }

fn main() {
    let w = W { n: 1 };
    let x = w[0];
}
// error: `W` has no method `index`
