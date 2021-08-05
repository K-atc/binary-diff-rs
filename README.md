binary-diff-rs
====

Binary diff library & tool written in Rust.

**NOTE:** Algorithms of calculation on diff are **very unstable** since I'm thinking out good ones to describe what happened in mutations in Fuzzing. Expected behavior is documented as unit tests.


Requirements
----
* Rust & Cargo


How to install
----
```shell
cargo install --git https://github.com/K-atc/binary-diff-rs.git --features bin
```


How to build
----
```shell
cargo build --features bin
```


Running examples
----
### Seed files comparison
Files to be compared:

```
### Original file
$ xxd tests/samples/binary/573a46286deaf9df81fb90d7b786708d845b5f23
00000000: 2e03 0000 0302 da03 1803 1800 0016 0300  ................
00000010: 000b fee3 b7fd 0003 0003 02da 00         .............

### Patched file
$ xxd tests/samples/binary/c298122410da09836c59484e995c287294c31394
00000000: 2e03 0000 0302 da03 1803 1800 0018 0300  ................
00000010: 000b 0100 03fe 0003 0003 1dda 00         .............
```

This crate calculates difference as follows:

```
$ cargo run -q --features bin -- --same tests/samples/binary/seeds/*
Same   (offset=0x0, length=0xd)
Replace(offset=0xd, length=0x1, bytes=[18])
Same   (offset=0xe, length=0x4)
Insert (offset=0x12, bytes=[01 00 03])
Same   (offset=0x12, length=0x1)
Delete (offset=0x13, length=0x3)
Same   (offset=0x16, length=0x4)
Replace(offset=0x1a, length=0x1, bytes=[1d])
Same   (offset=0x1b, length=0x2)
```

For example, chunk `Insert(offset=0x12, bytes=[01 00 03])` states that bytes `[01 00 03]` are inserted at offset 0x12 of *original file*.
We can see that its bytes locates in offset 0x12 of *patched file*.

Using `--offset` option, we can see that offset 0x12 of patched file derives from chunk `Insert(offset=0x12, bytes=[01 00 03])`.

```
$ cargo run -q --features bin -- tests/samples/binary/seeds/* --offset 12
Insert (offset=0x12, bytes=[01 00 03])
```

### Inspecting minimized crash input
Files to be compared:

```
### Original file
$ xxd tests/samples/binary/crash-minimization/crash-235641cefe524570bf0df6a3b3722535ce2dbbf7
00000000: 5c53 3f5c 435c 533f 5c43 d5ac 322a d5ac  \S?\C\S?\C..2*..
00000010: 435c 5316                                C\S.

### Patched file
$ xxd tests/samples/binary/crash-minimization/minimized-from-10dad543216eabe6d97b9d0ba8459215f6dca3f3
00000000: 5c43 5c53 3fd5 ac16 5c16                 \C\S?...\.
```

This crate calculates difference as follows:

```
$ cargo run -q --features bin tests/samples/binary/crash-minimization/* --same
Same   (offset=0x0, length=0x1)
Delete (offset=0x1, length=0x3)
Same   (offset=0x4, length=0x4)
Delete (offset=0x8, length=0x2)
Same   (offset=0xa, length=0x2)
Replace(offset=0xc, length=0x5, bytes=[16])
Same   (offset=0x11, length=0x1)
Delete (offset=0x12, length=0x1)
Same   (offset=0x13, length=0x1)
```

We can inspect which bytes are deleted easily :smile: