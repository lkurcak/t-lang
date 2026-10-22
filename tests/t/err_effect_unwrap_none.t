fn main() with panic {
    let o: Option<i64> = Option::None;
    print(o.unwrap());
}
// error: runtime error: panicked
