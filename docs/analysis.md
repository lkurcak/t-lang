# Semantic analysis and LSP snapshots

`check_source` and `check_files` return `Checked`: sources, warnings,
semantic information (including what checking learned before an error),
and either the first error or a `CheckedProgram`. Analysis includes type,
effect, refinement and borrow checking, along with the warning passes.
It leaves nested functions, local types and generics in the checked AST.

A `CheckedProgram` pairs that AST with its own semantic information. Its
`program()` and `info()` methods borrow them without permitting mutation.
`lower()` lifts and monomorphises it into `mono::Executable`, which the
interpreter accepts. The passes can also be called explicitly:
`lift::lift` accepts a `CheckedProgram` and returns `lift::Lifted`;
`mono::monomorphise` accepts that lifted program and returns an executable.
The wrappers prevent skipping a pass or mixing a program with another
program's checker information. Lowering preserves the analysis for reuse.

`t check` and the language server stop at analysis. Execution and `t mono`
lower only successful analyses. Renaming reuses the checked AST, but still
checks the proposed renamed program to reject capture and other changes
in meaning.

The language server caches analyses by program root and root discovery by
open document. Each snapshot records the inputs it actually read:

- The root and loaded module files, including reads that failed.
- Open documents' text, URIs and versions; an overlay takes precedence
  over the disk.
- Closed files' contents and canonical paths. Contents are compared on
  validation, so equal sizes or unchanged timestamps cannot hide edits.
- Candidate ancestor `main.t` files considered during root discovery,
  including missing candidates and modules read to decide membership.

Repeated requests validate those inputs and reuse the same analysis. A
changed input triggers full semantic rechecking; there is no fine-grained
incremental checking. Watch notifications and saves refresh diagnostics,
and requests also poll dependencies, so correctness does not depend on a
client watching disk files. Closing an overlay restores the file on disk.
Every open document's program is resolved before stale diagnostics are
cleared, so detaching or attaching a module cannot erase its new program's
diagnostics. Cached programs with no open documents are released.

Library navigation uses read-only copies of the binary's embedded T sources
in a private temporary directory per LSP server. The compiler keeps its
`<core>`, `<alloc>` and `<std>` source identities and spans; only the editor
URIs map to the copies. No paths or text from a compiler checkout are used,
and no wrappers or newline conversions change offsets. Definition and
type-definition results include these files; workspace edits and program
references exclude them.

Opening a library copy does not discover a program root or add disk inputs
to a program snapshot. All library buffers share a separate cached analysis
of an empty valid program, which reuses the already checked libraries.
Their text comes from the binary even if an editor sends an altered overlay.
Library buffers publish no diagnostics, offer no formatting or code actions,
and do not invalidate program analyses. The copies remain readable after the
server stops until the system temporary directory is cleared.

## Repeated-request measurement

Build both revisions with the same profile, then run
`python3 scripts/bench_lsp.py <path-to-t>` for each binary. The script
generates nine files with 105 functions, including generic helpers and
calls across modules. It opens the root and warms hover, hints and
references, then times five batches of 60 sequential requests, cycling
through those three methods. Initialization and the first analysis are outside the samples;
stdio, dependency validation and response generation are included.

Measured on macOS ARM64 with Rust 1.99.1 and unoptimised `cargo build`
binaries, comparing commit `5fb02cc` with this change:

| | Before | After |
|---|---:|---:|
| Median batch time (60 requests) | 10.317 s | 0.394 s |
| Median batch time per request | 171.95 ms | 6.56 ms |
| Checks during the 300 measured requests | 300 | 0 |
| Lowerings during the 300 measured requests | 300 | 0 |

That is about 26 times faster for this workload. Timing is informational;
tests assert reuse and invalidation, without wall-clock thresholds.

Before this change each measured request checked, lifted and
monomorphised the program again: 300 checks and 300 lowerings for the
measured requests. With snapshots those requests perform no semantic
checks or lowerings. The cache regression tests assert that requests and
different open files share the same analysis, including failed analyses,
and that changed inputs replace it.
