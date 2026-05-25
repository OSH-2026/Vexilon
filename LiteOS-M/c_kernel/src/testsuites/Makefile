CC=gcc
CFLAGS=-O2 -Wall --coverage -Iinclude -I../include

SRC=$(wildcard *.c)

all:
	$(CC) $(CFLAGS) $(SRC) -o kernel_test

run:
	./kernel_test

coverage:
	gcov *.c
