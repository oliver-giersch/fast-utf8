use core::{hint, mem};

const WORD_BYTES: usize = mem::size_of::<usize>();
const NONASCII_MASK: usize = usize::from_ne_bytes([0x80; WORD_BYTES]);

#[derive(Debug, PartialEq, Eq)]
pub struct Utf8Error {
    pub valid_up_to: usize,
    pub error_len: Option<u8>,
}

// TODO: implement short/long string versions (cache line boundary?)
// TODO: implement dynamic back-off mechanism (back off from SIMD)
//  - non-ASCII char in block-wise path: add penalty for further block checks (increase with block-size)
//  - ASCII char in byte-wise path: decrease penalty
//  - non-ASCII char in byte-wise: increase penalty
// byte-wise checks: best for 0-10% ASCII
// ???: anything in between
// large blocks: best for 100% ASCII

#[cfg(feature = "stats")]
#[derive(Default, Debug)]
pub struct Stats {
    count_8x: usize,
    count_4x: usize,
    count_2x: usize,
    failed_8x: usize,
    failed_4x: usize,
    failed_2x: usize,
    pessimistic_byte_wise_count: usize,
    non_ascii_count: usize,
}

#[inline(never)]
pub fn validate_utf8_baseline<const N: usize>(buf: &[u8]) -> Result<(), Utf8Error> {
    /*if buf.len() < 32 {
        todo!()
    } else {
        validate_long(buf)
    }*/
    validate_long_baseline::<N>(buf)
}

// relatively close to STD implementation, minor improvements, generic block size
#[inline(always)]
fn validate_long_baseline<const _N: usize>(buf: &[u8]) -> Result<(), Utf8Error> {
    // establish byte buffer bounds
    let (mut curr, end) = (0, buf.len());
    let start = buf.as_ptr();
    // calculate the byte offset until the first word aligned block
    let align_offset = start.align_offset(WORD_BYTES);

    // calculate the maximum byte at which a block of size N could begin,
    // without taking alignment into account
    let block_end_2x = block_end(end, 2 * WORD_BYTES);
    let block_end_8x = block_end(end, 8 * WORD_BYTES);
    let mut masked_words = [0usize; 8];

    let mut penalty: usize = 0;

    while curr < end {
        if buf[curr] < 128 {
            // `align_offset` can basically only be `usize::MAX` for pointers to
            // ZSTs, so the first check/branch is almost certainly optimized out
            if align_offset == usize::MAX {
                curr += 1;
                continue;
            }

            // check if `curr`'s pointer is word-aligned
            let offset = align_offset.wrapping_sub(curr) % WORD_BYTES;
            if offset == 0 {
                let mut has_non_ascii = 0;
                if penalty == 0 {
                    while curr < block_end_8x {
                        let block = unsafe { &*(start.add(curr) as *const [usize; 8]) };
                        if has_non_ascii_byte_8x(block) {
                            masked_words = [
                                block[0] & NONASCII_MASK,
                                block[1] & NONASCII_MASK,
                                block[2] & NONASCII_MASK,
                                block[3] & NONASCII_MASK,
                                block[4] & NONASCII_MASK,
                                block[5] & NONASCII_MASK,
                                block[6] & NONASCII_MASK,
                                block[7] & NONASCII_MASK,
                            ];

                            penalty += 16;
                            has_non_ascii = 8;

                            break;
                        }

                        curr += 8 * WORD_BYTES;
                    }
                }

                // check N-word sized blocks for non-ASCII bytes
                // word-alignment has been determined at this point, so only
                // the buffer length needs to be taken into consideration
                while curr < block_end_2x {
                    // SAFETY: the loop condition guarantees that there is
                    // sufficient room for N word-blocks in the buffer
                    let block = unsafe { &*(start.add(curr) as *const [usize; 2]) };
                    let mut i = 0;
                    while i < 2 {
                        masked_words[i] = block[i] & NONASCII_MASK;
                        i += 1;
                    }

                    if has_non_ascii_byte(&masked_words) {
                        has_non_ascii = 2;
                        break;
                    }

                    curr += 2 * WORD_BYTES;
                    penalty = penalty.saturating_sub(2);
                }

                // if the block loop was stopped due to a non-ascii byte
                // in some word, do another word-wise search using the same word
                // buffer used before in order to avoid having to checking all
                // bytes individually again.
                if has_non_ascii > 0 {
                    // calculate the amount of bytes that can be skipped without
                    // having to check them individually
                    let skip = unsafe { non_ascii_byte_position(&masked_words, has_non_ascii) };
                    curr += skip as usize;
                    continue;
                }

                // ...otherwise, fall back to byte-wise checks
                while curr < end && buf[curr] < 128 {
                    curr += 1;
                }
            } else {
                // byte is < 128 (ASCII), but pointer is not word-aligned, skip
                // until the loop reaches the next word-aligned block)
                let mut i = 0;
                while i < offset {
                    // no need to check alignment again for every byte, so skip
                    // up to `offset` valid ASCII bytes if possible
                    curr += 1;

                    if !(curr < end && buf[curr] < 128) {
                        break;
                    }

                    i += 1;
                }
            }
        } else {
            // non-ASCII case: validate up to 4 bytes, then advance `curr`
            // accordingly
            match validate_non_acii_bytes(buf, curr, end) {
                Ok((next, _)) => {
                    curr = next;
                }
                Err(e) => return Err(e),
            }
        }
    }

    Ok(())
}

