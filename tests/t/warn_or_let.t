#[derive(Discard)]
enum W { Only(i64) }
fn main() {
    let W::Only(x) | W::Only(x) = W::Only(1);
    print(x);
}
// warning: unreachable alternative
// expect: 1
