# Plan

How to make SRS run on the GPU using WebGPU.

## Program entry points

- Run separate commands for `place_I`, `place_J`, *etc.*
- Two kinds of output:
  - **Trace:** Collection of all visited bitboards with successors
  - **Solve:** Collection of solutions (list of pieces)
- For now, JavaScript to choreograph everything

## Commentary

- Kicks written directly in program
  - Compiler might be able to inline if it's smart
  - Uploading a new shader for different physics is cheap
  - Need separate commands for this (`place_*`)
    - Means the CPU has to use a single actual piece queue

- WebGPU only has u32 for now
  - Solved problem for SRS: use 3-line chunks

- Tracing might be useless for this problem

- Want deterministic output
  - Sort

- Need several library functions
  - Prefix sum
  - Sort
    - Radix? Merge?

- Should keep data on GPU if possible

- How much memory to allocate?
  - Could communicate back to CPU on each step
    - Sounds slow but who knows
  - Could optimistically use a single big allocation
    - Then give an error at the end of the pass if need more

- Need *very good idea* of internal state to write any code

## Data structures

```
struct Field {
    cells : array<u32; 2>,
    complete : u32,   // which rows are complete
}

struct Solution {
    field : Field,
    pieces : array<u32; 5>,   // packed; see below
}

struct Piece {
    kind: u3,
    rows: u4,
    col: u4,
}
// only works for 4-line solutions
// other packings might be possible, otherwise `rows: u16` should cover it
```

## Interfaces

```
fn trace_i(array<Field>): array<Field>
fn trace_j
...

fn solve_i(array<Solution>, trace: array<Field>): array<Solution>
fn solve_j
...

// or
fn solve_i(array<Solution>): array<Solution>
// or
fn solve_i(array<Solution>, trace: bloom_filter<Field>): array<Solution>
```