const fn has_non_ascii_byte_8x(block: &[usize; 8]) -> bool {
    let vector = [
        block[0] & NONASCII_MASK,
        block[1] & NONASCII_MASK,
        block[2] & NONASCII_MASK,
        block[3] & NONASCII_MASK,
        block[4] & NONASCII_MASK,
        block[5] & NONASCII_MASK,
        block[6] & NONASCII_MASK,
        block[7] & NONASCII_MASK,
    ];

    let mut i = 0;
    let mut res = 0;

    while i < 8 {
        res |= vector[i];
        i += 1;
    }

    res > 0
}

#[inline(always)]
const fn has_non_ascii_byte(masked_words: &[usize; 8]) -> bool {
    masked_words[0] > 0 || masked_words[1] > 0
}

#[inline(always)]
const fn has_non_ascii_byte_nx<const N: usize>(masked_words: &[usize; 8]) -> bool {
    let mut res = 0;
    let mut i = 0;
    while i < N {
        res |= masked_words[i];
        i += 1;
    }

    res > 0
}

#[inline(always)]
#[track_caller]
const unsafe fn non_ascii_byte_position(mask_block: &[usize; 8], len: usize) -> u32 {
    let mut i = 0;
    while i < len {
        // number of trailing zeroes in a word is equivalent to the number of
        // valid ASCII "nibbles"
        let ctz = mask_block[i].trailing_zeros();
        if ctz < usize::BITS {
            let byte = ctz / WORD_BYTES as u32;
            return byte + (i as u32 * WORD_BYTES as u32);
        }

        i += 1;
    }

    // SAFETY: presence of a non-ASCII byte is required as function invariant
    unreachable!()
    //unsafe { hint::unreachable_unchecked() }
}

