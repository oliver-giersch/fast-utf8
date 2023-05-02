use core::{mem, slice};

const BYTES: usize = mem::size_of::<usize>();
const NONASCII_MASK: usize = usize::from_ne_bytes([0x80; BYTES]);

#[inline]
pub fn validate_utf8(buf: &[u8]) -> bool {
    // we check aligned blocks of up to 8 words at a time
    const ASCII_BLOCK_8X: usize = 8 * BYTES;
    const ASCII_BLOCK_4X: usize = 4 * BYTES;
    const ASCII_BLOCK_2X: usize = 2 * BYTES;

    // establish buffer extent
    let (mut curr, end) = (0, buf.len());
    let start = buf.as_ptr();
    // calculate the byte offset until the first word aligned block
    let align_offset = start.align_offset(BYTES);

    // calculate the maximum byte at which a block of size N could begin,
    // without taking alignment into account
    let block_end_8x = block_end(end, ASCII_BLOCK_8X);
    let block_end_4x = block_end(end, ASCII_BLOCK_4X);
    let block_end_2x = block_end(end, ASCII_BLOCK_2X);

    while curr < end {
        if buf[curr] < 128 {
            // `align_offset` can basically only be `usize::MAX` for ZST
            // pointers, so the first check is almost certainly optimized away
            if align_offset == usize::MAX {
                curr += 1;
                continue;
            }

            // check if `curr`'s pointer is word-aligned
            let offset = align_offset.wrapping_sub(curr) % BYTES;
            if offset == 0 {
                let len = 'block: loop {
                    macro_rules! block_loop {
                        ($N:expr) => {
                            // SAFETY: we have checked before that there are
                            // still at least `N * size_of::<usize>()` in the
                            // buffer and that the current byte is word-aligned
                            let block = unsafe { &*(start.add(curr) as *const [usize; $N]) };
                            if has_non_ascii_byte(block) {
                                break 'block Some($N);
                            }

                            curr += $N * BYTES;
                        };
                    }

                    // check 8-word blocks for non-ASCII bytes
                    while curr < block_end_8x {
                        block_loop!(8);
                    }

                    // check 4-word blocks for non-ASCII bytes
                    while curr < block_end_4x {
                        block_loop!(4);
                    }

                    // check 2-word blocks for non-ASCII bytes
                    while curr < block_end_2x {
                        block_loop!(2);
                    }

                    // `(size_of::<usize>() * 2) + (align_of::<usize> - 1)`
                    // bytes remain at most
                    break None;
                };

                // if the block loops were stopped due to a non-ascii byte
                // in some block, do another block-wise search using the last
                // used block-size for the specific byte in the previous block
                // in order to skip checking all bytes up to that one
                // individually.
                // NOTE: this operation does not auto-vectorize well, so it is
                // done only in case a non-ASCII byte is actually found
                if let Some(len) = len {
                    // SAFETY: `curr` has not changed since the last block loop,
                    // so it still points at a byte marking the beginning of a
                    // word-sized block of the given `len`
                    let block = unsafe {
                        let ptr = start.add(curr) as *const usize;
                        slice::from_raw_parts(ptr, len)
                    };

                    // calculate the amount of bytes that can be skipped without
                    // having to check them individually
                    let (skip, non_ascii) = non_ascii_byte_position(block);
                    curr += skip;

                    // if a non-ASCII byte was found, skip the subsequent
                    // byte-wise loop and go straight back to the main loop
                    if non_ascii {
                        continue;
                    }
                }

                // ...otherwise, fall back to byte-wise checks
                while curr < end && buf[curr] < 128 {
                    curr += 1;
                }
            } else {
                // byte is < 128 (ASCII), but pointer is not word-aligned, skip
                // until the loop reaches the next word-aligned block)
                for _ in 0..offset {
                    // no need to check alignment again for every byte, so skip
                    // up to `offset` valid ASCII bytes if possible
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

#[inline]
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
/// Returns `block.len() * size_of::<usize>()` and `false`, if all bytes are
/// ASCII bytes.
#[inline(always)]
const fn non_ascii_byte_position(block: &[usize]) -> (usize, bool) {
    let mut i = 0;
    while i < block.len() {
        let mask = block[i] & NONASCII_MASK;
        let ctz = mask.trailing_zeros() as usize;
        let byte = ctz / BYTES;

        if byte != BYTES {
            return (byte + (i * BYTES), true);
        }

        i += 1;
    }

    (BYTES * block.len(), false)
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
    const VERY_LONG_TEXT_UTF: &str = include_str!("../assets/text_utf8");

    #[test]
    fn validate_mostly_ascii() {
        assert!(super::validate_utf8(VERY_LONG_TEXT_UTF.as_bytes()));
    }

    #[test]
    fn validate_invalid() {
        let mut vec = Vec::from(VERY_LONG_TEXT_UTF);
        vec.push(0xFF);

        assert_eq!(super::validate_utf8(&vec), false);
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
