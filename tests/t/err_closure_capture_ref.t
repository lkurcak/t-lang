fn main() {
    let n = 1;
    let r = &n;
    let get = || *r;
    print(get());
}
// error: this closure can't capture `r`: it's a `&int`, which holds a reference
