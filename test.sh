#!/bin/bash

export RUST_LOG=warn
cargo build

echo ./test/test1
./target/debug/rusn_dos --graphics run ./app/test/test1
echo ./test/test2
./target/debug/rusn_dos --graphics run ./app/test/test2
echo ./test/test3
./target/debug/rusn_dos --graphics run ./app/test/test3
echo ./test/test4
./target/debug/rusn_dos --graphics run ./app/test/test4
echo ./app/TASM_W~2.EXE 
./target/debug/rusn_dos --graphics run ./app/TASM_W~2.EXE 
echo ./app/test/HARD_E~1.EXE 
./target/debug/rusn_dos --graphics run ./app/test/HARD_E~1.EXE 
echo ./app/test/working.exe 
./target/debug/rusn_dos --graphics run ./app/test/working.exe 
echo ./app/test/TASM_T~2.EXE 
./target/debug/rusn_dos --graphics run ./app/test/TASM_T~2.EXE 
echo ./app/test/TASM_W~1.EXE 
./target/debug/rusn_dos --graphics run ./app/test/TASM_W~1.EXE 
echo ./app/test/TESTVGA.COM
./target/debug/rusn_dos --graphics run ./app/test/TESTVGA.COM 
echo ./app/biing/BIPRO.EXE 
./target/debug/rusn_dos --graphics run -w c:\\biing ./app/biing/BIPRO.EXE 
echo ./app/biing/B.EXE 
./target/debug/rusn_dos --graphics run -w c:\\biing ./app/biing/B.EXE 
echo ./app/biing/BINT.EXE 
./target/debug/rusn_dos --graphics run -w c:\\biing ./app/biing/BINT.EXE 
echo ./app/biing/INSTALL.EXE 
./target/debug/rusn_dos --graphics run -w c:\\biing ./app/biing/INSTALL.EXE 
echo ./app/XCOM/INTRO.EXE
./target/debug/rusn_dos --graphics run -w c:\\xcom ./app/XCOM/INTRO.EXE
echo ./app/XCOM/SETUP.EXE
./target/debug/rusn_dos --graphics run -w c:\\xcom ./app/XCOM/SETUP.EXE
echo ./app/XCOM/GO.COM
./target/debug/rusn_dos --graphics run -w c:\\xcom ./app/XCOM/GO.COM
echo ./app/TASM/BIN/tlink.exe /t testcmd.obj
./target/debug/rusn_dos --graphics run -w c:\\test ./app/TASM/BIN/tlink.exe /t testcmd.obj
echo ./app/test/build.bat
./target/debug/rusn_dos --graphics run ./app/test/build.bat
echo ./app/test/fail.bat
./target/debug/rusn_dos --graphics run ./app/test/fail.bat
echo --max_call_depth 3 ./app/test/recurse.bat
./target/debug/rusn_dos --graphics --max-call-depth 3 run ./app/test/recurce.bat
echo --max_call_depth 300 ./app/test/recurse.bat
./target/debug/rusn_dos --graphics --max-call-depth 300 run ./app/test/recurse.bat
echo  ./app/test/chain.bat hello
./target/debug/rusn_dos --graphics run ./app/test/chain.bat hello
echo  -w c:\\tasm\\bin grep.com ?
./target/debug/rusn_dos --graphics run -w c:\\tasm\\bin grep.com ?