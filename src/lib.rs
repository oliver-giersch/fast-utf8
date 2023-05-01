use core::mem;

const BYTES: usize = mem::size_of::<usize>();
const NONASCII_MASK: usize = usize::from_ne_bytes([0x80; BYTES]);

#[inline]
pub fn validate_utf8(buf: &[u8]) -> bool {
    // we check aligned blocks of up to 8 words at a time
    const ASCII_BLOCK_8X: usize = 8 * BYTES;
    const ASCII_BLOCK_4X: usize = 4 * BYTES;
    const ASCII_BLOCK_2X: usize = 2 * BYTES;

    let (mut curr, end) = (0, buf.len());
    // calculate the byte offset until the first word aligned block
    let start = buf.as_ptr();
    let align_offset = start.align_offset(BYTES);

    // calculate the maximum byte at which a block of size N could begin,
    // without taking alignment into account
    let block_end_8x = block_end(end, ASCII_BLOCK_8X);
    let block_end_4x = block_end(end, ASCII_BLOCK_4X);
    let block_end_2x = block_end(end, ASCII_BLOCK_2X);

    macro_rules! block_loop {
        ($flag:expr, $N:expr) => {
            // SAFETY: we have checked before that there are still at least
            // `N * size_of::<usize>()` in the buffer and that the current byte
            // is word-aligned
            let block = unsafe { &*(start.add(curr) as *const [usize; $N]) };
            if has_non_ascii_byte(block) {
                $flag = false;
                break;
            }

            curr += $N * BYTES;
        };
    }

    while curr < end {
        if buf[curr] < 128 {
            if align_offset == usize::MAX {
                curr += 1;
                continue;
            }

            // check if `curr`'s pointer is word-aligned
            let offset = align_offset.wrapping_sub(curr) % BYTES;
            // `align_offset` can basically only be `usize::MAX` for ZST
            // pointers, so the first check is most likely optimized away
            if offset == 0 {
                let mut ascii = true;
                // check 8-word blocks for non-ASCII bytes
                while curr < block_end_8x {
                    block_loop!(ascii, 8);
                }

                // check 4-word blocks for non-ASCII bytes
                if ascii {
                    while curr < block_end_4x {
                        block_loop!(ascii, 4);
                    }
                }

                // check 2-word blocks for non-ASCII bytes
                if ascii {
                    while curr < block_end_2x {
                        block_loop!(ascii, 2);
                    }
                }

                // if the block loops were stopped due to a non-ascii byte,
                // (otherwise this loop condition will be instantly false)
                // perform a 2-word blockwise calculation of the specific byte
                // in order to avoid having to check each byte individually
                // NOTE: this operation does not auto-vectorize well, so it is
                // done only in case a non-ascii byte is actually found
                while curr < block_end_2x {
                    // SAFETY: `curr` is word-aligned and the are enough bytes
                    // after `curr` for a 2-word block
                    let block = unsafe { &*(start.add(curr) as *const [usize; 2]) };
                    // skip the number of bytes that are definitely ASCII bytes
                    let (skip, non_ascii) = non_ascii_byte_position(block);
                    curr += skip;

                    if non_ascii {
                        break;
                    }
                }

                // ...otherwise, fall back to byte-wise checks
                while curr < end && buf[curr] < 128 {
                    curr += 1;
                }
            } else {
                // byte is < 128 (ASCII), but pointer is not word-aligned, skip
                // until the loop reaches a word-aligned block)
                for _ in 0..offset {
                    // no need to check alignment again, yet, skip `offset`
                    // valid ASCII bytes if possible
                    curr += 1;
                    if !(curr < end && buf[curr] < 128) {
                        break;
                    }
                }
            }
        } else {
            // non-ASCII case: validate up to 4 bytes, then advance `curr`
            // accordingly
            match validate_non_acii_bytes(buf, curr, end) {
                Some(next) => curr = next,
                None => return false,
            }
        }
    }

    true
}

