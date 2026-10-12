fn main() {
    let x = { let inner = 1; inner };
    print(inner);
}
// error: unknown variable `inner`
