use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

const CHINESE_1MB: &str = include_str!("../assets/chinese_1mb.txt");

/// German text, 16'240 characters, 232 thereof non-ASCII.
const MOSTLY_ASCII: &str = include_str!("../assets/text_utf8");
/// English text, 191'725 plain ASCII characters.
const HAMLET: &str = include_str!("../assets/hamlet.txt");

const LONG_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed efficitur quam vitae consequat mattis. Phasellus imperdiet urna tortor, in imperdiet sapien auctor id. In mollis vulputate arcu et rhoncus. Aliquam suscipit consequat eros in accumsan. Nam laoreet purus eu nunc egestas vulputate. Phasellus massa magna, suscipit non ante ut, tempor aliquet purus. Aenean faucibus rhoncus magna egestas interdum. Mauris interdum, enim nec iaculis rhoncus, urna nisi consequat mauris, quis pulvinar magna purus eu lectus. Ut tincidunt metus sit amet ultricies fermentum. Donec gravida imperdiet metus, malesuada tincidunt lectus iaculis quis.
Integer iaculis odio sodales nibh pulvinar elementum. Donec eu volutpat enim. Fusce malesuada bibendum dolor non consectetur. Mauris a quam auctor, suscipit ligula quis, porttitor arcu. Donec ex lectus, rutrum vitae arcu in, sollicitudin semper libero. In consectetur imperdiet tellus, ut convallis tellus accumsan eu. Proin molestie sem nec ipsum luctus porttitor. Phasellus pulvinar faucibus consectetur.
Aliquam erat volutpat. Aliquam ut hendrerit odio. Suspendisse aliquet orci sit amet nibh lobortis, at pharetra velit vulputate. Vestibulum ornare lobortis mi, vestibulum efficitur lectus suscipit in. Mauris sollicitudin metus eget elit ornare, eu varius ligula elementum. Curabitur maximus justo non libero luctus ultricies. Pellentesque accumsan purus pulvinar hendrerit efficitur. Curabitur mollis turpis sit amet fermentum auctor. Suspendisse hendrerit mauris sed felis dictum, quis vehicula mauris rhoncus.
Nullam commodo dolor non est aliquam, at pulvinar nisi consequat. In convallis nunc sit amet nisl vulputate mollis et ut dui. Donec eget neque ac urna pharetra hendrerit. Fusce consequat ipsum id metus mollis facilisis. Fusce at fringilla dui. Morbi sollicitudin tristique ante at malesuada. Sed porttitor sapien sed urna cursus rhoncus. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Morbi et aliquet quam, ac imperdiet odio. Proin et urna velit. Suspendisse accumsan metus dui, ac mollis massa blandit non. Curabitur pulvinar rhoncus facilisis.
Aenean purus felis, dictum id pharetra eget, auctor ut magna. Vestibulum eu purus non erat malesuada fermentum eget vitae mi. Proin malesuada sem at accumsan facilisis. Curabitur posuere sem non eros condimentum aliquam. Fusce rutrum rhoncus augue, nec pretium tellus mollis et. Proin nec lorem ullamcorper turpis imperdiet consequat sit amet a quam. Morbi commodo ex justo, eu auctor nisl rutrum consectetur. Sed consequat hendrerit eros et tempor. Vestibulum pulvinar aliquet viverra. Maecenas commodo lacus sed congue laoreet. Orci varius natoque penatibus et magnis dis parturient montes, nascetur ridiculus mus. Suspendisse imperdiet posuere justo et pretium. Sed interdum nisl sapien, ac rhoncus mauris vehicula eu.";
const MEDIUM_TEXT: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nulla et eros porta, tincidunt est vitae, pulvinar neque. Phasellus enim nulla, finibus vitae odio ac, iaculis pharetra mi. Aliquam sit amet enim nec felis ornare sagittis vel ut dui. Quisque vitae rhoncus sapien. Donec malesuada enim non lacus bibendum, et suscipit nunc vehicula. Proin eget nunc eget libero mattis elementum. Donec justo quam, scelerisque at erat a, consectetur faucibus risus. Fusce quis aliquet tellus. Lorem ipsum dolor sit amet, consectetur adipiscing elit. Cras aliquet tincidunt euismod.";
const SHORT_TEXT: &str = "Lorem ipsum dolor sit amet.";
const SHORT_TEXT_UTF8: &str = "Lörem ipsüm dölör sit ämet.";

#[inline(always)]
fn fast(buf: &[u8]) -> bool {
    fast_utf8::validate_utf8(buf).is_ok()
}

#[inline(always)]
fn std(buf: &[u8]) -> bool {
    std::str::from_utf8(buf).is_ok()
}

fn group_name<'a>(bytes: &'a [u8], language: &'static str) -> (&'a [u8], String) {
    (
        bytes,
        format!(
            "{language}/{}/{}%ascii",
            text_size(bytes),
            ascii_ratio(bytes)
        ),
    )
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

fn none_0b(c: &mut Criterion) {
    let (text, group_name) = group_name(b"", "none");

    let mut group = c.benchmark_group(group_name);
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn faust_213kb(c: &mut Criterion) {
    const FAUST: &str = include_str!("../assets/faust_213kb.txt");
    let (text, group_name) = group_name(FAUST.as_bytes(), "german");

    let mut group = c.benchmark_group(group_name);
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn hungarian_246kb(c: &mut Criterion) {
    const HUNGARIAN: &str = include_str!("../assets/hungarian_246kb.txt");
    let (text, group_name) = group_name(HUNGARIAN.as_bytes(), "hungarian");

    let mut group = c.benchmark_group(group_name);
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn chinese_1mb(c: &mut Criterion) {
    let text = CHINESE_1MB.as_bytes();
    let group_name = format!("chinese/1mb/{}%ascii", ascii_ratio(CHINESE_1MB.as_bytes()));

    let mut group = c.benchmark_group(group_name);
    group.sampling_mode(criterion::SamplingMode::Flat);

    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn english_191kb(c: &mut Criterion) {
    const ENGLISH: &str = include_str!("../assets/hamlet.txt");
    let (text, group_name) = group_name(ENGLISH.as_bytes(), "english");

    let mut group = c.benchmark_group(group_name);
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn hamlet(c: &mut Criterion) {
    let text = HAMLET.as_bytes();
    let mut group = c.benchmark_group("hamlet");
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn mostly_ascii(c: &mut Criterion) {
    let text = MOSTLY_ASCII.as_bytes();
    let mut group = c.benchmark_group("mostly-ascii");
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn long(c: &mut Criterion) {
    let text = LONG_TEXT.as_bytes();
    let mut group = c.benchmark_group("long");
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
    group.finish();
}

fn medium(c: &mut Criterion) {
    let text = MEDIUM_TEXT.as_bytes();
    let mut group = c.benchmark_group("medium");
    group.bench_function("fast", |b| b.iter(|| validate(fast, text)));
    group.bench_function("std", |b| b.iter(|| validate(std, text)));
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
}

criterion_group!(
    benches,
    none_0b,
    english_191kb,
    faust_213kb,
    hungarian_246kb,
    chinese_1mb,
    hamlet,
    mostly_ascii,
    long,
    medium,
    short,
    short_utf8
);
criterion_main!(benches);
