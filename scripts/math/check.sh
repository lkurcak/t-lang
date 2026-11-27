#!/bin/sh
# Checks one of core's math functions against MPFR (see check.c):
#
#     scripts/math/check.sh <fn> f32|f64 all|bits|range|range2|list ...
#
# It exports T's function to C, as `m_<fn>_<type>`, compiles that with
# check.c and MPFR, and runs it with the rest of the arguments. `$T` is the
# `t` to build with, `target/release/t` if not said. Needs MPFR and GMP.
set -eu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)
t=${T:-$root/target/release/t}
fn=$1
ty=$2
dir=$(mktemp -d "${TMPDIR:-/tmp}/t-math-XXXXXX")
trap 'rm -rf "$dir"' EXIT
# `$T_FILE`, if set, is a T program that exports them itself.
if [ -n "${T_FILE:-}" ]; then
    cp "$T_FILE" "$dir/exports.t"
else
case $fn in
pow | atan2 | hypot)
    echo "extern fn m_${fn}_$ty(x: $ty, y: $ty) -> $ty with {} { x.$fn(y) }" >"$dir/exports.t" ;;
*)
    echo "extern fn m_${fn}_$ty(x: $ty) -> $ty with {} { x.$fn() }" >"$dir/exports.t" ;;
esac
echo 'fn main() {}' >>"$dir/exports.t"
fi
"$t" build --emit-c "$dir/exports.t" >"$dir/exports.c"
# The flags `t build` compiles with (src/cgen.rs `compile`), and its
# `main` out of the way of check.c's.
cc -std=c11 -O2 -ffp-contract=off -fno-math-errno -pthread -Dmain=t_program_main \
    -c -o "$dir/exports.o" "$dir/exports.c"
cc -std=gnu11 -O2 -pthread -I"$here" -o "$dir/check" "$here/check.c" "$dir/exports.o" \
    -lmpfr -lgmp -lm
shift 2
"$dir/check" "$fn" "$ty" "$@"