/// Used by all variants, validates non-ascii bytes, identical to STD
#[inline(always)]
const fn validate_non_acii_bytes(
    buf: &[u8],
    mut curr: usize,
    end: usize,
) -> Result<(usize, usize), Utf8Error> {
    let prev = curr;
    macro_rules! err {
        ($error_len: expr) => {
            return Err(Utf8Error {
                valid_up_to: prev,
                error_len: $error_len,
            })
        };
    }

    macro_rules! next {
        () => {{
            curr += 1;
            // we needed data, but there was none: error!
            if curr >= end {
                err!(None);
            }
            buf[curr]
        }};
    }

    let byte = buf[curr];
    let width = utf8_char_width(byte);
    match width {
        2 => {
            if next!() as i8 >= -64 {
                err!(Some(1));
            }
        }
        3 => {
            match (byte, next!()) {
                (0xE0, 0xA0..=0xBF)
                | (0xE1..=0xEC, 0x80..=0xBF)
                | (0xED, 0x80..=0x9F)
                | (0xEE..=0xEF, 0x80..=0xBF) => {}
                _ => err!(Some(1)),
            }

            if next!() as i8 >= -64 {
                err!(Some(2));
            }
        }
        4 => {
            match (byte, next!()) {
                (0xF0, 0x90..=0xBF) | (0xF1..=0xF3, 0x80..=0xBF) | (0xF4, 0x80..=0x8F) => {}
                _ => err!(Some(1)),
            }
            if next!() as i8 >= -64 {
                err!(Some(2));
            }
            if next!() as i8 >= -64 {
                err!(Some(3));
            }
        }
        _ => err!(Some(1)),
    }

    curr += 1;
    Ok((curr, width))
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
    const GERMAN_UTF8_16KB: &str = include_str!("../assets/german_16kb.txt");

    fn validate_utf8(buf: &[u8]) -> Result<(), super::Utf8Error> {
        super::validate_utf8_baseline::<2>(buf)
    }

    fn validate_long_baseline_8x(buf: &[u8]) -> Result<(), super::Utf8Error> {
        super::validate_utf8_baseline::<8>(buf)
    }

    #[cfg(not(feature = "stats"))]
    #[test]
    fn invalid_utf8() {
        assert_eq!(
            validate_utf8(b"A\xC3\xA9 \xF1 "),
            Err(super::Utf8Error {
                valid_up_to: 4,
                error_len: Some(1)
            })
        );

        assert_eq!(
            validate_long_baseline_8x(b"A\xC3\xA9 \xF1 "),
            Err(super::Utf8Error {
                valid_up_to: 4,
                error_len: Some(1)
            })
        );

        assert_eq!(
            validate_utf8(b"A\xC3\xA9 \xF1\x80 "),
            Err(super::Utf8Error {
                valid_up_to: 4,
                error_len: Some(2)
            })
        );

        assert_eq!(
            validate_long_baseline_8x(b"A\xC3\xA9 \xF1\x80 "),
            Err(super::Utf8Error {
                valid_up_to: 4,
                error_len: Some(2)
            })
        );
    }

    #[cfg(not(feature = "stats"))]
    #[test]
    fn validate_mostly_ascii() {
        assert!(validate_utf8(GERMAN_UTF8_16KB.as_bytes()).is_ok());
        assert!(validate_long_baseline_8x(GERMAN_UTF8_16KB.as_bytes()).is_ok());
    }

    #[cfg(not(feature = "stats"))]
    #[test]
    fn validate_invalid() {
        let mut vec = Vec::from(GERMAN_UTF8_16KB);
        vec.push(0xFF);

        assert_eq!(validate_utf8(&vec).is_ok(), false);
        assert_eq!(validate_long_baseline_8x(&vec).is_ok(), false);
    }

    #[cfg(not(feature = "stats"))]
    #[test]
    fn validate_utf() {
        assert!(validate_utf8(b"Lorem ipsum dolor sit amet.").is_ok());
        assert!(validate_utf8("Lörem ipsüm dölör sit ämet.".as_bytes()).is_ok());

        assert!(validate_long_baseline_8x(b"Lorem ipsum dolor sit amet.").is_ok());
        assert!(validate_long_baseline_8x("Lörem ipsüm dölör sit ämet.".as_bytes()).is_ok());
    }

    /*#[cfg(not(feature = "stats"))]
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
    }*/

    #[cfg(not(feature = "stats"))]
    #[test]
    fn faust() {
        const FAUST: &str = include_str!("../assets/faust_213kb.txt");
        assert!(validate_utf8(FAUST.as_bytes()).is_ok());
        assert!(validate_long_baseline_8x(FAUST.as_bytes()).is_ok());
    }

    #[cfg(not(feature = "stats"))]
    #[test]
    fn chinese() {
        const CHINESE: &str = include_str!("../assets/chinese_1mb.txt");
        assert!(validate_utf8(CHINESE.as_bytes()).is_ok());
        assert!(validate_long_baseline_8x(CHINESE.as_bytes()).is_ok());
    }

    #[cfg(feature = "stats")]
    #[test]
    fn stats() {
        const FAUST: &str = include_str!("../assets/faust_213kb.txt");

        let mut stats = super::Stats::default();
        assert!(super::validate_utf8(FAUST.as_bytes(), Some(&mut stats)).is_ok(),);
        dbg!(stats);

        const A_ROOM_WITH_A_VIEW: &str = include_str!("../assets/english_406kb.txt");
        stats = super::Stats::default();
        assert!(super::validate_utf8(A_ROOM_WITH_A_VIEW.as_bytes(), Some(&mut stats)).is_ok(),);
        dbg!(stats);

        const ENGLISH: &str = include_str!("../assets/hamlet.txt");
        stats = super::Stats::default();
        assert!(super::validate_utf8(ENGLISH.as_bytes(), Some(&mut stats)).is_ok(),);
        dbg!(stats);
    }
}
