@echo off
set /p max="Max player count: "
..\target\debug\mscg_server %max%
pause