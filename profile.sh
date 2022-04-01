#! /bin/sh
BINARY=target/release/plox-example

cargo build --release
valgrind --tool=callgrind --dump-instr=yes --collect-jumps=yes --simulate-cache=yes $BINARY
