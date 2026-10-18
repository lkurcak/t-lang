// `let mut` variables can be changed, by assigning to them, to their
// fields, or through a `&mut` to them.

#[derive(Discard)]
struct Counter { count: int, step: int }

fn tick(c: &mut Counter) { c.count += c.step; }

fn exchange(a: &mut int, b: &mut int) {
    let t = *a;
    *a = *b;
    *b = t;
}

fn main() {
    let mut x = 1;
    x = x + 1;
    x *= 10;
    print(x);                   // expect: 20
    let mut c = Counter { count: 0, step: 3 };
    tick(&mut c);
    tick(&mut c);
    c.step = 1;
    tick(&mut c);
    print(c.count);             // expect: 7
    let mut a = 1;
    let mut b = 2;
    exchange(&mut a, &mut b);
    print(a);                   // expect: 2
    print(b);                   // expect: 1
    let r = &mut a;
    *r -= 5;
    print(a);                   // expect: -3
}
