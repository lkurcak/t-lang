fn main() with panic {
    let o: Option<int> = Option::None;
    print(o.unwrap());
}
// error: runtime error: panicked
