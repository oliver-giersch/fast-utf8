use core::{hint, mem, slice};

const WORD_BYTES: usize = mem::size_of::<usize>();
const NONASCII_MASK: usize = usize::from_ne_bytes([0x80; WORD_BYTES]);

#[derive(Debug, PartialEq, Eq)]
pub struct Utf8Error {
    pub valid_up_to: usize,
    pub error_len: Option<u8>,
}

#[derive(Debug, Default)]
pub struct Statistics {
    pub success_blocks_8x: usize,
    pub failed_blocks_8x: usize,
    pub success_blocks_2x: usize,
    pub failed_blocks_2x: usize,
    pub unaligned_blocks: usize,
    pub bytewise_checks: usize,
    pub non_ascii_checks: usize,
    pub optimistic_2x_to_8x: usize,
}

impl Statistics {
    pub fn success_ratio_8x(&self) -> f64 {
        let total = self.success_blocks_8x + self.failed_blocks_8x;
        if total == 0 {
            return 0.0;
        }

        self.success_blocks_8x as f64 / total as f64
    }

    pub fn success_ratio_2x(&self) -> f64 {
        let total = self.success_blocks_2x + self.failed_blocks_2x;
        if total == 0 {
            return 0.0;
        }

        self.success_blocks_2x as f64 / total as f64
    }

    pub fn ratio_8x_to_2x(&self) -> f64 {
        let total_8x = self.success_blocks_8x + self.failed_blocks_8x;
        let total_2x = self.success_blocks_2x + self.failed_blocks_2x;
        if total_2x == 0 {
            0.0
        } else {
            total_8x as f64 / total_2x as f64
        }
    }
}

#[inline(never)]
pub fn validate_utf8(buf: &[u8]) -> Result<(), Utf8Error> {
    validate_utf8_with_stats(buf, None)
}

#[inline(always)]
pub fn validate_utf8_with_stats(
    buf: &[u8],
    mut stats: Option<&mut Statistics>,
) -> Result<(), Utf8Error> {
    let (mut curr, end) = (0, buf.len());
    let start = buf.as_ptr();
    // calculate the byte offset until the first word aligned block
    let align_offset = start.align_offset(WORD_BYTES);

    // calculate the maximum byte at which a block of size N could begin,
    // without taking alignment into account
    let block_end_2x = block_end(end, 2 * WORD_BYTES);
    let block_end_8x = block_end(end, 8 * WORD_BYTES);

    'outer: while curr < end {
        // this block allows us to inexpensively jump to the non-ASCII branch
        // without having to go through the outer loop condition again
        'ascii: {
            if buf[curr] >= 128 {
                break 'ascii;
            }

            // `align_offset` should only ever return `usize::MAX` for ZST
            // pointers to, so ideally the check/branch should be optimized out
            // NOTE: outside of the `std` library, the compiler seems to be
            // unable to determine this must always be false
            if align_offset == usize::MAX {
                curr += 1;
                continue 'outer;
            }

            // check if `curr`'s pointer is word-aligned, otherwise advance curr
            // bytewise until it is byte aligned
            let offset = align_offset.wrapping_sub(curr) % WORD_BYTES;
            if offset > 0 {
                if let Some(stats) = stats.as_mut() {
                    stats.unaligned_blocks += 1;
                }

                // loop until `curr` reaches an aligned byte without checking
                // the alignment condition each time
                let aligned = curr + offset;
                loop {
                    curr += 1;

                    if curr == end {
                        return Ok(());
                    }

                    if buf[curr] >= 128 {
                        break 'ascii;
                    }

                    if curr == aligned {
                        break;
                    }
                }
            }

            // check 8 or 2 word sized blocks for non-ASCII bytes
            let non_ascii = 'block: {
                //if penalty == 0 {
                while curr < block_end_8x {
                    // cast to 8-word array reference
                    // SAFETY: ...
                    let block = unsafe { &*(start.add(curr) as *const [usize; 8]) };
                    if has_non_ascii_byte(block) {
                        if let Some(stats) = stats.as_mut() {
                            stats.failed_blocks_8x += 1;
                        }

                        break 'block 8;
                    }

                    curr += 8 * WORD_BYTES;

                    if let Some(stats) = stats.as_mut() {
                        stats.success_blocks_8x += 1;
                    }
                }

                // check 2-word sized blocks for non-ASCII bytes
                // word-alignment has been determined at this point, so only
                // the buffer length needs to be taken into consideration
                while curr < block_end_2x {
                    // SAFETY: the loop condition guarantees that there is
                    // sufficient room for N word-blocks in the buffer
                    let block = unsafe { &*(start.add(curr) as *const [usize; 2]) };
                    if has_non_ascii_byte(&block) {
                        if let Some(stats) = stats.as_mut() {
                            stats.failed_blocks_2x += 1;
                        }

                        break 'block 2;
                    }

                    curr += 2 * WORD_BYTES;

                    if let Some(stats) = stats.as_mut() {
                        stats.success_blocks_2x += 1;
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
                // SAFETY: the bound invariants as in the previous [8|2]-word
                // block loop apply, since `curr` has not been changed since
                curr += unsafe {
                    let ptr = start.add(curr);
                    let block = slice::from_raw_parts(ptr as *const usize, non_ascii);
                    // SAFETY: since a previous [8|2]-word check "failed", there
                    // *must* be at least one non-ASCII byte somewhere in the
                    // block
                    non_ascii_byte_position(&block) as usize
                };

                break 'ascii;
            }

            if let Some(stats) = stats.as_mut() {
                stats.bytewise_checks += 1;
            }

            // ...otherwise, fall back to byte-wise checks
            loop {
                curr += 1;

                if curr >= end {
                    return Ok(());
                }

                if buf[curr] >= 128 {
                    break 'ascii;
                }
            }

            //curr += 1;
            //continue 'outer;
        }

        if let Some(stats) = stats.as_mut() {
            stats.non_ascii_checks += 1;
        }

        // non-ASCII case: validate up to 4 bytes, then advance `curr`
        // accordingly
        match validate_non_acii_bytes(buf, curr, end) {
            Ok(next) => curr = next,
            Err(e) => return Err(e),
        }
    }

    Ok(())
}

