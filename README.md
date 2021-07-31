binary-diff-rs
====

Binary diff library & tool written in Rust


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


This crate inferences diff chunks as follows:

```
$ cargo run -q --features bin -- --same tests/samples/binary/*
Same   (offset=0x0, length=0xd)
Replace(offset=0xd, length=0x1, bytes=[18])
Same   (offset=0xe, length=0x4)
Replace(offset=0x12, length=0x4, bytes=[01 00 03 fe])
Same   (offset=0x16, length=0x4)
Replace(offset=0x1a, length=0x1, bytes=[1d])
Same   (offset=0x1b, length=0x2)
```

Based on above diff, offset 0x12 of patched file derives from chunk `Insert(offset=0x16, bytes=[01 00 03 fe])`.

```
$ cargo run -q --features bin -- tests/samples/binary/* --offset 12
Insert (offset=0x16, bytes=[01 00 03 fe])
```
