#!/bin/bash

export RUST_LOG=info
cargo build

RUST_LOG=info ./target/debug/rusn_dos --graphics run ./test/test1 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./test/test2 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./test/test3 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./test/test4 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/TASM_W~2.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/test/HARD_E~1.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/test/working.exe 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/test/TASM_T~2.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/test/TASM_W~1.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/biing/INSTALL.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/biing/BIPRO.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/biing/B.EXE 2>&1 | grep ERROR
RUST_LOG=info ./target/debug/rusn_dos --graphics run ./app/biing/BINT.EXE 2>&1 | grep ERROR