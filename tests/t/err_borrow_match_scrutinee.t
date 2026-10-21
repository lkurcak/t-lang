#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main() with panic {
    let mut o = Option::Some(1);
    match &mut o {
        Option::Some(n) => {
            o = Option::None;
            *n += 1;
        }
        Option::None => {}
    }
}
// error: cannot assign to `o` while `o` is mutably borrowed
