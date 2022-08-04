@echo off
echo launching login server..
start cargo run --bin server_login
echo launching lobby server..
start cargo run --bin server_lobby
echo launching launcher..
start cargo run --bin launcher