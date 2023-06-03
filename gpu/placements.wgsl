// The core of the search.
// Finds placements --- outgoing edges.
// This interface is well understood.  It is unlikely to improve much.
//
// Input:   1 bit field (filled cells)
// Output:  4 bit vectors of possible placements
//
// This implementation uses:
//    - u32 backing
//    - row-major order
//    - 3-row chunks
//    - most significant bit at top-right
// That is:
//     20 21 22 ... 27 28 29
//     10 11 12 ... 17 18 19
//      0  1  2 ...  7  8  9
//
// Other ways are possible:
//    - different backing (currently only u32 in WGSL)
//    - column-major order (easier code, much more memory given u32)
//    - different-size chunks (requires different backing)
//    - MSB top-left (inverts typical coordinate system used for kicks)

// Every placement function:
//    - runs `init` for every orientation
//    - initializes `dirty`
//    - looping until no chunks are dirty:
//      - runs `slide` for the chunk
//      - runs `kick` for every kick in every direction
//    - runs `placeable` for every orientation

// The magic constants are not magic.
// Well, they're a little bit magic.
// Here's how you can compute them in BQN.
//
// init:
//
//     UnBits ← +´⊢∧2⋆·↕≠   ⋄! 11 = UnBits 1‿1‿0‿1
//     draw ← [ "J"‿[1‿0‿0,1‿1‿1] ⋄ "L"‿[0‿0‿1,1‿1‿1] ⋄ "O"‿[1‿1,1‿1]
//              "S"‿[0‿1‿1,1‿1‿0] ⋄ "T"‿[0‿1‿0,1‿1‿1] ⋄ "Z"‿[1‿1‿0,0‿1‿1] ]
//
//     Rot  ← (UnBits·⥊3‿10↑⌽)¨(⍉⌽)⍟(↕4)
//     Mask ← 4⥊·(UnBits 30⥊10↑⥊⟜1)¨11-⌽∘≢
//
//     # I is special because it's long and requires 31 bits
//     i_s ← "I" ⋈ UnBits¨ 4⥊⟨4⥊1, ⥊4‿10↑4‿1⥊1⟩
//     i_m ← "I" ⋈ 4⥊UnBits¨30⊸⥊¨⟨10↑7⥊1, 10⥊1⟩
//
//     shapes ← •Show i_s∾ Rot¨ ⌾(1⊏⍉) draw
//     masks  ← •Show i_m∾ Mask¨⌾(1⊏⍉) draw
//
// kick tables:
//
//     Kick ← { x‿y: 0⊸≤◶ ⟨0,30+shift,10+x⟩‿⟨1,shift,10+x⟩ shift←x+10×y }
//     Combine ← {UnBits∾(⊑𝕩)∾⊑⟜(⌽¨⥊↕5⥊2)¨1↓𝕩}
//
//     •Show Combine∘Kick¨ ⟨1‿¯1, 0‿¯1, 0‿0, 1‿¯3, 0‿¯3⟩



const chunk_count : u32 = 2;   // configurable, 2 ≤ chunk_count ≤ 7

alias Field = array<u32, chunk_count>;
// PField (*padded field*) has an extra chunk on top.
// This simplifies the initialization logic.
// More padding is possible but it doesn't seem worthwhile.
alias PField = array<u32, chunk_count + 1>;

// For reference:
const NORTH : u32 = 0;
const EAST  : u32 = 1;
const SOUTH : u32 = 2;
const WEST  : u32 = 3;



struct PlacementMachine {
    dirty : u32,
    navigable : array<PField, 4>,
    viable    : array<PField, 4>,
}
alias Machine = ptr<function, PlacementMachine>;
const init_dirty : u32 = 0xF << (4 * chunk_count);   // the padding chunks



// Initialize a machine for the given orientation.
//
//   m     - machine to initialize
//   f     - field to place into
//   o     - orientation to initialize
//   piece - bit field of the piece (but bit 30 is okay to set)
//   mask  - bit vector of positions which are horizontally in bounds

