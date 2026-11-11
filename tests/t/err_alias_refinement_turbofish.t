// Nor a type argument of a call.
type Square = { s: u8 | s < 64 };

fn id<T>(t: T) -> T { t }

fn main() {
    let _ = id::<Square>(1);
}
// error: `Square` stands for a refinement, which can only be
