// To compile:
// clang --target=riscv32-none-elf -g -march=rv32imafc -mrelax -O2 bitloader.c -fuse-ld=lld -nostdlib -Wl,--Ttext=0x20000000
// llvm-objcopy -O binary a.out bitloader.bin

#include <stdint.h>

#define BITSTREAM_RAM_SZ        (*(volatile uint32_t *)0x200000fc)
#define BITSTREAM_RAM_LOCATION  ((volatile uint32_t *)0x20000100)

void _start() {
    // Disable interrupts
    asm volatile("csrci mstatus, 8");
    // Switch clock to HSI, so we don't lose it when fabric is reconfigured
    *((volatile uint32_t *)0x0300000c) &= ~3;
    // Enable FCB0 in APB clock enable
    *((volatile uint32_t *)0x03000060) |= 1;
    // Configure FCB to accept a full bitstream in "auto" mode
    *((volatile uint32_t *)0x40010000) = 0x40;

    // Load
    uint32_t num_words = BITSTREAM_RAM_SZ;
    for (uint32_t i = 0; i < num_words; i++) {
        *((volatile uint32_t *)0x4001000c) = BITSTREAM_RAM_LOCATION[i];
    }

    // Finish
    *((volatile uint32_t *)0x40010000) = 0;
    asm volatile("ebreak");

    while (1) {}
}
