// TODO: also try e.g. SmallVec<[Kick; 6]> or ArrayVec<Kick, 6> or &[Kick]
//       or even fully inlined with an array of indices
pub struct Physics(pub [[Kicks; 4]; 7]);

/// Kick table for a single shape and orientation
pub struct Kicks {
    pub cw: Vec<Kick>,
    pub half: Vec<Kick>,
    pub ccw: Vec<Kick>,
}

pub struct Kick {
    pub up: bool,
    pub x: i8,
    pub y: u8,
}

pub trait Chunk: Copy + Clone {
    const LINES: usize;

    /// Shift this chunk left or right and **downwards** without teleporting.
    /// This corresponds to the upper chunk of a kick.
    fn upper_shift(self, x: i8, y: u8) -> Self;
    /// Shift this chunk left or right and **upwards** without teleporting.
    /// This corresponds to the lower chunk of a kick.
    fn lower_shift(self, x: i8, y: u8) -> Self;

    fn is_empty(self) -> bool;
    fn count_set(self) -> u32;

    fn update(self, other: &mut Self) -> bool;
}

pub struct Field<C: Chunk, const N: usize>(pub [C; N]);

impl Kick {
    pub const fn from_coords<C: Chunk>(x: i8, y: i8) -> Kick {
        if y > 0 || y == 0 && x >= 0 {
            Kick {
                up: true,
                x,
                y: y as u8,
            }
        } else {
            Kick {
                up: false,
                x,
                y: (C::LINES as i8 + y) as u8,
            }
        }
    }
}

/// Big boxes of bits:
///   - Each row looks like `0b0000000111` or `0b1110000000` for various
///     counts of 1 bits.
///   - Each row is identical.
/// These will be used as masks before shifting in chunk operations.
// TODO: U32_BOXES[-x] ≈ ~U32_BOXES[x]
static U32_BOXES: [u32; 19] = [
    0x20080200, 0x300c0300, 0x380e0380, 0x3c0f03c0, 0x3e0f83e0, 0x3f0fc3f0, 0x3f8fe3f8, 0x3fcff3fc,
    0x3feffbfe, 0x3fffffff, 0x1ff7fdff, 0xff3fcff, 0x7f1fc7f, 0x3f0fc3f, 0x1f07c1f, 0xf03c0f,
    0x701c07, 0x300c03, 0x100401,
];
static U64_BOXES: [u64; 19] = [
    0x802008020080200,
    0xc0300c0300c0300,
    0xe0380e0380e0380,
    0xf03c0f03c0f03c0,
    0xf83e0f83e0f83e0,
    0xfc3f0fc3f0fc3f0,
    0xfe3f8fe3f8fe3f8,
    0xff3fcff3fcff3fc,
    0xffbfeffbfeffbfe,
    0xfffffffffffffff,
    0x7fdff7fdff7fdff,
    0x3fcff3fcff3fcff,
    0x1fc7f1fc7f1fc7f,
    0xfc3f0fc3f0fc3f,
    0x7c1f07c1f07c1f,
    0x3c0f03c0f03c0f,
    0x1c0701c0701c07,
    0xc0300c0300c03,
    0x4010040100401,
];
/*
static U32_BOXES: [u32; 19] = {
    const fn one_row(i: i8) -> u32 {
        if i >= 0 {
            (1 << (10 - i)) - 1
        } else {
            ((1 << (10 + i)) - 1) << (-i)
        }
    }
    const fn whole_box(i: i8) -> u32 {
        let r = one_row(i);
        (r << 0) | (r << 10) | (r << 20)
    }
    const fn arr() -> [u32; 19] {
        let mut arr = [0; 19];
        let mut i = 0;
        while i < 19 {
            arr[i] = whole_box(i as i8 - 9);
            i += 1;
        }
        arr
    }
    arr()
};
*/
impl Chunk for u32 {
    const LINES: usize = 3;

    fn upper_shift(self, x: i8, y: u8) -> Self {
        debug_assert!(-9 <= x && x <= 9);
        debug_assert!(y <= 3);

        let amt = 30 - 10 * (y as i8) - x;
        debug_assert!(0 <= amt && amt <= 30);

        let b = U32_BOXES[(x + 9) as usize];
        (self & b) >> amt
    }

