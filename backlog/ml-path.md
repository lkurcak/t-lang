# The ML and GPU path


Interest raised 2026-11-26 (ML/AI, reinforcement learning, GPUs; inspired by Bend). The
machine has an NVIDIA RTX 5070 (12 GB).

Not Bend's model: running arbitrary code on the GPU needs a runtime like HVM2's
interaction nets, which goes against having no runtime. Instead, a *checked kernel
subset*: a kernel is ordinary T `with {}` (effects already exclude alloc, panic and io),
refinements prove the indices (no bounds checks on the device), and disjoint per-thread
writes are a typing rule.

1. Floats: done, 0.109.0.
2. Math functions in T, correctly rounded: done, 0.110.0 (`exp`, `ln`, `pow`, `sin`, `cos`,
   `tanh` and eight more, `docs/102-math.md`) and 0.111.0 (the inverses, `cbrt`, `hypot`,
   `sin_pi` and `cos_pi`, `docs/103-more-math.md`).
3. CPU float speed, measured before it's tuned: n-body, spectral-norm, mandelbrot, a dot
   product, and a naive and a tiled matrix multiply, in T, Rust and C on this machine; then
   fix what the numbers show. Rust has T's float rules (no fast-math, no fusing), so it's the
   fair comparison. Likely fixes: a target CPU option (`-O2` with no `-march` is SSE2 on
   x86-64, so no AVX and no FMA); a proved index's check given to the C compiler as an
   assumption, not a branch, so loops vectorise; `mul_add`, and sums in a fixed SIMD-shaped
   order ("Fast floats" in `BACKLOG.md`). Never fast-math. The math functions too: on
   0.110.0, `exp` takes 18 ns to glibc's 5 and `ln` 27 to 3, as each exact product is
   Dekker's 17 operations, where FMA makes it two; an `f32` runs its `f64`'s path, where one
   of its own, checked on every input, would do; and trigonometry above 2^20 reduces in
   fixed point, about a microsecond. Correct rounding means any of these can change without
   changing an answer.
4. MNIST: a matrix and neural-network library in t-libs, tested on MNIST, for numbers to
   compare with others: test accuracy and seconds per epoch, beside PyTorch on one CPU
   thread (library against library) and the same algorithm in plain Rust (language against
   language). Each model in turn is a check of what breaks: a linear model (about 92%), an
   MLP 784–128–10 (about 98%), a LeNet-style CNN (about 99%), a small residual net (skip
   connections); then Fashion-MNIST, the same format and harder. Supervised and seeded, so
   a wrong gradient shows as a wrong accuracy, unlike RL's noisy rewards.
   - Data: a script downloads and unzips it (T has no HTTP or gzip); T reads the IDX files.
   - Expected friction: backward passes by hand, or a small tape of operations; index
     proofs in convolutions over shapes known only at run time (`(c * h + y) * w + x` isn't
     linear) against fixed-size arrays, whose indices prove easily; a normal distribution
     for initial weights (Box–Muller needs `ln` and `cos`); `f32` throughout.
5. An RL agent that learns a T game (t-maze, for example), on the same library.
6. FFI with pointers, plus linking shared libraries (also unlocks graphics, windowing and
   sqlite).
7. `std::gpu` over the CUDA driver API (libcuda: `cuMemAlloc`, `cuModuleLoadData`,
   `cuLaunchKernel`).
8. `#[kernel]` fns compiled by the C backend to CUDA C, then NVRTC, then PTX, with the same
   rules as on the CPU: `--fmad=false` (CUDA fuses `a * b + c` by default) and T's own math
   functions rather than CUDA's approximate ones. Tensor cores (bf16 and fp16; NVIDIA doesn't
   fully say how they round or in what order they add) only through a named operation that
   promises a bound on its error, not exact bits.

The GPU comes later, once the CPU versions are the bottleneck.
