# summing

A mathematical puzzle game for the terminal — a rewrite of
["Summing for PalmOS"](https://palmdb.net/app/summing-math).

![summing screenshot](https://raw.githubusercontent.com/xitep/summing/refs/heads/main/summing.png)

## How to play

1. You're given a grid of 7x7 numbers on a board of 9x9 tiles
2. You're also given a stream of random numbers of which you see the next four to come
3. Place the next random number on a free tile on the board such that the last digit of neighbours' sum equals the placed number. If there's a match, the newly placed random number as well as the neighbours get cleared.
4. Repeat the previous step until the board is either empty or full

You're goal is to clear the board in as few placements as possible.
