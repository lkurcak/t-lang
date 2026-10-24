// `Copy` is a trait with no methods: a type that implements it is copied
// rather than moved. `#[derive(Copy)]` needs of the type parameters only
// what the fields need.

#[derive(Discard, Copy)]
struct Pair<A, B> { a: A, b: B }

// `Copy` for every `T`: a function is, whatever it takes.
#[derive(Discard, Copy)]
struct Callback<T> { f: fn(T) -> i64 }

#[derive(Discard, Copy)]
struct Celsius { degrees: i64 }

fn dup<T: Copy>(x: T) -> Pair<T, T> { Pair { a: x, b: x } }

fn unbox(b: Box<i64>) -> i64 { *b }

fn main() with {panic, alloc} {
    let p = dup(3i64);
    print(p.a + p.b);                                // expect: 6
    let c = Callback { f: unbox };
    let d = c;
    print((c.f)(Box::new(1)) + (d.f)(Box::new(2)));  // expect: 3
    let t = Celsius { degrees: 20 };
    let u = t;
    print(t.degrees + u.degrees);                    // expect: 40
    let q = dup(Pair { a: true, b: 7i64 });
    print(q.b.b);                                    // expect: 7
}
