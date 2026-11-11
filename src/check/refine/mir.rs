//! The refinement checker on MIR (docs/65-refinements-on-mir.md).

use super::*;

impl Checker {
    /// The walk over the MIR of the body of `f`, and its closures'.
    pub(super) fn walk_mir(&self, f: &FnDecl) -> Walked {
        self.walk_ast(f)
    }
}
