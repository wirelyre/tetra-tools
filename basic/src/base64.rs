//! Encoding for bit vectors of arbitrary length (not just multiples of 8).
//!
//! This encoding scheme uses at most one byte more than conventional Base64
//! schemes for any input.  It also uses only URL-safe characters, following
//! RFC 4648 §5, but using '.' as an extra character rather than '='.

use bitvec::prelude::*;

const ENCODE_TABLE: [u8; 64] = [
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'b', b'c', b'd', b'e', b'f',
    b'g', b'h', b'i', b'j', b'k', b'l', b'm', b'n', b'o', b'p', b'q', b'r', b's', b't', b'u', b'v',
    b'w', b'x', b'y', b'z', b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'-', b'_',
];
const DECODE_TABLE: [u8; 256] = [
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 62, 255, 255, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 255,
    255, 255, 255, 255, 255, 255, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18,
    19, 20, 21, 22, 23, 24, 25, 255, 255, 255, 255, 63, 255, 26, 27, 28, 29, 30, 31, 32, 33, 34,
    35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
    255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
];

pub fn base64_encode(mut slice: &BitSlice, into: &mut String) {
    while slice.len() >= 6 {
        let bits = slice[..6].load_le::<usize>();
        slice = &slice[6..];

        into.push(ENCODE_TABLE[bits] as char);
    }

    if slice.len() > 0 {
        into.push('.');

        let tail = match slice.len() {
            5 => slice.load_le::<usize>(),
            4 => slice.load_le::<usize>() + 32,
            3 => slice.load_le::<usize>() + 32 + 16,
            2 => slice.load_le::<usize>() + 32 + 16 + 8,
            1 => slice.load_le::<usize>() + 32 + 16 + 8 + 4,
            _ => unreachable!(),
        };

        into.push(ENCODE_TABLE[tail] as char);
    }
}

pub fn base64_decode(encoded: impl AsRef<[u8]>) -> Option<BitVec> {
    let mut v = BitVec::new();
    let mut bytes = encoded.as_ref().iter().copied();

    for b in &mut bytes {
        if b == b'.' {
            let tail = DECODE_TABLE[bytes.next()? as usize];

            if bytes.next().is_some() {
                return None; // tail too long
            }

            let (bits, size) = match tail {
                0..=31 => (tail, 5),
                32..=47 => (tail - 32, 4),
                48..=55 => (tail - 48, 3),
                56..=59 => (tail - 56, 2),
                60..=61 => (tail - 60, 1),
                _ => return None, // (62, 63, or 255) invalid tail
            };

            v.extend_from_bitslice(&bits.view_bits::<Lsb0>()[..size]);

            break;
        }

        let decoded = DECODE_TABLE[b as usize];
        if decoded == 255 {
            return None; // invalid character
        }

        v.extend_from_bitslice(&decoded.view_bits::<Lsb0>()[..6]);
    }

    Some(v)
}

#[cfg(test)]
mod tests {
    use bitvec::prelude::*;

    use crate::base64::{base64_decode, base64_encode};

    fn round_trip(length: usize, before: &BitSlice) {
        assert_eq!(length, before.len());

        let mut s = String::new();
        base64_encode(before, &mut s);

        let after = base64_decode(&s).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn valid() {
        let mut len_64 = BitVec::new();
        len_64.extend(b"ABC DEF ".view_bits::<Lsb0>());

        let mut len_65 = len_64.clone();
        len_65.push(true);

        // no tail
        round_trip(0, &bitvec![]);
        round_trip(8, &bitvec![0, 1, 0, 1, 0, 1, 0, 1]);
        round_trip(64, &len_64);

        // has tail
        round_trip(1, &bitvec![1]);
        round_trip(65, &len_65);
    }

    #[test]
    fn all_tails() {
        for len in 0..6 {
            for val in 0..32usize {
                round_trip(len, &val.view_bits::<Lsb0>()[..len]);
            }
        }
    }

    #[test]
    fn invalid() {
        assert!(base64_decode("~").is_none()); // illegal character

        assert!(base64_decode("A=").is_none()); // tail too short
        assert!(base64_decode("A=AA").is_none()); // tail too long

        assert!(base64_decode("A=_").is_none()); // invalid tail
    }
}
