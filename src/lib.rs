use core::{hint, mem, slice};

const WORD_BYTES: usize = mem::size_of::<usize>();
const NONASCII_MASK: usize = usize::from_ne_bytes([0x80; WORD_BYTES]);

#[derive(Debug, PartialEq, Eq)]
pub struct Utf8Error {
    pub valid_up_to: usize,
    pub error_len: Option<u8>,
}

struct Statistics {
    failed_blocks_8x: usize,
    failed_blocks_2x: usize,
    unaligned_blocks: usize,
    bytewise_checks: usize,
    non_ascii_checks: usize,
    optimistic_2x_to_8x: usize,
}

#[inline(never)]
pub fn validate_utf8(buf: &[u8]) -> Result<(), Utf8Error> {
    // establish byte buffer bounds
    let (mut curr, end) = (0, buf.len());
    let start = buf.as_ptr();
    // calculate the byte offset until the first word aligned block
    let align_offset = start.align_offset(WORD_BYTES);

    // calculate the maximum byte at which a block of size N could begin,
    // without taking alignment into account
    let block_end_2x = block_end(end, 2 * WORD_BYTES);
    let block_end_8x = block_end(end, 8 * WORD_BYTES);

    // this serves as a replacement for a goto statement, any invocation of this
    // macro should be compiled to a simple jump instruction to the appropriate
    // label within in the function and *not* be inlined separately each time
    macro_rules! non_ascii {
        () => {
            // non-ASCII case: validate up to 4 bytes, then advance `curr`
            // accordingly
            match validate_non_acii_bytes(buf, curr, end) {
                Ok(next) => curr = next,
                Err(e) => return Err(e),
            }
        };
    }

    let mut penalty: usize = 0;

    while curr < end {
        if buf[curr] >= 128 {
            non_ascii!();
            continue;
        }

        // `align_offset` can basically only be `usize::MAX` for pointers to
        // ZSTs, so the first check/branch is almost certainly optimized out
        if align_offset == usize::MAX {
            curr += 1;
            continue;
        }

        // check if `curr`'s pointer is word-aligned, otherwise advance curr
        // bytewise until it is byte aligned
        let offset = align_offset.wrapping_sub(curr) % WORD_BYTES;
        if offset > 0 {
            // the offset at which the `curr` pointer will be aligned again
            let aligned = curr + offset;
            // the first unaligned byte has already be determined the be
            // valid ASCII, so it is not checked again
            curr += 1;

            // no need to check alignment again for every byte, so skip
            // up to `offset` valid ASCII bytes if possible
            while curr < aligned {
                // the buffer may end before an aligned byte is reached
                if curr == buf.len() {
                    return Ok(());
                }

                if buf[curr] < 128 {
                    curr += 1;
                    continue;
                }

                non_ascii!();
            }
        }

        let non_ascii = 'block: loop {
            if penalty <= 16 && curr < block_end_8x {
                let blocks = (block_end_8x - curr) / (8 * WORD_BYTES);
                let mut i = 0;

                while i < blocks {
                    let block = unsafe { &*(start.add(curr) as *const [usize; 8]) };
                    if has_non_ascii_byte(block) {
                        penalty += 16;
                        break 'block 8;
                    }

                    curr += 8 * WORD_BYTES;
                    i += 1;
                }
            }

            // check N-word sized blocks for non-ASCII bytes
            // word-alignment has been determined at this point, so only
            // the buffer length needs to be taken into consideration
            while curr < block_end_2x {
                // SAFETY: the loop condition guarantees that there is
                // sufficient room for N word-blocks in the buffer
                let block = unsafe { &*(start.add(curr) as *const [usize; 2]) };
                if has_non_ascii_byte(&block) {
                    penalty += 4;
                    break 'block 2;
                }

                curr += 2 * WORD_BYTES;
                if penalty >= 2 {
                    penalty -= 2;
                } else if curr < block_end_8x {
                    continue 'block;
                }
            }

            break 'block 0;
        };

        // if the block loop was stopped due to a non-ascii byte
        // in some word, do another word-wise search using the same word
        // buffer used before in order to avoid having to checking all
        // bytes individually again.
        if non_ascii > 0 {
            // calculate the amount of bytes that can be skipped without
            // having to check them individually
            curr += unsafe {
                let ptr = start.add(curr);
                let block = slice::from_raw_parts(ptr as *const usize, non_ascii);
                non_ascii_byte_position(&block) as usize
            };

            non_ascii!();
            continue;
        }

        // ...otherwise, fall back to byte-wise checks
        curr = validate_ascii_bytewise(buf, curr);
    }

    Ok(())
}

#[inline(always)]
#[cold]
const fn validate_ascii_bytewise(buf: &[u8], mut curr: usize) -> usize {
    while curr < buf.len() && buf[curr] < 128 {
        curr += 1;
    }

    curr
}

