@echo off
testcmd 7
if errorlevel 7 goto seven
echo WRONG LEVEL
exit 0
:seven
echo SEVEN OK
exit 9