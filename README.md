# nom optimization challenge

The goal is to get this library as fast or faster than nom 4.2.

There are two HTTP benchmarks in `benches/`:
- `nom-http.rs` uses nom 4.2
- `http.rs` uses a new parser combinator design, defined in `src/lib.rs`

Here are the results I have right now (running on a late 2013 Macbook Pro):

```
Running target/release/deps/http-49395a45c3a98734

running 4 tests
test bigger_test           ... bench:     328,234 ns/iter (+/- 16,034) = 325 MB/s
test httparse_example_test ... bench:       1,499 ns/iter (+/- 358) = 468 MB/s
test one_test              ... bench:         941 ns/iter (+/- 126) = 309 MB/s
test small_test            ... bench:      63,261 ns/iter (+/- 12,063) = 337 MB/s

test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured

Running target/release/deps/json-26693d6b4b918e1f

running 1 test
test manytest ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out

Running target/release/deps/nom_http-897516bd33a05864

running 4 tests
test bigger_test           ... bench:     295,794 ns/iter (+/- 42,071) = 361 MB/s
test httparse_example_test ... bench:       1,347 ns/iter (+/- 78) = 521 MB/s
test one_test              ... bench:         800 ns/iter (+/- 45) = 363 MB/s
test small_test            ... bench:      56,932 ns/iter (+/- 9,422) = 375 MB/s
```

To run the tests, do `cargo +stable bench`. Stable (1.32) and beta versions of Rust will
be fine, but nightly can be a bit capricious.

If we can prove that this design can get as fast as the current nom version (or at least
get closer than 5% perf difference), I'll get to work to release a nom version 5.0
that will integrate it with a nice, type checked API, and have the macros use it under
the hood, to keep it backward compatible with older code.
Also, I have a feeling it could fix the UX issue around `Incomplete` usage better than
the `CompleteStr` and `CompleteByteSlice` types :)

So, please help me optimize this!
