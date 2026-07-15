# crayon

A WIP CPU emulator for the Cray-1 supercomputer.

## What it is

The Cray-1 (1976) was a 64-bit vector supercomputer designed by Seymour Cray. It had
a 12.5 ns clock period and twelve pipelined functional units. Its defining feature was
eight vector registers, each holding 64 64-bit elements — a single instruction could
operate on all of them at once. This is the same idea behind modern SIMD instruction
sets like AVX-512, predating them by decades.

## Architecture

**Registers**

| Name | Width | Count | Purpose |
|---|---|---|---|
| A0–A7 | 24-bit | 8 | Address registers — pointers, loop counters |
| S0–S7 | 64-bit | 8 | Scalar registers |
| V0–V7 | 64×64-bit | 8 | Vector registers |
| B00–B77 | 24-bit | 64 | Intermediate address (staging for A) |
| T00–T77 | 64-bit | 64 | Intermediate scalar (staging for S) |
| VL | 7-bit | 1 | Vector length (0–64) |
| VM | 64-bit | 1 | Vector mask |
| P | 24-bit | 1 | Parcel counter |

**Memory**

Up to 1,048,576 64-bit words in 16 interleaved banks. Word-addressed.

**Functional units**

| Unit | Latency |
|---|---|
| Address integer add | 2 clocks |
| Address multiply | 6 clocks |
| Scalar integer add | 3 clocks |
| Scalar logical | 1 clock |
| Scalar shift | 2–3 clocks |
| Scalar leading zero/pop count | 3–4 clocks |
| Vector integer add | 3 clocks |
| Vector logical | 2 clocks |
| Vector shift | 4 clocks |
| Floating point add | 6 clocks |
| Floating point multiply | 7 clocks |
| Floating point reciprocal | 14 clocks |

## Building

```
cargo build
```

## References

Cray Research, Inc. — [*The CRAY-1 Computer System*](https://s3data.computerhistory.org/brochures/cray.cray1.1977.102638650.pdf) (1977), publication number 2240008 8.
