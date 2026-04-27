#!/bin/bash

export RUST_LOG=trace
cargo build


./target/debug/rusn_dos --graphics run ./test/test1 
./target/debug/rusn_dos --graphics run ./test/test2 
./target/debug/rusn_dos --graphics run ./test/test3 
./target/debug/rusn_dos --graphics run ./test/test4 
./target/debug/rusn_dos --graphics run ./app/TASM_W~2.EXE 
./target/debug/rusn_dos --graphics run ./app/test/HARD_E~1.EXE 
./target/debug/rusn_dos --graphics run ./app/test/working.exe 
./target/debug/rusn_dos --graphics run ./app/test/TASM_T~2.EXE 
./target/debug/rusn_dos --graphics run ./app/test/TASM_W~1.EXE 
./target/debug/rusn_dos --graphics run ./app/biing/BIPRO.EXE 
./target/debug/rusn_dos --graphics run ./app/biing/B.EXE 
./target/debug/rusn_dos --graphics run ./app/biing/BINT.EXE 
./target/debug/rusn_dos --graphics run ./app/biing/INSTALL.EXE 