// `unwrap` panics on `None`, so the function calling it may panic.
fn first(o: Option<i64>) -> i64 { o.unwrap() }

fn main() with panic {
    print(first(Option::Some(1)));
}
// error: `unwrap` may panic, but `first` doesn't say `with panic`
