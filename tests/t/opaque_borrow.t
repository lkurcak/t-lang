// The type a function returns may borrow what its lifetimes do: `reader`'s
// closure holds `x`, so the result can't outlive what `x` points to.
fn reader<'a>(x: &'a int) -> impl Fn() -> int with panic + Discard { move || *x }

fn pick<'a>(x: &'a int) -> impl Fn() -> &'a int + Discard { move || x }

fn both<'a, 'b>(x: &'a int, y: &'b int) -> impl Fn() -> int with panic + Discard { move || *x + *y }

// Neither `x` nor `y` lives as long as the other is known to, but `'a: 'b`
// says `x` does: the closure holds a `&'b int`.
fn either<'b, 'a: 'b>(c: bool, x: &'a int, y: &'b int) -> impl Fn() -> int with panic + Discard {
    let r = if c { x } else { y };
    move || *r
}

fn bump(r: &mut int) -> impl FnMut() + Discard { move || *r += 1 }

fn main() with panic {
    let n = 3;
    let r = reader(&n);
    print(r());                     // expect: 3
    let m = 4;
    print(both(&n, &m)());          // expect: 7
    print(*pick(&n)());             // expect: 3
    print(either(false, &n, &m)()); // expect: 4
    let mut k = 0;
    let mut b = bump(&mut k);
    b();
    b();
    print(k);                       // expect: 2
}
