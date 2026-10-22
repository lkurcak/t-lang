fn main() {
    match true { true => 1, false => 2 }
    print(3);
}
// error: expected `()`, found `{integer}`: this `match` is a statement,
