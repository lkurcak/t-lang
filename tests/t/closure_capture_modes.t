// What a closure borrows, and for how long.

#[derive(Discard)]
struct P { x: int, y: int }

fn main() {
    // Borrows end with the closure's last use.
    let mut n = 0;
    let mut inc = || n = n + 1;
    inc();
    inc();
    print(n);                       // expect: 2
    // Several closures may read the same variable.
    let p = P { x: 1, y: 2 };
    let fx = || p.x;
    let fy = || p.y;
    print(fx() + fy());             // expect: 3
    // Nested closures capture through the outer one.
    let g = || { let h = || p.x * 10; h() };
    print(g());                     // expect: 10
    // `move` copies `Copy` values in: changing `m` afterwards doesn't
    // change the closure's.
    let mut m = 1;
    let get = move || m;
    m = 5;
    print(get() + m);               // expect: 6
    // A closure is `Copy` if what it captures is: here a `&P`.
    let f = || p.x;
    let f2 = f;
    print(f() + f2());              // expect: 2
}
