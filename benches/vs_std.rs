#![feature(test)]

extern crate test;

use std::hint::black_box;

use test::Bencher;

/// German text, 16240 characters, 232 thereof non-ASCII.
const VERY_LONG_TEXT_UTF8: &str = include_str!("../assets/text_utf8");
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
    fast_utf8::validate_utf8(buf)
}

#[inline(always)]
fn std(buf: &[u8]) -> bool {
    std::str::from_utf8(buf).is_ok()
}

#[inline(always)]
fn validate(f: fn(&[u8]) -> bool, text: &[u8]) {
    let ok = black_box(f(black_box(text)));
    assert!(black_box(ok));
}

#[bench]
fn fast_hamlet(b: &mut Bencher) {
    let text = HAMLET.as_bytes();
    b.iter(|| validate(fast, text));
}

#[bench]
fn std_hamlet(b: &mut Bencher) {
    let text = HAMLET.as_bytes();
    b.iter(|| validate(std, text));
}

#[bench]
fn fast_mostly_ascii(b: &mut Bencher) {
    let text = VERY_LONG_TEXT_UTF8.as_bytes();
    b.iter(|| validate(fast, text));
}

#[bench]
fn std_mostly_ascii(b: &mut Bencher) {
    let text = VERY_LONG_TEXT_UTF8.as_bytes();
    b.iter(|| validate(std, text));
}

#[bench]
fn fast_long_utf8(b: &mut Bencher) {
    let text = LONG_TEXT.as_bytes();
    b.iter(|| validate(fast, text));
}

#[bench]
fn std_long_utf8(b: &mut Bencher) {
    let text = LONG_TEXT.as_bytes();
    b.iter(|| validate(std, text));
}

#[bench]
fn fast_medium(b: &mut Bencher) {
    let text = MEDIUM_TEXT.as_bytes();
    b.iter(|| validate(fast, text));
}

#[bench]
fn std_medium(b: &mut Bencher) {
    let text = MEDIUM_TEXT.as_bytes();
    b.iter(|| validate(std, text));
}

#[bench]
fn fast_short(b: &mut Bencher) {
    let text = SHORT_TEXT.as_bytes();
    b.iter(|| validate(fast, text));
}

#[bench]
fn std_short(b: &mut Bencher) {
    let text = SHORT_TEXT.as_bytes();
    b.iter(|| validate(std, text));
}

#[bench]
fn fast_short_utf8(b: &mut Bencher) {
    let text = SHORT_TEXT_UTF8.as_bytes();
    b.iter(|| validate(fast, text));
}

#[bench]
fn std_short_utf8(b: &mut Bencher) {
    let text = SHORT_TEXT_UTF8.as_bytes();
    b.iter(|| validate(std, text));
}