@echo off
for %%i in (..\src\protobuf\*.proto) do protoc -I=..\src --csharp_out=.\ "%%i"
pause