// A closure generic over a lifetime can be returned as an `impl Fn` that
// says it is, and passed where a function type at one lifetime is wanted:
// it can be used at that one.
fn make(k: int) -> impl Fn(&int) -> &int + Discard {
    move |x: &int| -> &int { if k > 0 { x } else { x } }
}

fn at<'a>(f: fn(&'a int) -> &'a int, y: &'a int) -> int {
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
    let id = |x: &int| -> &int { x };
    print(at(id, &a)); // expect: 1
}