fn init(m: Machine, f: Field, o: u32, piece: u32, mask: u32) {
    for (var i = 0u; i < chunk_count - 1; i++) {
        // iterate over trailing bits
        for (var piece = piece; piece != 0; piece &= piece - 1) {
            let idx = firstTrailingBit(piece);
            (*m).viable[o][i] |= f[i] >> idx;
            (*m).viable[o][i] |= f[i+1] << (30 - idx);
        }

        (*m).viable[o][i] = mask & ~(*m).viable[o][i];
    }

    // f[chunk_count] would be out of bounds
    {
        let i = chunk_count - 1;
        for (var piece = piece; piece != 0; piece &= piece - 1) {
            let idx = firstTrailingBit(piece);
            (*m).viable[o][i] |= f[i] >> idx;
        }

        (*m).viable[o][i] = mask & ~(*m).viable[o][i];
    }

    // f[chunk_count..] all out of bounds
    (*m).viable[o][chunk_count] = mask; // mask & ~0 == mask

    (*m).navigable[o][chunk_count] = mask;
}



// Flood fill a single chunk.
//
// Assumes `navigable` is a subset of `viable`.

fn flood_fill(navigable: u32, viable: u32) -> u32 {
    var prev = viable;

    // could also unroll
    loop {
        let down  = ((prev >> 10) & navigable) | prev;
        let left  = (down << 1) & 0x3FEFFBFE;
        let right = (down >> 1) & 0x1FF7FDFF;
        let next = (down | left | right) & navigable;

        if next == prev { return next; }

        prev = next;
    }
}

// Navigate within a chunk by sliding, and navigate to the next chunk down.
//
//   m - machine to update
//   o - orientation
//   h - height of (upper) chunk

fn slide(m: Machine, o: u32, h: u32) {
    let c = flood_fill((*m).navigable[o][h], (*m).viable[o][h]);
    (*m).navigable[o][h] = c;
    // this chunk is now dirty, but we knew it was dirty already

    if h > 0 {
        // slide down into next chunk
        let prev = (*m).navigable[o][h - 1];
        let c = (c << 20) & (*m).viable[o][h - 1];
        if (prev & c) != prev {
            (*m).navigable[o][h - 1] |= c;
            (*m).dirty |= 1u << ((h << 2) | o);
        }
    }
}



// Perform a kick from a single chunk into two other chunks.
//
//   m      - machine to update
//   height - height of the original chunk
//   orient - final orientation
//   chunk  - updated to remove successful kicks
//   id     - bit-packed:
//     bit     0: is this an upward kick?
//                0 - no, downward
//                1 - yes, upward
//     bits  1-5: how much to bit-shift left for the lower chunk
//     bits 6-10: 10 more than the horizontal shift
//                0 - (-10, _), shift 10 left
//                1 - ( -9, _), shift  9 left
//               10 - (  0, _), no horizontal shift
//               20 - (+10, _), shift 10 right

fn kick(
    m:      Machine,
    height: u32,
    orient: u32,
    chunk:  ptr<function, u32>,
    id:     u32,
) {
    const masks = array(0x0u, 0x20080200, 0x300c0300, 0x380e0380, 0x3c0f03c0,
        0x3e0f83e0, 0x3f0fc3f0, 0x3f8fe3f8, 0x3fcff3fc, 0x3feffbfe,
        0x3fffffff, 0x1ff7fdff, 0xff3fcff, 0x7f1fc7f, 0x3f0fc3f, 0x1f07c1f,
        0xf03c0f, 0x701c07, 0x300c03, 0x100401, 0x0);

    let up = id & 1;
    let h_upper = height + up;
    let h_lower = h_upper - 1;

    let shift_lower = (id >> 1) & 31;
    let shift_upper = 30 - shift_lower;

    let c = *chunk & masks[id >> 6];

    // lower
    if h_upper > 0 {
        let prev = (*m).navigable[orient][h_lower];
        let next = (c << shift_lower) & (*m).viable[orient][h_lower];

        if (prev & next) != prev {
            (*m).navigable[orient][h_lower] |= next;
            (*m).dirty |= 1u << (4*h_lower + orient);
        }
        *chunk &= ~(next >> shift_lower);
    } else {
        // all kicks went into the floor
        // try them again; don't update `chunk`
    }

    // upper
    if h_upper < chunk_count {
        let prev = (*m).navigable[orient][h_upper];
        let next = (c >> shift_upper) & (*m).viable[orient][h_upper];

        if (prev & next) != prev {
            (*m).navigable[orient][h_upper] |= next;
            (*m).dirty |= 1u << (4*h_upper + orient);
        }
        *chunk &= ~(next << shift_upper);
    } else {
        // kicking upwards; all in-bounds kicks were successful
        *chunk &= ~c;
    }
}



