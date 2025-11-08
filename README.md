# Webboy
A cycle inaccurate Gameboy Emulator written in Rust. 

### Out of scope
- Gameboy Link cable 
- Gameboy color 


## How should you go about building a Gameboy Emulator?
# 1. The CPU
## Steps
 - Building a register abstraction
 - Dealing with half carries
## Half carry flag
Not well defined in the documentation so adding it here

## Things to note
Instructions are 1 byte but may also require the next 1 or two bytes
following the instruction if the instruction is using a constant value as
an operation parameter

## Test roms
https://github.com/c-sp/game-boy-test-roms?tab=readme-ov-file

## Interrupt handling
Interrupt enabling is all set manually through load instructions via the CPU.
blarggs second test is good for this. First step is to pass their first test
which just ensures the timer interrupt is correctly handled. 

Second step is to pass their vblank interrupt test


#
- You can start from the CPU. GameBoy's instruction set is much larger than chip8, but some are the same so you will be fine. Make it really precise so when you implement other components you don't need to look back at CPU to see what's wrong.
- Then you can implement timer and interrupt. Don't need to be very accurate, just make it works.
- After that you can go implementing PPU(Pixel Processing Unit). Choose a rendering method: whole frame or scanline or pixel FIFO. Imo you should choose a scanline renderer; it stays in between whole frame and pixel FIFO method, and most games work fine with a scanline rendering PPU. Pixel FIFO is the most accurate method, but it is quite hard to understand how it works, and has a lot of quirks.
- If you don't want sound you can skip the APU(Audio Processing Unit) part. APU is quite hard to implement right because you can't see what 's wrong. But well, you don't need to worry it before done implementing above components.
- Then you implement MBCs(Memory Bank Controller). MBC is a circuit lies in the game cartridge control which part of the cartridge data that GameBoy can "see". In order to play games you need to implement it's corresponding MBC. For example, Super Mario Land requires MBC1; Tetris requires no MBC.
- Big Tips: you should use test ROMs to check if your emu works or not. Checkout blargg's cpu_instrs after implementing CPU. You can also use CPU test suites like sm83-test-data, it would help you a lot.

Use blargs tests
but make sure to do
ram[0xFF44] = 0x90; // Set LY to simulate some VBlank progress
to ensure you can run those tests without a proper PPU


# Blarg tests

[ ] Boot

[x] 1

[ ] 2

[x] 3

[x] 4

[x] 5

[x] 6

[x] 7

[x] 8

[x] 9

[x] 10

[x] 11

Note: The boot screen will take a long time to get right

Other roms will also take a very long time to get right


I recommend first rendering out what the tile data looks like and then rendering the background tilemap
When you render the background tilemap