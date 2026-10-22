// A closure generic over a lifetime can be returned as an `impl Fn` that
// says it is, and passed where a function type at one lifetime is wanted:
// it can be used at that one.
fn make(k: i64) -> impl Fn(&i64) -> &i64 + Discard {
    move |x: &i64| -> &i64 { if k > 0 { x } else { x } }
}

fn at<'a>(f: fn(&'a i64) -> &'a i64, y: &'a i64) -> i64 {
    *f(y)
}

fn main() {
    let f = make(1);
    let a = 1;
    {
        let b = 2;
        print(*f(&b)); // expect: 2
    }
    print(*f(&a)); // expect: 1
    let id = |x: &i64| -> &i64 { x };
    print(at(id, &a)); // expect: 1
}
