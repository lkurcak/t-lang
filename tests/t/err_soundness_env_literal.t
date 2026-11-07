// Nor does any code make an `Env`.
fn main() {
    let env = std::env::Env {}; // error: `Env` is intrinsic: only its own functions make one or take one apart
    let _ = env.get("HOME");
}
