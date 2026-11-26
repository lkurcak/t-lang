# Step 101: floats

T has no floating-point types. They're overdue for ML, BLAS and graphics, and they're the first
stop on the ML/GPU path below.

A design step. The decisions to bring, each with a recommendation:

- `Eq`/`Ord` vs NaN: what `==` and `<` mean, and whether `f64` gets `Ord` at all (its laws
  fail with NaN);
- no float refinements at first;
- literals (`1.0`, `1e-3`, suffixes) and how they interact with the integer literal rules;
- formatting and parsing (`Display`, `Debug`, `str::parse`): round-tripping and shortest
  output;
- the C backend (IEEE semantics, no fast-math), the interpreter, and const eval.

## After floats

Interest raised 2026-11-26 (ML/AI, reinforcement learning, GPUs; inspired by Bend). The
machine has an NVIDIA RTX 5070 (12 GB).

Not Bend's model: running arbitrary code on the GPU needs a runtime like HVM2's
interaction nets, which goes against having no runtime. Instead, a *checked kernel
subset*: a kernel is ordinary T `with {}` (effects already exclude alloc, panic and io),
refinements prove the indices (no bounds checks on the device), and disjoint per-thread
writes are a typing rule.

1. Floats (this step).
2. A small CPU matrix/MLP library in t-libs, and an RL agent that learns a T game
   (t-maze, for example) as dogfood.
3. FFI with pointers, plus linking shared libraries (also unlocks graphics, windowing and
   sqlite).
4. `std::gpu` over the CUDA driver API (libcuda: `cuMemAlloc`, `cuModuleLoadData`,
   `cuLaunchKernel`).
5. `#[kernel]` fns compiled by the C backend to CUDA C, then NVRTC, then PTX.

The GPU comes later, once the CPU version of the RL agent is the bottleneck.