// Tear down a machine, find which navigable positions are placeable.

fn placeable(m: Machine, orient: u32) -> Field {
    var out = Field();

    let unsupported = (*m).viable[orient][0] << 10;
    out[0] = (*m).navigable[orient][0] & ~unsupported;

    for (var i = 1u; i < chunk_count; i++) {
        let unsupported =
            ((*m).viable[orient][i]     << 10) |
            ((*m).viable[orient][i - 1] >> 20);
        out[i] = (*m).navigable[orient][i] & ~unsupported;
    }

    return out;
}



///////////////////////
// Example placement //
///////////////////////



// Find all possible placements for the T piece in a given field.

fn place1_T(f: Field) -> array<Field, 4> {
    var machine = PlacementMachine();

    init(&machine, f, 0, 0x807,    0xFF3FCFF);  // T north
    init(&machine, f, 1, 0x100C01, 0x1FF7FDFF); // T east
    init(&machine, f, 2, 0x1C02,   0xFF3FCFF);  // T south
    init(&machine, f, 3, 0x200C02, 0x1FF7FDFF); // T west

    machine.dirty = init_dirty;

    // kick table
    // currently inlined into function; could use giant table for all pieces
    const k_90 = array(
        array(746u, 680u, 641u, 706u, 640u), // ne
        array(634u, 641u, 680u, 615u, 681u), // es
        array(641u, 707u, 727u, 660u, 726u), // sw
        array(661u, 595u, 634u, 701u, 635u), // wn
    );
    // if e.g. no 180 kicks, k_180 should not be generated at all
    const k_180 = array(
        array(680u, 641u, 707u, 634u, 746u, 614u), // ns
        array(634u, 641u, 681u, 661u, 615u, 595u), // ew
        array(661u, 641u, 634u, 707u, 595u, 727u), // sn
        array(707u, 641u, 681u, 661u, 747u, 727u), // we
    );
    const k_270 = array(
        array(680u, 746u, 707u, 640u, 706u), // nw
        array(595u, 661u, 641u, 635u, 701u), // en
        array(707u, 641u, 661u, 726u, 660u), // se
        array(641u, 634u, 614u, 681u, 615u), // ws
    );

    while machine.dirty != 0 {
        let b = firstLeadingBit(machine.dirty);
        let height = b >> 2;

        let r_0   =  b      & 3;
        let r_90  = (b + 1) & 3;
        let r_180 = (b + 2) & 3;
        let r_270 = (b + 3) & 3;

        slide(&machine, r_0, height);
        var k: u32;

        // looks a bit ridiculous, but it's easy to generate
        k = machine.navigable[r_0][height];
        kick(&machine, height, r_90, &k, k_90[r_0][0]);
        kick(&machine, height, r_90, &k, k_90[r_0][1]);
        kick(&machine, height, r_90, &k, k_90[r_0][2]);
        kick(&machine, height, r_90, &k, k_90[r_0][3]);
        kick(&machine, height, r_90, &k, k_90[r_0][4]);

        k = machine.navigable[r_0][height];
        kick(&machine, height, r_180, &k, k_180[r_0][0]);
        kick(&machine, height, r_180, &k, k_180[r_0][1]);
        kick(&machine, height, r_180, &k, k_180[r_0][2]);
        kick(&machine, height, r_180, &k, k_180[r_0][3]);
        kick(&machine, height, r_180, &k, k_180[r_0][4]);
        kick(&machine, height, r_180, &k, k_180[r_0][5]);

        k = machine.navigable[r_0][height];
        kick(&machine, height, r_270, &k, k_270[r_0][0]);
        kick(&machine, height, r_270, &k, k_270[r_0][1]);
        kick(&machine, height, r_270, &k, k_270[r_0][2]);
        kick(&machine, height, r_270, &k, k_270[r_0][3]);
        kick(&machine, height, r_270, &k, k_270[r_0][4]);

        machine.dirty &= ~(1u << ((height << 2) | r_0));
    }

    return array(
        placeable(&machine, 0),
        placeable(&machine, 1),
        placeable(&machine, 2),
        placeable(&machine, 3),
    );
}
