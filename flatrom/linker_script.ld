OUTPUT_FORMAT("elf32-v810", "elf32-v810", "elf32-v810")
OUTPUT("a.elf") /* Force elf format output */
OUTPUT_ARCH(v810)
TARGET(elf32-v810)
ENTRY(_entry)
SEARCH_DIR(.);

MEMORY
{
    ram (rwx) : ORIGIN = 0x05000000, LENGTH = 64k
    sram (rw) : ORIGIN = 0x06000000, LENGTH = 8M
}

__text_vma = ORIGIN(ram) + 0x0400;
__sram_vma = ORIGIN(sram);
__stack = ORIGIN(ram) + LENGTH(ram) - 64 - 256; /* Extra 256b stack space to leave room for the loader's stack */

SECTIONS
{
    /* Text sections */
    .text __text_vma :
    {
        PROVIDE (__tp = .);
        *(.entry*) /* Ensure the entry point is at the beginning of the binary */
        *(.text*)
    } >ram = 0xff

    /* Read-only data sections */
    .rodata :
    {
        *(.rodata*)
    } >ram = 0xff

    /* Data sections */
    __data_vma = .;
    .data : AT(__data_vma)
    {
        *(.data*)
    } >ram = 0xff

    .sdata :
    {
        PROVIDE (__gp = . + 0x8000);
        *(.sdata*)
    } >ram = 0xff

    __bss_start = .;
    .sbss (NOLOAD):
    {
        *(.sbss*)
        *(.scommon*)
    } >ram = 0xff

    .bss (NOLOAD):
    {
        *(.bss*)
        *(COMMON)
    } >ram = 0xff
    __bss_end = .;

    /* SRAM work area */
    .sram (NOLOAD):
    {
        . = ALIGN(4);
        PROVIDE(__sram_start = .);
        . = ALIGN(4);
        *(.sram*)
        . = ALIGN(4);
        PROVIDE(__sram_end = .);
    } >sram
}
