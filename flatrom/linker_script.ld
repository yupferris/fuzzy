OUTPUT_FORMAT("elf32-v810", "elf32-v810", "elf32-v810")
OUTPUT("a.elf") /* force elf format output */
OUTPUT_ARCH(v810)
TARGET(elf32-v810)
ENTRY(_entry)
SEARCH_DIR(.);

MEMORY
{
    ram (rwx)    : ORIGIN =  0x05000000, LENGTH = 64k /*64k*/
}

__text_vma = ORIGIN(ram) + 0x2000;

SECTIONS
{
    .text __text_vma :
    {
        *(.text*)
    }
}