#[inline(always)]
#[cold]
const fn validate_ascii_bytewise_unaligned(
    buf: &[u8],
    offset: usize,
    mut curr: usize,
) -> (usize, bool) {
    let end = curr + offset;
    while curr < end {
        // no need to check alignment again for every byte, so skip
        // up to `offset` valid ASCII bytes if possible
        curr += 1;

        if !(curr < buf.len() && buf[curr] < 128) {
            return (curr, true);
        }
    }

    (curr, false)
}

#[inline(always)]
const fn has_non_ascii_byte<const N: usize>(block: &[usize; N]) -> bool {
    let vector = mask_block(block);

    let mut i = 0;
    let mut res = 0;
    while i < N {
        res |= vector[i];
        i += 1;
    }

    res > 0
}

#[inline(always)]
const fn mask_block<const N: usize>(block: &[usize; N]) -> [usize; N] {
    let mut masked = [0usize; N];
    let mut i = 0;

    while i < N {
        masked[i] = block[i] & NONASCII_MASK;
        i += 1;
    }

    masked
}

#[inline(always)]
#[cold]
const unsafe fn non_ascii_byte_position(block: &[usize]) -> u32 {
    let mut i = 0;
    while i < block.len() {
        // number of trailing zeroes in a word is equivalent to the number of
        // valid ASCII "nibbles"
        let ctz = (block[i] & NONASCII_MASK).trailing_zeros();
        if ctz < usize::BITS {
            let byte = ctz / WORD_BYTES as u32;
            return byte + (i as u32 * WORD_BYTES as u32);
        }

        i += 1;
    }

    // SAFETY: presence of a non-ASCII byte is required as function invariant
    unsafe { hint::unreachable_unchecked() }
}

/// Used by all variants, validates non-ascii bytes, identical to STD
#[inline(always)]
#[cold]
const fn validate_non_acii_bytes(
    buf: &[u8],
    mut curr: usize,
    end: usize,
) -> Result<usize, Utf8Error> {
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
    Ok(curr)
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

    use super::validate_utf8;

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
            validate_utf8(b"A\xC3\xA9 \xF1\x80 "),
            Err(super::Utf8Error {
                valid_up_to: 4,
                error_len: Some(2)
            })
        );
    }

    #[test]
    fn validate_mostly_ascii() {
        assert!(validate_utf8(GERMAN_UTF8_16KB.as_bytes()).is_ok());
    }

    #[test]
    fn validate_invalid() {
        let mut vec = Vec::from(GERMAN_UTF8_16KB);
        vec.push(0xFF);

        assert_eq!(validate_utf8(&vec).is_ok(), false);
    }

    #[test]
    fn validate_utf() {
        assert!(validate_utf8(b"Lorem ipsum dolor sit amet.").is_ok());
        assert!(validate_utf8("Lörem ipsüm dölör sit ämet.".as_bytes()).is_ok());
    }

    #[test]
    fn non_ascii_byte_count() {
        unsafe {
            let block = [0x7F7F7F7F_7F7F7FFF];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 0);
            let block = [0x7F7F7F7F_7F7FFF7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 1);
            let block = [0x7F7F7F7F_7FFF7F7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 2);
            let block = [0x7F7F7F7F_FF7F7F7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 3);
            let block = [0x7F7F7FFF_7F7F7F7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 4);
            let block = [0x7F7FFF7F_7F7F7F7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 5);
            let block = [0x7FFF7F7F_7F7F7F7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 6);
            let block = [0xFF7F7F7F_7F7F7F7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 7);
            let block = [0x7F7F7F7F_7F7F7F7F, 0x7F7F7F7F_7F7F7FFF];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 8);
            let block = [0x7F7F7F7F_7F7F7F7F, 0x7F7F7F7F_7F7FFF7F];
            let res = super::non_ascii_byte_position(&block);
            assert_eq!(res, 9);
        }
    }

    #[test]
    fn faust() {
        const FAUST: &str = include_str!("../assets/faust_213kb.txt");
        assert!(validate_utf8(FAUST.as_bytes()).is_ok());
    }

    #[test]
    fn chinese() {
        const CHINESE: &str = include_str!("../assets/chinese_1mb.txt");
        assert!(validate_utf8(CHINESE.as_bytes()).is_ok());
    }

    #[test]
    fn latin_3kb() {
        const LATIN_3KB: &str = include_str!("../assets/latin_3kb.txt");
        assert!(validate_utf8(LATIN_3KB.as_bytes()).is_ok());
    }

    #[test]
    fn english_99pct_ascii() {
        const ENGLISH: &str = include_str!("../assets/english_971kb.txt");
        assert!(validate_utf8(ENGLISH.as_bytes()).is_ok());
    }
}
