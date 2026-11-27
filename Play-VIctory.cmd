@echo off
rem Starts the title with scripts\play.ps1 (README.md, "Getting started").
rem Every argument goes to it, e.g.:
rem   Play-VIctory.cmd -Exe <path of your copy's executable> [-Console series-s] [-Windowed]
pwsh -NoProfile -ExecutionPolicy Bypass -File "%~dp0scripts\play.ps1" %*
exit /b %errorlevel%
