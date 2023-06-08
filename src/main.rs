use fast_utf8::Statistics;

fn main() {
    const ENGLISH_406: &str = include_str!("../assets/english_406kb.txt");
    const ENGLISH_971: &str = include_str!("../assets/english_971kb.txt");
    const HUNGARIAN_246: &str = include_str!("../assets/hungarian_246kb.txt");

    function_sizes();

    let mut stats = Statistics::default();
    assert!(fast_utf8::validate_utf8_with_stats(ENGLISH_406.as_bytes(), Some(&mut stats)).is_ok());
    println!("==========english/406kb/95pct-ascii==========\n{stats:#?}");
    println!("success ratio 8x: {}", stats.success_ratio_8x());
    println!("success ratio 2x: {}", stats.success_ratio_2x());
    println!("ratio 8x to 2x: {}", stats.success_ratio_8x());

    stats = Statistics::default();
    assert!(fast_utf8::validate_utf8_with_stats(ENGLISH_971.as_bytes(), Some(&mut stats)).is_ok());
    println!("==========english/977kb/99pct-ascii==========\n{stats:#?}");
    println!("success ratio 8x: {}", stats.success_ratio_8x());
    println!("success ratio 2x: {}", stats.success_ratio_2x());
    println!("ratio 8x to 2x: {}", stats.success_ratio_8x());

    stats = Statistics::default();
    assert!(
        fast_utf8::validate_utf8_with_stats(HUNGARIAN_246.as_bytes(), Some(&mut stats)).is_ok()
    );
    println!("==========hungarian/246kb/XXpct-ascii==========\n{stats:#?}");
    println!("success ratio 8x: {}", stats.success_ratio_8x());
    println!("success ratio 2x: {}", stats.success_ratio_2x());
    println!("ratio 8x to 2x: {}", stats.success_ratio_8x());
}

/*
 * Linux-ELF, x86-64, function sizes
 * fast_utf8::validate_utf8_std:    474B
 * fast_utf8::validate_utf8:        1.1KiB (~2x larger)
 */

fn function_sizes() {
    let text = b"";
    assert!(fast_utf8::validate_utf8(text).is_ok());
    assert!(fast_utf8::validate_utf8_std(text).is_ok());
}
