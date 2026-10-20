// Closures can be stored, as long as what they borrow outlives them.
#[derive(Discard)]
struct Counter<F> { next: F, calls: int }

fn step<e, F: FnMut() -> int with e>(c: &mut Counter<F>) -> int with {panic, e} {
    c.calls = c.calls + 1;
    (c.next)()
}

fn main() with panic {
    let mut i = 0;
    let mut c = Counter { next: || { i = i + 10; i }, calls: 0 };
    print(step(&mut c));            // expect: 10
    print(step(&mut c));            // expect: 20
    print(c.calls);                 // expect: 2
    print(i);                       // expect: 20
}