/// Returns `true` if any byte in `block` contains a non-ASCII byte.
///
/// # Note
///
/// This function is written to allow for relatively reliable
/// auto-vectorization, not code size.
#[inline(always)]
const fn has_non_ascii_byte<const N: usize>(block: &[usize; N]) -> bool {
    // mask each word in the block
    let vector = mask_block(block);

    let mut i = 0;
    let mut res = 0;
    while i < N {
        res |= vector[i];
        i += 1;
    }

    res > 0
}

/// Masks every byte of every word in `block`, so that only the MSB of each byte
/// remains, indicating a non-ASCII byte.
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

/// Determines the precise position of the first non-ASCII byte in the given
/// `block`.
///
/// # Safety
///
/// The caller has to guarantee, that `block` does in fact contain a non-ASCII
/// byte.
///
/// # Note
///
/// It would be valid to just return 0 or panic, but this has non-trivial impact
/// on generated code size.
#[inline(always)]
#[cold]
const unsafe fn non_ascii_byte_position(block: &[usize]) -> u32 {
    let mut i = 0;
    while i < block.len() {
        // number of trailing zeroes in a word divided by the size of a word is
        // equivalent to the number of valid ASCII bytes, since the first one
        // bit will be MSB of the first byte within the word that is non-ASCII.
        let ctz = (block[i] & NONASCII_MASK).trailing_zeros();
        if ctz < usize::BITS {
            let byte = ctz / WORD_BYTES as u32;
            return byte + (i as u32 * WORD_BYTES as u32);
        }

        i += 1;
    }

    if cfg!(debug_assertions) {
        panic!("no non-ASCII byte present in block");
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

#[inline(never)]
pub fn validate_utf8_std(v: &[u8]) -> Result<(), Utf8Error> {
    let mut index = 0;
    let len = v.len();

    let usize_bytes = mem::size_of::<usize>();
    let ascii_block_size = 2 * usize_bytes;
    let blocks_end = if len >= ascii_block_size {
        len - ascii_block_size + 1
    } else {
        0
    };
    let align = v.as_ptr().align_offset(usize_bytes);

    while index < len {
        let old_offset = index;
        macro_rules! err {
            ($error_len: expr) => {
                return Err(Utf8Error {
                    valid_up_to: old_offset,
                    error_len: $error_len,
                })
            };
        }

        macro_rules! next {
            () => {{
                index += 1;
                // we needed data, but there was none: error!
                if index >= len {
                    err!(None)
                }
                v[index]
            }};
        }

        let first = v[index];
        if first >= 128 {
            let w = utf8_char_width(first);
            // 2-byte encoding is for codepoints  \u{0080} to  \u{07ff}
            //        first  C2 80        last DF BF
            // 3-byte encoding is for codepoints  \u{0800} to  \u{ffff}
            //        first  E0 A0 80     last EF BF BF
            //   excluding surrogates codepoints  \u{d800} to  \u{dfff}
            //               ED A0 80 to       ED BF BF
            // 4-byte encoding is for codepoints \u{1000}0 to \u{10ff}ff
            //        first  F0 90 80 80  last F4 8F BF BF
            //
            // Use the UTF-8 syntax from the RFC
            //
            // https://tools.ietf.org/html/rfc3629
            // UTF8-1      = %x00-7F
            // UTF8-2      = %xC2-DF UTF8-tail
            // UTF8-3      = %xE0 %xA0-BF UTF8-tail / %xE1-EC 2( UTF8-tail ) /
            //               %xED %x80-9F UTF8-tail / %xEE-EF 2( UTF8-tail )
            // UTF8-4      = %xF0 %x90-BF 2( UTF8-tail ) / %xF1-F3 3( UTF8-tail ) /
            //               %xF4 %x80-8F 2( UTF8-tail )
            match w {
                2 => {
                    if next!() as i8 >= -64 {
                        err!(Some(1))
                    }
                }
                3 => {
                    match (first, next!()) {
                        (0xE0, 0xA0..=0xBF)
                        | (0xE1..=0xEC, 0x80..=0xBF)
                        | (0xED, 0x80..=0x9F)
                        | (0xEE..=0xEF, 0x80..=0xBF) => {}
                        _ => err!(Some(1)),
                    }
                    if next!() as i8 >= -64 {
                        err!(Some(2))
                    }
                }
                4 => {
                    match (first, next!()) {
                        (0xF0, 0x90..=0xBF) | (0xF1..=0xF3, 0x80..=0xBF) | (0xF4, 0x80..=0x8F) => {}
                        _ => err!(Some(1)),
                    }
                    if next!() as i8 >= -64 {
                        err!(Some(2))
                    }
                    if next!() as i8 >= -64 {
                        err!(Some(3))
                    }
                }
                _ => err!(Some(1)),
            }
            index += 1;
        } else {
            // Ascii case, try to skip forward quickly.
            // When the pointer is aligned, read 2 words of data per iteration
            // until we find a word containing a non-ascii byte.
            if align != usize::MAX && align.wrapping_sub(index) % usize_bytes == 0 {
                let ptr = v.as_ptr();
                while index < blocks_end {
                    // SAFETY: since `align - index` and `ascii_block_size` are
                    // multiples of `usize_bytes`, `block = ptr.add(index)` is
                    // always aligned with a `usize` so it's safe to dereference
                    // both `block` and `block.add(1)`.
                    unsafe {
                        let block = ptr.add(index) as *const usize;
                        // break if there is a nonascii byte
                        let zu = contains_nonascii(*block);
                        let zv = contains_nonascii(*block.add(1));
                        if zu || zv {
                            break;
                        }
                    }
                    index += ascii_block_size;
                }
                // step from the point where the wordwise loop stopped
                while index < len && v[index] < 128 {
                    index += 1;
                }
            } else {
                index += 1;
            }
        }
    }

    Ok(())
}

#[inline]
const fn contains_nonascii(x: usize) -> bool {
    (x & NONASCII_MASK) != 0
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
    fn invalid_ascii() {
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