    fn lower_shift(self, x: i8, y: u8) -> Self {
        debug_assert!(-9 <= x && x <= 9);
        debug_assert!(y <= 3);

        let amt = 10 * (y as i8) + x;
        debug_assert!(0 <= amt && amt <= 30);

        let b = U32_BOXES[(x + 9) as usize];
        // TODO: currently clearing the top bits; might be unnecessary
        (self & b) << (amt + 2) >> 2
    }

    fn is_empty(self) -> bool {
        self == 0
    }

    fn count_set(self) -> u32 {
        self.count_ones()
    }

    fn update(self, other: &mut Self) -> bool {
        let did_change = *other | self != *other;
        *other |= self;
        did_change
    }
}
impl Chunk for u64 {
    const LINES: usize = 6;

    fn upper_shift(self, x: i8, y: u8) -> Self {
        debug_assert!(-9 <= x && x <= 9);
        debug_assert!(y <= 6);

        let amt = 60 - 10 * (y as i8) - x;
        debug_assert!(0 <= amt && amt <= 60);

        let b = U64_BOXES[(x + 9) as usize];
        (self & b) >> amt
    }

    fn lower_shift(self, x: i8, y: u8) -> Self {
        debug_assert!(-9 <= x && x <= 9);
        debug_assert!(y <= 6);

        let amt = 10 * (y as i8) + x;
        debug_assert!(0 <= amt && amt <= 60);

        let b = U64_BOXES[(x + 9) as usize];
        // TODO: currently clearing the top bits; might be unnecessary
        (self & b) << (amt + 4) >> 4
    }

    fn is_empty(self) -> bool {
        self == 0
    }

    fn count_set(self) -> u32 {
        self.count_ones()
    }

    fn update(self, other: &mut Self) -> bool {
        let did_change = *other | self != *other;
        *other |= self;
        did_change
    }
}

/*
    rotating up:   n.rotate_left(11)
    rotating down: n.rotate_right(9)

    up-kicks   are: [+1] rot_down / [0] rot_up
    down-kicks are: [0] rot_down / [+1] rot_up
    in either case, the upper half rotates down, and the lower half rotates up
        https://fumen.zui.jp/?v115@MgwhHeJ8whCeAtFeglCeAtDeglCeQpEeJ8EeQpheAg?H
*/

/*
fn one_kick(still_to_check: &mut u64, kick_num: usize) {
    /****** sameish for placement ******/
    let UP_KICK_VIABLE: u64 = todo!();
    let DOWN_KICK_VIABLE: u64 = todo!();

    /****** same for physics / chunk kind ******/
    let UP_KICK_MASK: u64 = todo!();
    let DOWN_KICK_MASK: u64 = todo!();
    let kick_x: i8 = 2;
    let kick_y: i8 = 1;
    let down_kick_shift: u32 = 10 * kick_y + kick_x;
    let up_kick_shift: u32 = 30 - down_kick_shift;

    /***** change during placement ******/
    let mut UP_KICK_REACHABLE: u64 = todo!();
    let mut DOWN_KICK_REACHABLE: u64 = todo!();
    let mut UP_KICK_DIRTY: bool = todo!();
    let mut DOWN_KICK_DIRTY: bool = todo!();

    /****** runtime ******/
    let successful_up: u64 = ((still_to_check & UP_KICK_MASK) >> up_kick_shift) & UP_KICK_VIABLE;
    let successful_down: u64 =
        ((still_to_check & DOWN_KICK_MASK) << down_kick_shift) & DOWN_KICK_VIABLE;

    {
        UP_KICK_DIRTY |= (UP_KICK_REACHABLE | successful_up) != UP_KICK_REACHABLE;
        UP_KICK_REACHABLE = (UP_KICK_REACHABLE | successful_up);

        DOWN_KICK_DIRTY |= (successful_down & DOWN_KICK_REACHABLE) != successful_down;
        DOWN_KICK_REACHABLE |= successful_down;
    }

    {
        if (UP_KICK_REACHABLE | successful_up) != UP_KICK_REACHABLE {
            UP_KICK_DIRTY = true;
            UP_KICK_REACHABLE = UP_KICK_REACHABLE | successful_up;
        }

        if todo!() {
            todo!() // same
        }
    }

    still_to_check =
        still_to_check & !(successful_up << up_kick_shift) & !(successful_down >> down_kick_shift);
}
*/
