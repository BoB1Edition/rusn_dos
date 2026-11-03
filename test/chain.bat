@echo off
echo param1=%1
set GREET=HELLO
echo var=%GREET%
cd \
cd test
testcmd 3
if errorlevel 3 echo LEVEL-GE-3 OK
call fail.bat
echo after call
exit 0