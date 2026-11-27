# The ML and GPU path


Interest raised 2026-11-26 (ML/AI, reinforcement learning, GPUs; inspired by Bend). The
machine has an NVIDIA RTX 5070 (12 GB).

Not Bend's model: running arbitrary code on the GPU needs a runtime like HVM2's
interaction nets, which goes against having no runtime. Instead, a *checked kernel
subset*: a kernel is ordinary T `with {}` (effects already exclude alloc, panic and io),
refinements prove the indices (no bounds checks on the device), and disjoint per-thread
writes are a typing rule.

1. Floats: done, 0.109.0.
2. Math functions in T (`exp`, `ln`, `pow`, `sin`, `cos`, `tanh`), the same on every
   machine, which libm isn't.
3. A small CPU matrix/MLP library in t-libs, and an RL agent that learns a T game
   (t-maze, for example) as dogfood.
4. FFI with pointers, plus linking shared libraries (also unlocks graphics, windowing and
   sqlite).
5. `std::gpu` over the CUDA driver API (libcuda: `cuMemAlloc`, `cuModuleLoadData`,
   `cuLaunchKernel`).
6. `#[kernel]` fns compiled by the C backend to CUDA C, then NVRTC, then PTX.

The GPU comes later, once the CPU version of the RL agent is the bottleneck.
