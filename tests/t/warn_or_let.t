enum W { Only(int) }
fn main() {
    let W::Only(x) | W::Only(x) = W::Only(1);
    print(x);
}
// warning: unreachable alternative
// expect: 1
