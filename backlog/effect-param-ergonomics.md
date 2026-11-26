# Effect parameters for closure helpers

From t-fmt's FRICTION.md (the top item). A generic helper that takes a closure has to write
`effect e`, `F: FnMut(..) with e + Discard` and `T: Discard`. Once `with e` is written, the
fn's own effects stop being inferred, so they have to be spelled out in full as
`with {alloc, panic, e}`.

Options, each to bring with a recommendation:

- a private fn infers the rest of its effects beside a written `e`;
- an unannotated `F: FnMut(..)` bound in a private fn gets an effect parameter inferred
  for it.

Both extend effect inference for private fns (step 24: `with` only on `pub`). The question
is what stays checkable signature-only at module and package boundaries.
