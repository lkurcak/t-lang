// A variable has one type. `id` is `fn(int) -> int` once it's called with an
// int, so it can't take a bool too. That takes generics (step 4).
fn main() {
    let id = |x| x;
    print(id(1));
    print_bool(id(true));
}
// error: expected `int`, found `bool`
