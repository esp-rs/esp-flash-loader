/*  The flash loader will be placed in RAM by the debugger,
 *  so we don't need to specify any memory areas here.
 */

SECTIONS {
    . = 0x0;

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
        
        . = ALIGN(4);
    }

    /* Section for data, specified by flashloader standard. */
    PrgData : {
        *(.data .data.*)
        *(.sdata .sdata.*)

    }

    PrgData : {
        /* Zero-initialized data */
        *(.bss .bss.*)
        *(.sbss .sbss.*)

        *(COMMON)
    }

    /* Description of the flash algorithm */
    DeviceData . : {
        /* The device data content is only for external tools,
         * and usually not referenced by the code.
         *
         * The KEEP statement ensures it's not removed by accident.
         */
        KEEP(*(DeviceData))
    }
}


