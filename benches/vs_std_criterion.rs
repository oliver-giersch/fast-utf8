use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};

#[inline(always)]
fn fast_baseline_2x(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8_baseline::<2>(buf).is_ok()
}

#[inline(always)]
fn fast_baseline_4x(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8_baseline::<4>(buf).is_ok()
}

#[inline(always)]
fn fast_baseline_8x(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8_baseline::<8>(buf).is_ok()
}

#[inline(always)]
fn std(buf: &[u8]) -> bool {
    std::str::from_utf8(buf).is_ok()
}

fn bench_group(c: &mut Criterion, language: &'static str, text: &[u8]) {
    let group_name = format!(
        "{language}/{}/{}pct-ascii",
        text_size(text),
        ascii_ratio(text)
    );

    let mut group = c.benchmark_group(group_name);
    validate_group(&mut group, text);
    group.finish();
}

fn text_size(bytes: &[u8]) -> String {
    let mut size = bytes.len();
    let mut i = 0;
    loop {
        let next = size / 1000;
        if next == 0 {
            return format!(
                "{}{}",
                size,
                match i {
                    0 => "b",
                    1 => "kb",
                    2 => "mb",
                    3 => "gb",
                    4 => "tb",
                    _ => unreachable!(),
                }
            );
        }

        size = next;
        i += 1;
    }
}

fn ascii_ratio(bytes: &[u8]) -> u32 {
    if bytes.is_empty() {
        return 0;
    }

    let mut ascii_count = 0;
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i].is_ascii() {
            ascii_count += 1;
        }

        i += 1;
    }

    (ascii_count * 100 / bytes.len()) as u32
}

fn validate(f: fn(&[u8]) -> bool, text: &[u8]) {
    let ok = black_box(f(black_box(text)));
    assert!(black_box(ok));
}

fn validate_group(group: &mut BenchmarkGroup<'_, criterion::measurement::WallTime>, text: &[u8]) {
    //group.bench_function("fast-dynamic", |b| b.iter(|| validate(fast_dynamic, text)));
    group.bench_function("fast-2x", |b| b.iter(|| validate(fast_baseline_2x, text)));
    group.bench_function("fast-4x", |b| b.iter(|| validate(fast_baseline_4x, text)));
    group.bench_function("fast-8x", |b| b.iter(|| validate(fast_baseline_8x, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
}

fn none_0b(c: &mut Criterion) {
    bench_group(c, "none", b"");
}

fn latin_3kb(c: &mut Criterion) {
    const LATIN_3KB: &str = include_str!("../assets/latin_3kb.txt");
    bench_group(c, "latin", LATIN_3KB.as_bytes());
}

fn latin_27b(c: &mut Criterion) {
    bench_group(c, "latin", b"Lorem ipsum dolor sit amet.");
}

fn mixed_14kb(c: &mut Criterion) {
    const DEMO: &str = include_str!("../assets/demo_14kb.txt");
    bench_group(c, "mixed", DEMO.as_bytes());
}

fn german_16kb(c: &mut Criterion) {
    const GERMAN: &str = include_str!("../assets/german_16kb.txt");
    bench_group(c, "german", GERMAN.as_bytes());
}

fn greek_57kb(c: &mut Criterion) {
    const GREEK: &str = include_str!("../assets/greek_57kb.txt");
    bench_group(c, "greek", GREEK.as_bytes());
}

fn german_100kb(c: &mut Criterion) {
    const GERMAN: &str = include_str!("../assets/german_100kb.txt");
    bench_group(c, "german", GERMAN.as_bytes());
}

fn greek_152kb(c: &mut Criterion) {
    const GREEK: &str = include_str!("../assets/greek_152kb.txt");
    bench_group(c, "greek", GREEK.as_bytes());
}

fn english_191kb(c: &mut Criterion) {
    const HAMLET: &str = include_str!("../assets/hamlet.txt");
    bench_group(c, "english", HAMLET.as_bytes());
}

fn faust_213kb(c: &mut Criterion) {
    const FAUST: &str = include_str!("../assets/faust_213kb.txt");
    bench_group(c, "german", FAUST.as_bytes());
}

fn hungarian_246kb(c: &mut Criterion) {
    const HUNGARIAN: &str = include_str!("../assets/hungarian_246kb.txt");
    bench_group(c, "hungarian", HUNGARIAN.as_bytes());
}

fn english_406kb(c: &mut Criterion) {
    const A_ROOM_WITH_A_VIEW: &str = include_str!("../assets/english_406kb.txt");
    bench_group(c, "english", A_ROOM_WITH_A_VIEW.as_bytes());
}

fn english_971kb(c: &mut Criterion) {
    const COUNT_FATHOM: &str = include_str!("../assets/english_971kb.txt");
    bench_group(c, "english", COUNT_FATHOM.as_bytes());
}

fn german_978kb(c: &mut Criterion) {
    const GERMAN: &str = include_str!("../assets/german_978kb.txt");
    bench_group(c, "german", GERMAN.as_bytes());
}

fn chinese_1mb(c: &mut Criterion) {
    const CHINESE: &str = include_str!("../assets/chinese_1mb.txt");
    bench_group(c, "chinese", CHINESE.as_bytes());
}

fn greek_1_5mb(c: &mut Criterion) {
    const GREEK: &str = include_str!("../assets/greek_1_5mb.txt");
    bench_group(c, "greek", GREEK.as_bytes());
}

/*fn hamlet(c: &mut Criterion) {
    let text = HAMLET.as_bytes();
    let mut group = c.benchmark_group("hamlet");
    validate_group(&mut group, text);
    group.finish();
}

fn mostly_ascii(c: &mut Criterion) {
    let text = MOSTLY_ASCII.as_bytes();
    let mut group = c.benchmark_group("mostly-ascii");
    validate_group(&mut group, text);
    group.finish();
}

fn long(c: &mut Criterion) {
    let text = LONG_TEXT.as_bytes();
    let mut group = c.benchmark_group("long");
    validate_group(&mut group, text);
    group.finish();
}

fn medium(c: &mut Criterion) {
    let text = MEDIUM_TEXT.as_bytes();
    let mut group = c.benchmark_group("medium");
    validate_group(&mut group, text);
    group.finish();
}

fn short(c: &mut Criterion) {
    let text = SHORT_TEXT.as_bytes();
    let mut group = c.benchmark_group("short");
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn short_utf8(c: &mut Criterion) {
    let text = SHORT_TEXT_UTF8.as_bytes();
    let mut group = c.benchmark_group("short-utf8");
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}*/

criterion_group!(
    benches,
    none_0b,
    latin_3kb,
    latin_27b,
    mixed_14kb,
    german_16kb,
    greek_57kb,
    german_100kb,
    greek_152kb,
    english_191kb,
    english_406kb,
    english_971kb,
    german_978kb,
    faust_213kb,
    hungarian_246kb,
    chinese_1mb,
    greek_1_5mb,
    //hamlet,
    //mostly_ascii,
    //long,
    //medium,
    //short,
    //short_utf8
);
criterion_main!(benches);
