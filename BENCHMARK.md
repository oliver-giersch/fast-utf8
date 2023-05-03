# Raspberry Pi 4 Model B Rev 1.1 (aarch64)

with `target-cpu=native`:

```
running 12 tests
test validate_fast_hamlet         ... bench:      28,192 ns/iter (+/- 155)
test validate_fast_long_utf8      ... bench:         433 ns/iter (+/- 0)
test validate_fast_medium         ... bench:         122 ns/iter (+/- 0)
test validate_fast_short          ... bench:          34 ns/iter (+/- 0)
test validate_fast_short_utf8     ... bench:         155 ns/iter (+/- 0)
test validate_fast_very_long_utf8 ... bench:      11,398 ns/iter (+/- 357)

test validate_std_hamlet          ... bench:      32,337 ns/iter (+/- 73)
test validate_std_long_utf8       ... bench:         526 ns/iter (+/- 0)
test validate_std_medium          ... bench:         140 ns/iter (+/- 0)
test validate_std_short           ... bench:          37 ns/iter (+/- 0)
test validate_std_short_utf8      ... bench:         127 ns/iter (+/- 0)
test validate_std_very_long_utf8  ... bench:      17,521 ns/iter (+/- 308)
```

# Intel i5-6500

w/o `target-cpu=native`

```
test fast_hamlet       ... bench:       6,847 ns/iter (+/- 122)
test fast_long_utf8    ... bench:         110 ns/iter (+/- 2)
test fast_medium       ... bench:          29 ns/iter (+/- 0)
test fast_mostly_ascii ... bench:       3,217 ns/iter (+/- 99)
test fast_short        ... bench:          11 ns/iter (+/- 0)
test fast_short_utf8   ... bench:          28 ns/iter (+/- 0)

test std_hamlet        ... bench:       6,759 ns/iter (+/- 55)
test std_long_utf8     ... bench:         119 ns/iter (+/- 2)
test std_medium        ... bench:          28 ns/iter (+/- 0)
test std_mostly_ascii  ... bench:       2,916 ns/iter (+/- 73)
test std_short         ... bench:          10 ns/iter (+/- 0)
test std_short_utf8    ... bench:          28 ns/iter (+/- 0)
```

with `target-cpu=native`:

```
running 12 tests
test fast_hamlet       ... bench:       4,811 ns/iter (+/- 114)
test fast_long_utf8    ... bench:          72 ns/iter (+/- 5)
test fast_medium       ... bench:          20 ns/iter (+/- 0)
test fast_mostly_ascii ... bench:       2,516 ns/iter (+/- 78)
test fast_short        ... bench:          10 ns/iter (+/- 0)
test fast_short_utf8   ... bench:          25 ns/iter (+/- 0)

test std_hamlet        ... bench:       6,707 ns/iter (+/- 82)
test std_long_utf8     ... bench:         118 ns/iter (+/- 2)
test std_medium        ... bench:          28 ns/iter (+/- 1)
test std_mostly_ascii  ... bench:       2,887 ns/iter (+/- 36)
test std_short         ... bench:          11 ns/iter (+/- 0)
test std_short_utf8    ... bench:          29 ns/iter (+/- 0)
```

# Intel i5-12400

with `target-cpu=native`:

```
running 12 tests
test validate_fast_hamlet         ... bench:       3,462 ns/iter (+/- 323)
test validate_fast_long_utf8      ... bench:          68 ns/iter (+/- 4)
test validate_fast_medium         ... bench:          17 ns/iter (+/- 5)
test validate_fast_short          ... bench:           7 ns/iter (+/- 1)
test validate_fast_short_utf8     ... bench:          29 ns/iter (+/- 2)
test validate_fast_very_long_utf8 ... bench:       1,730 ns/iter (+/- 111)

test validate_std_hamlet          ... bench:       4,388 ns/iter (+/- 992)
test validate_std_long_utf8       ... bench:          80 ns/iter (+/- 7)
test validate_std_medium          ... bench:          26 ns/iter (+/- 2)
test validate_std_short           ... bench:           5 ns/iter (+/- 0)
test validate_std_short_utf8      ... bench:          16 ns/iter (+/- 1)
test validate_std_very_long_utf8  ... bench:       2,104 ns/iter (+/- 240)
```