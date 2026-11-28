//! The generated families, a module each, in one test binary. Cargo runs
//! test binaries one after another but a binary's tests side by side, and
//! most families are a test or two, each checking its programs one after
//! another: each family a binary of its own kept all but a core or two
//! idle while it ran. Here every family's tests run at once.
//!
//! `cargo test --test generated captures::` runs one family.

mod assoc_unless;
mod bounds;
mod captures;
mod cond_effects;
mod contracts;
mod contradictions;
mod copies;
mod counting;
mod discard_when;
mod division;
mod entry;
mod exports;
mod field_where;
mod float_places;
mod floats;
mod for_loops;
mod inherit;
mod instantiation;
mod items;
mod joins;
mod laws;
mod learned;
mod loans;
mod math;
mod open_bounds;
mod order;
mod origins;
mod semantics;
mod soundness;
mod sources;
mod summaries;
mod unless;
mod values;

/// Each thread here checks programs one after another, allocating all the
/// while: with the system's allocator they wait for each other.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
