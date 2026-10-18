// Variables bound by patterns can be `mut` too.

#[derive(Discard)]
struct P { x: int, y: int }

#[allow(dead_code)]
#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main() {
    let P { x: mut a, y } = P { x: 1, y: 2 };
    a += y;
    print(a);                       // expect: 3
    match Option::Some(5) {
        Option::Some(mut n) => {
            n *= 2;
            print(n);               // expect: 10
        },
        Option::None => {},
    }
}
