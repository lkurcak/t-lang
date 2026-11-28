// An open bound's effect parameter isn't written in `::<..>`.
fn twice<T: Discard, F: FnMut() -> T + Discard>(mut f: F) -> (T, T) { (f(), f()) }

fn main() {
    let _ = twice::<u8, _, _>(|| 1u8);
}
// error: `twice` takes 2 type arguments but 3 were supplied
