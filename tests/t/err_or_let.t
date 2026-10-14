enum W { Only(int) }
fn main() {
    let W::Only(x) | W::Only(x) = W::Only(1);
}
// error: unreachable alternative
