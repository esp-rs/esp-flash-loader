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

        KEEP(*(.literal))
        KEEP(*(.literal.*))

        KEEP(*(.rodata))
        KEEP(*(.rodata.*))

        KEEP(*(.text))
        KEEP(*(.text.*))

        KEEP(*(.srodata .srodata.*))

        . = ALIGN(4);
    } > IRAM

    /* Description of the flash algorithm */
    DeviceData : {
        /* The device data content is only for external tools,
         * and usually not referenced by the code.
         *
         * The KEEP statement ensures it's not removed by accident.
         */
        KEEP(*(DeviceData))
    } > IRAM

    /* TODO: these section names are non-standard, but target-gen has no concept of separate instruction and data busses */

    bss (NOLOAD) : ALIGN(4)
    {
        _bss_start = ABSOLUTE(.);
        . = ALIGN (4);
        *(.sbss .sbss.* .bss .bss.*);
        _bss_end = ABSOLUTE(.);
        . = ALIGN(4);
    } > RWDATA

    rwdata : {
        *(.data .data.*)
        *(.sdata .sdata.*)

        . = ORIGIN(RWDATA) + LENGTH(RWDATA);
        _stack_start = ABSOLUTE(.);
    } > RWDATA
}
