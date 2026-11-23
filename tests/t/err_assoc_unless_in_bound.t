// A bound on an associated effect says what it is at most; that can't
// hang on another bound (docs/96-associated-effects-unless.md).
trait Low {}

fn f<T, I: Iterator<e = {panic unless T: Low}>>(_i: I, _t: T) {}

fn main() {}
// error: only a function's own effects, or an impl's associated effect, can say `unless`, not a type's or a bound's
