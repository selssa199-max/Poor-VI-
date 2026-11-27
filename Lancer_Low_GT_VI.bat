@echo off
title Low GT VI - Launcher (RX 6600 XT / 16 GB RAM)
cd /d "%~dp0"
powershell -ExecutionPolicy Bypass -Command "& '.\scripts\play.ps1' -Exe 'C:\Users\elsas\Desktop\gfta6 cyberleek\game_scarlett_master.exe' -Build release -More @{VICTORY_RELAY='0'; VICTORY_TEXTURE_MAX='512'; VICTORY_TEXTURE_MAX_MASK='256'; VICTORY_CARD_BUFFERS_MB='2048'; VICTORY_CARD_WINDOWS_MB='1024'; VICTORY_MAP_THREADS='1'; VICTORY_DSTORAGE_WORKERS='6'}"
pause