#[inline(always)]
#[cold]
const fn validate_non_acii_bytes(buf: &[u8], mut curr: usize, end: usize) -> Option<usize> {
    macro_rules! next {
        () => {{
            curr += 1;
            // we needed data, but there was none: error!
            if curr >= end {
                return None;
            }
            buf[curr]
        }};
    }

    let byte = buf[curr];
    match utf8_char_width(byte) {
        2 => {
            if next!() as i8 >= -64 {
                return None;
            }
        }
        3 => {
            match (byte, next!()) {
                (0xE0, 0xA0..=0xBF)
                | (0xE1..=0xEC, 0x80..=0xBF)
                | (0xED, 0x80..=0x9F)
                | (0xEE..=0xEF, 0x80..=0xBF) => {}
                _ => return None,
            }

            if next!() as i8 >= -64 {
                return None;
            }
        }
        4 => {
            match (byte, next!()) {
                (0xF0, 0x90..=0xBF) | (0xF1..=0xF3, 0x80..=0xBF) | (0xF4, 0x80..=0x8F) => {}
                _ => return None,
            }
            if next!() as i8 >= -64 {
                return None;
            }
            if next!() as i8 >= -64 {
                return None;
            }
        }
        _ => return None,
    }

    curr += 1;
    Some(curr)
}

/// Returns `true` if any one block is not a valid ASCII byte.
#[inline(always)]
const fn has_non_ascii_byte<const N: usize>(block: &[usize; N]) -> bool {
    let mut vector = *block;
    let mut i = 0;

    while i < N {
        vector[i] &= NONASCII_MASK;
        i += 1;
    }

    i = 0;
    while i < N {
        if vector[i] > 0 {
            return true;
        }

        i += 1;
    }

    false
}

/// Returns the number of consecutive ASCII bytes within `block` until the first
/// non-ASCII byte and `true`, if a non-ASCII byte was found.
///
/// Returns `N * size_of::<usize>()` and `false`, if all bytes are ASCII bytes.
#[inline(always)]
const fn non_ascii_byte_position<const N: usize>(block: &[usize; N]) -> (usize, bool) {
    let mut vector = *block;

    let mut i = 0;
    while i < N {
        vector[i] &= NONASCII_MASK;
        i += 1;
    }

    i = 0;
    while i < N {
        let ctz = vector[i].trailing_zeros() as usize;
        let byte = ctz / BYTES;

        if byte != BYTES {
            return (byte + (i * BYTES), true);
        }

        i += 1;
    }

    (BYTES * N, false)
}

#[inline(always)]
const fn block_end(end: usize, block_size: usize) -> usize {
    if end >= block_size {
        end - block_size + 1
    } else {
        0
    }
}

#[inline(always)]
const fn utf8_char_width(byte: u8) -> usize {
    // https://tools.ietf.org/html/rfc3629
    const UTF8_CHAR_WIDTH: [u8; 256] = [
        // 1  2  3  4  5  6  7  8  9  A  B  C  D  E  F
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 0
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 1
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 2
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 3
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 4
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 5
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 6
        1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // 7
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 8
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // 9
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // A
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // B
        0, 0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // C
        2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, // D
        3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, // E
        4, 4, 4, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, // F
    ];

    UTF8_CHAR_WIDTH[byte as usize] as usize
}

#[cfg(test)]
mod tests {
    #[test]
    fn validate_mostly_ascii() {
        const VERY_LONG_TEXT_UTF: &str = include_str!("../assets/text_utf8");
        assert!(super::validate_utf8(VERY_LONG_TEXT_UTF.as_bytes()));
    }

    #[test]
    fn validate_utf() {
        assert!(super::validate_utf8(b"Lorem ipsum dolor sit amet."));
        assert!(super::validate_utf8(
            "Lörem ipsüm dölör sit ämet.".as_bytes()
        ));
    }

    #[test]
    fn non_ascii_byte_count() {
        let block = [0x7F7F7F7F_7F7F7FFF];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (0, true));
        let block = [0x7F7F7F7F_7F7FFF7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (1, true));
        let block = [0x7F7F7F7F_7FFF7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (2, true));
        let block = [0x7F7F7F7F_FF7F7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (3, true));
        let block = [0x7F7F7FFF_7F7F7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (4, true));
        let block = [0x7F7FFF7F_7F7F7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (5, true));
        let block = [0x7FFF7F7F_7F7F7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (6, true));
        let block = [0xFF7F7F7F_7F7F7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (7, true));
        let block = [0x7F7F7F7F_7F7F7F7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (8, false));

        let block = [0x7F7F7F7F_7F7F7F7F, 0x7F7F7F7F_7F7F7FFF];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (8, true));
        let block = [0x7F7F7F7F_7F7F7F7F, 0x7F7F7F7F_7F7FFF7F];
        let res = super::non_ascii_byte_position(&block);
        assert_eq!(res, (9, true));
    }
}
