use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion, SamplingMode};

#[inline(always)]
fn fast(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8(buf).is_ok()
}

#[inline(always)]
fn std(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8_std(buf).is_ok()
}

fn bench_group(c: &mut Criterion, language: &'static str, text: &[u8]) {
    bench_group_sampling(c, language, text, None);
}

fn bench_group_sampling(
    c: &mut Criterion,
    language: &'static str,
    text: &[u8],
    mode: Option<SamplingMode>,
) {
    let group_name = format!(
        "{language}/{}/{}pct-ascii",
        text_size(text),
        ascii_ratio(text)
    );

    let mut group = c.benchmark_group(group_name);
    if let Some(mode) = mode {
        group.sampling_mode(mode);
    }

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
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
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

fn hungarian_52kb(c: &mut Criterion) {
    const HUNGARIAN: &str = include_str!("../assets/hungarian_52kb.txt");
    bench_group(c, "hungarian", HUNGARIAN.as_bytes());
}

fn greek_57kb(c: &mut Criterion) {
    const GREEK: &str = include_str!("../assets/greek_57kb.txt");
    bench_group(c, "greek", GREEK.as_bytes());
}

fn german_100kb(c: &mut Criterion) {
    const GERMAN: &str = include_str!("../assets/german_100kb.txt");
    bench_group(c, "german", GERMAN.as_bytes());
}

fn hungarian_104kb(c: &mut Criterion) {
    const HUNGARIAN: &str = include_str!("../assets/hungarian_104kb.txt");
    bench_group(c, "hungarian", HUNGARIAN.as_bytes());
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

fn spanish_414kb(c: &mut Criterion) {
    const SPANISH: &str = include_str!("../assets/spanish_414kb.txt");
    bench_group(c, "spanish", SPANISH.as_bytes());
}

fn bulgarian_461kb(c: &mut Criterion) {
    const BULGARIAN: &str = include_str!("../assets/bulgarian_461kb.txt");
    bench_group_sampling(
        c,
        "bulgarian",
        BULGARIAN.as_bytes(),
        Some(SamplingMode::Flat),
    );
}

fn hungarian_427kb(c: &mut Criterion) {
    const HUNGARIAN: &str = include_str!("../assets/hungarian_427kb.txt");
    bench_group_sampling(
        c,
        "hungarian",
        HUNGARIAN.as_bytes(),
        Some(SamplingMode::Flat),
    );
}

fn hungarian_889kb(c: &mut Criterion) {
    const HUNGARIAN: &str = include_str!("../assets/hungarian_889kb.txt");
    bench_group_sampling(
        c,
        "hungarian",
        HUNGARIAN.as_bytes(),
        Some(SamplingMode::Flat),
    );
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
    bench_group_sampling(c, "chinese", CHINESE.as_bytes(), Some(SamplingMode::Flat));
}

fn spanish_1_1mb(c: &mut Criterion) {
    const SPANISH: &str = include_str!("../assets/spanish_1_1mb.txt");
    bench_group_sampling(c, "spanish", SPANISH.as_bytes(), Some(SamplingMode::Flat));
}

fn greek_1_5mb(c: &mut Criterion) {
    const GREEK: &str = include_str!("../assets/greek_1_5mb.txt");
    bench_group_sampling(c, "greek", GREEK.as_bytes(), Some(SamplingMode::Flat));
}

criterion_group!(
    benches,
    none_0b,
    latin_3kb,
    latin_27b,
    mixed_14kb,
    german_16kb,
    hungarian_52kb,
    greek_57kb,
    german_100kb,
    hungarian_104kb,
    greek_152kb,
    english_191kb,
    english_406kb,
    spanish_414kb,
    hungarian_427kb,
    bulgarian_461kb,
    hungarian_889kb,
    english_971kb,
    german_978kb,
    faust_213kb,
    hungarian_246kb,
    chinese_1mb,
    spanish_1_1mb,
    greek_1_5mb,
);
criterion_main!(benches);
