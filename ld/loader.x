/* Shared loader, requires MEMORY definitions each chip */

SECTIONS {
    /* Section for code and readonly data, specified by flashloader standard. */
    PrgCode : {
        . = ALIGN(4);

        /* The KEEP is necessary to ensure that the
         * sections don't get garbage collected by the linker.
         * 
         * Because this is not a normal binary with an entry point,
         * the linker would just discard all the code without the
         * KEEP statement here.
         */

        KEEP(*(.text))
        KEEP(*(.text.*))

        KEEP(*(.rodata))
        KEEP(*(.rodata.*))

        KEEP(*(.srodata .srodata.*))

        *(.data .data.*)
        *(.sdata .sdata.*)

        *(.bss .bss.*)
        *(.sbss .sbss.*)
        
        . = ALIGN(4);
    } > IRAM
}


