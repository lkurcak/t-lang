// An opaque type's bound can have effects, and pass on a parameter's.
fn adder(n: int) -> impl Fn(int) -> int with panic + Discard { move |x| x + n }

fn wrap<e, F: Fn(int) -> int with e + Discard>(f: F) -> impl Fn(int) -> int with e + Discard { move |x| f(x) }

fn main() with panic {
    let a = adder(2);
    print(a(1)); // expect: 3
    let w = wrap(|x| if x < 0 { 0 } else { x });
    print(w(5)); // expect: 5
}
