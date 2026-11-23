// An impl's own method for a generic type, `W<T>`, in place of a default
// that says `with panic unless Self: Small`, can't panic if any `W<..>`
// may be `Small`: here `W<u8>` is, and a call of `bump` at `W<u8>` has no
// `panic`, but runs this body (lemma B6 of docs/92-where-a-bound-holds.md;
// a hole in 0.105.0 and 0.106.0, which asked only whether `W<T>` itself
// matched an impl).
trait Small {
    fn size(&self) -> u8;
}

#[derive(Discard)]
struct W<T> { n: u8, t: T }

impl Small for W<u8> {
    fn size(&self) -> u8 { self.n }
}

trait Bump {
    fn bump(&self) -> u8 with panic unless Self: Small { 1 }
}

impl<T> Bump for W<T> {
    fn bump(&self) -> u8 with panic { self.n + 250 }
}

fn main() with {} {
    let w = W { n: 9, t: 0u8 };
    let _v = w.bump();
    w.discard();
}
// error: `+` may panic where `W<T>: Small`, but `bump` says `with panic unless W<T>: Small`
