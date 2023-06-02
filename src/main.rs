use fast_utf8::Statistics;

fn main() {
    const ENGLISH_406: &str = include_str!("../assets/english_406kb.txt");
    const ENGLISH_971: &str = include_str!("../assets/english_971kb.txt");

    let mut stats = Statistics::default();
    assert!(fast_utf8::validate_utf8_with_stats(ENGLISH_406.as_bytes(), Some(&mut stats)).is_ok());
    println!("==========english/406kb/95pct-ascii==========\n{stats:#?}");

    stats = Statistics::default();
    assert!(fast_utf8::validate_utf8_with_stats(ENGLISH_971.as_bytes(), Some(&mut stats)).is_ok());
    println!("==========english/977kb/99pct-ascii==========\n{stats:#?}");
}
