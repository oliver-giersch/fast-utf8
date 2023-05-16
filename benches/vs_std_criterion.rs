use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BenchmarkGroup, Criterion};

/// German text, 16'240 characters, 232 thereof non-ASCII.
//const MOSTLY_ASCII: &str = include_str!("../assets/text_utf8");
/// English text, 191'725 plain ASCII characters.
//const HAMLET: &str = include_str!("../assets/hamlet.txt");

//const LONG_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed efficitur quam vitae consequat mattis. Phasellus imperdiet urna tortor, in imperdiet sapien auctor id. In mollis vulputate arcu et rhoncus. Aliquam suscipit consequat eros in accumsan. Nam laoreet purus eu nunc egestas vulputate. Phasellus massa magna, suscipit non ante ut, tempor aliquet purus. Aenean faucibus rhoncus magna egestas interdum. Mauris interdum, enim nec iaculis rhoncus, urna nisi consequat mauris, quis pulvinar magna purus eu lectus. Ut tincidunt metus sit amet ultricies fermentum. Donec gravida imperdiet metus, malesuada tincidunt lectus iaculis quis.
//Integer iaculis odio sodales nibh pulvinar elementum. Donec eu volutpat enim. Fusce malesuada bibendum dolor non consectetur. Mauris a quam auctor, suscipit ligula quis, porttitor arcu. Donec ex lectus, rutrum vitae arcu in, sollicitudin semper libero. In consectetur imperdiet tellus, ut convallis tellus accumsan eu. Proin molestie sem nec ipsum luctus porttitor. Phasellus pulvinar faucibus consectetur.
//Aliquam erat volutpat. Aliquam ut hendrerit odio. Suspendisse aliquet orci sit amet nibh lobortis, at pharetra velit vulputate. Vestibulum ornare lobortis mi, vestibulum efficitur lectus suscipit in. Mauris sollicitudin metus eget elit ornare, eu varius ligula elementum. Curabitur maximus justo non libero luctus ultricies. Pellentesque accumsan purus pulvinar hendrerit efficitur. Curabitur mollis turpis sit amet fermentum auctor. Suspendisse hendrerit mauris sed felis dictum, quis vehicula mauris rhoncus.
//Nullam commodo dolor non est aliquam, at pulvinar nisi consequat. In convallis nunc sit amet nisl vulputate mollis et ut dui. Donec eget neque ac urna pharetra hendrerit. Fusce consequat ipsum id metus mollis facilisis. Fusce at fringilla dui. Morbi sollicitudin tristique ante at malesuada. Sed porttitor sapien sed urna cursus rhoncus. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Morbi et aliquet quam, ac imperdiet odio. Proin et urna velit. Suspendisse accumsan metus dui, ac mollis massa blandit non. Curabitur pulvinar rhoncus facilisis.
//Aenean purus felis, dictum id pharetra eget, auctor ut magna. Vestibulum eu purus non erat malesuada fermentum eget vitae mi. Proin malesuada sem at accumsan facilisis. Curabitur posuere sem non eros condimentum aliquam. Fusce rutrum rhoncus augue, nec pretium tellus mollis et. Proin nec lorem ullamcorper turpis imperdiet consequat sit amet a quam. Morbi commodo ex justo, eu auctor nisl rutrum consectetur. Sed consequat hendrerit eros et tempor. Vestibulum pulvinar aliquet viverra. Maecenas commodo lacus sed congue laoreet. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Suspendisse imperdiet posuere justo et pretium. Sed interdum nisl sapien, ac rhoncus mauris vehicula eu.";
//const MEDIUM_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nulla et eros porta, tincidunt est vitae, pulvinar neque. Phasellus enim nulla, finibus vitae odio ac, iaculis pharetra mi. Aliquam sit amet enim nec felis ornare sagittis vel ut dui. Quisque vitae rhoncus sapien. Donec malesuada enim non lacus bibendum, et suscipit nunc vehicula. Proin eget nunc eget libero mattis elementum. Donec justo quam, scelerisque at erat a, consectetur faucibus risus. Fusce quis aliquet tellus. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Cras aliquet tincidunt euismod.";
//const SHORT_TEXT_UTF8: &str = "Lörem ipsüm dölör sit ämet.";

#[inline(always)]
fn fast_dynamic(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8_dynamic(buf).is_ok()
}

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
    let group_name = format!("{language}/{}/{}%ascii", text_size(text), ascii_ratio(text));

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
    group.bench_function("fast-dynamic", |b| b.iter(|| validate(fast_dynamic, text)));
    group.bench_function("fast-baseline-2x", |b| {
        b.iter(|| validate(fast_baseline_2x, text))
    });
    group.bench_function("fast-baseline-4x", |b| {
        b.iter(|| validate(fast_baseline_4x, text))
    });
    group.bench_function("fast-baseline-8x", |b| {
        b.iter(|| validate(fast_baseline_8x, text))
    });
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
}

fn none_0b(c: &mut Criterion) {
    bench_group(c, "none", b"");
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
