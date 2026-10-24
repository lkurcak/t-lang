// Type arguments can be written out with `::<..>`, the "turbofish", where
// inference can't work them out or to say them anyway.

#[derive(Discard)]
enum Option<T> { Some(T), None }
#[derive(Discard)]
struct Pair<A, B> { first: A, second: B }
// `T` isn't in either of these signatures, so callers have to say it.
fn none<T>() -> Option<T> { Option::None }
fn count<T>(n: i64) -> i64 { n }
// A type with a parameter none of its fields use: a number tagged with
// what it counts.
#[derive(Discard)]
struct Id<T> { n: i64 }

fn id<T>(x: T) -> T { x }
fn second<A: Discard, B>(_a: A, b: B) -> B { b }
fn apply<T>(f: fn(T) -> T, x: T) -> T { f(x) }
fn is_some<T: Discard>(o: Option<T>) -> bool {
    match o {
        Option::Some(_) => true,
        Option::None => false,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(id::<i64>(1)))?;                                // expect: 1
    out.line(&is_some(Option::<i64>::None))?;           // expect: false
    out.line(&is_some(none::<bool>()))?;                // expect: false
    out.line(&(count::<Pair<i64, bool>>(3)))?;                 // expect: 3
    out.line(&apply(id::<i64>, 4))?;                         // expect: 4
    let p = Pair::<i64, bool> { first: 5, second: true };
    out.line(&p.first)?;                                     // expect: 5
    let users: Id<Pair<i64, i64>> = Id { n: 6 };
    out.line(&users.n)?;                                     // expect: 6
    let things = Id::<bool> { n: 7 };
    out.line(&things.n)?;                                    // expect: 7
    match Option::Some(8) {
        Option::<i64>::Some(n) => out.line(&n)?,             // expect: 8
        Option::<i64>::None => {}
    }
    let Pair::<i64, bool> { first, .. } = p;
    out.line(&first)?;                                       // expect: 5
    // `_` stands for a type to infer: write some type arguments, not all.
    out.line(&(second::<bool, _>(true, 9i64)))?;                  // expect: 9
    out.line(&unwrap(id::<Option<_>>(Option::Some(10))))?;   // expect: 10
    Result::Ok(())
}

fn unwrap(o: Option<i64>) -> i64 {
    match o {
        Option::Some(n) => n,
        Option::None => 0,
    }
}
