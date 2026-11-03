rem @echo off
c:
cd \test
cd
c:\tasm\bin\tasm.exe testvga.asm
if errorlevel 1 goto fail
c:\tasm\bin\tlink.exe /t testvga.obj
goto end
:fail
echo BUILD FAILED
exit 1
:end
echo BUILD OK