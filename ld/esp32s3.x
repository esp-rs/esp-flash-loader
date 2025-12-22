MEMORY {
    /* SRAM2 + 0x8400 */
    IRAM : ORIGIN = 0x40380400, LENGTH = 0x10000
    RWDATA : ORIGIN = 0x3FCB0000, LENGTH = 0x20000
}

INCLUDE "loader.x"

PROVIDE( ets_delay_us = 0x40000600 );
PROVIDE ( esp_rom_spiflash_attach = spi_flash_attach );

/***************************************
 Group miniz
 ***************************************/

/* Functions */
mz_adler32 = 0x4000078c;
mz_crc32 = 0x40000798;
mz_free = 0x400007a4;
tdefl_compress = 0x400007b0;
tdefl_compress_buffer = 0x400007bc;
tdefl_compress_mem_to_heap = 0x400007c8;
tdefl_compress_mem_to_mem = 0x400007d4;
tdefl_compress_mem_to_output = 0x400007e0;
tdefl_get_adler32 = 0x400007ec;
tdefl_get_prev_return_status = 0x400007f8;
tdefl_init = 0x40000804;
tdefl_write_image_to_png_file_in_memory = 0x40000810;
tdefl_write_image_to_png_file_in_memory_ex = 0x4000081c;
tinfl_decompress = 0x40000828;
tinfl_decompress_mem_to_callback = 0x40000834;
tinfl_decompress_mem_to_heap = 0x40000840;
tinfl_decompress_mem_to_mem = 0x4000084c;

/***************************************
 Group opi_flash
 ***************************************/

/* Functions */
PROVIDE( opi_flash_set_lock_func = 0x40000870 );
PROVIDE( esp_rom_spi_cmd_config = 0x4000087c );
PROVIDE( esp_rom_spi_cmd_start = 0x40000888 );
PROVIDE( esp_rom_opiflash_pin_config = 0x40000894 );
PROVIDE( esp_rom_spi_set_op_mode = 0x400008a0 );
PROVIDE( esp_rom_opiflash_mode_reset = 0x400008ac );
PROVIDE( esp_rom_opiflash_exec_cmd = 0x400008b8 );
PROVIDE( esp_rom_opiflash_soft_reset = 0x400008c4 );
PROVIDE( esp_rom_opiflash_read_id = 0x400008d0 );
PROVIDE( esp_rom_opiflash_rdsr = 0x400008dc );
PROVIDE( esp_rom_opiflash_wait_idle = 0x400008e8 );
PROVIDE( esp_rom_opiflash_wren = 0x400008f4 );
PROVIDE( esp_rom_opiflash_erase_sector = 0x40000900 );
PROVIDE( esp_rom_opiflash_erase_block_64k = 0x4000090c );
PROVIDE( esp_rom_opiflash_erase_area = 0x40000918 );
PROVIDE( esp_rom_opiflash_read = 0x40000924 );
PROVIDE( esp_rom_opiflash_write = 0x40000930 );
PROVIDE( esp_rom_spi_set_dtr_swap_mode = 0x4000093c );
PROVIDE( esp_rom_opiflash_exit_continuous_read_mode = 0x40000948 );
PROVIDE( esp_rom_opiflash_legacy_driver_init = 0x40000954 );
PROVIDE( esp_rom_opiflash_read_raw = 0x4004d9d4);

/***************************************
 Group spiflash_legacy
 ***************************************/

/* Functions */
PROVIDE( esp_rom_spiflash_wait_idle = 0x40000960 );
PROVIDE( esp_rom_spiflash_write_encrypted = 0x4000096c );
PROVIDE( esp_rom_spiflash_write_encrypted_dest = 0x40000978 );
PROVIDE( esp_rom_spiflash_write_encrypted_enable = 0x40000984 );
PROVIDE( esp_rom_spiflash_write_encrypted_disable = 0x40000990 );
PROVIDE( esp_rom_spiflash_erase_chip = 0x4000099c );
PROVIDE( _esp_rom_spiflash_erase_sector = 0x400009a8 );
PROVIDE( _esp_rom_spiflash_erase_block = 0x400009b4 );
PROVIDE( _esp_rom_spiflash_write = 0x400009c0 );
PROVIDE( _esp_rom_spiflash_read = 0x400009cc );
PROVIDE( _esp_rom_spiflash_unlock = 0x400009d8 );
PROVIDE( _SPIEraseArea = 0x400009e4 );
PROVIDE( _SPI_write_enable = 0x400009f0 );
PROVIDE( esp_rom_spiflash_erase_sector = 0x400009fc );
PROVIDE( esp_rom_spiflash_erase_block = 0x40000a08 );
PROVIDE( esp_rom_spiflash_write = 0x40000a14 );
PROVIDE( esp_rom_spiflash_read = 0x40000a20 );
PROVIDE( esp_rom_spiflash_unlock = 0x40000a2c );
PROVIDE( SPIEraseArea = 0x40000a38 );
PROVIDE( SPI_write_enable = 0x40000a44 );
PROVIDE( esp_rom_spiflash_config_param = 0x40000a50 );
PROVIDE( esp_rom_spiflash_read_user_cmd = 0x40000a5c );
PROVIDE( esp_rom_spiflash_select_qio_pins = 0x40000a68 );
PROVIDE( esp_rom_spi_flash_auto_sus_res = 0x40000a74 );
PROVIDE( esp_rom_spi_flash_send_resume = 0x40000a80 );
PROVIDE( esp_rom_spi_flash_update_id = 0x40000a8c );
PROVIDE( esp_rom_spiflash_config_clk = 0x40000a98 );
PROVIDE( esp_rom_spiflash_config_readmode = 0x40000aa4 );
PROVIDE( esp_rom_spiflash_read_status = 0x40000ab0 );
PROVIDE( esp_rom_spiflash_read_statushigh = 0x40000abc );
PROVIDE( esp_rom_spiflash_write_status = 0x40000ac8 );
PROVIDE( esp_rom_opiflash_cache_mode_config = 0x40000ad4 );
PROVIDE( esp_rom_spiflash_auto_wait_idle = 0x40000ae0 );
PROVIDE( spi_flash_attach = 0x40000aec );
PROVIDE( spi_flash_get_chip_size = 0x40000af8 );
PROVIDE( spi_flash_guard_set = 0x40000b04 );
PROVIDE( spi_flash_guard_get = 0x40000b10 );
PROVIDE( spi_flash_write_config_set = 0x40000b1c );
PROVIDE( spi_flash_write_config_get = 0x40000b28 );
PROVIDE( spi_flash_safe_write_address_func_set = 0x40000b34 );
PROVIDE( spi_flash_unlock = 0x40000b40 );
PROVIDE( spi_flash_erase_range = 0x40000b4c );
PROVIDE( spi_flash_erase_sector = 0x40000b58 );
PROVIDE( spi_flash_write = 0x40000b64 );
PROVIDE( spi_flash_read = 0x40000b70 );
PROVIDE( spi_flash_write_encrypted = 0x40000b7c );
PROVIDE( spi_flash_read_encrypted = 0x40000b88 );
PROVIDE( spi_flash_mmap_os_func_set = 0x40000b94 );
PROVIDE( spi_flash_mmap_page_num_init = 0x40000ba0 );
PROVIDE( spi_flash_mmap = 0x40000bac );
PROVIDE( spi_flash_mmap_pages = 0x40000bb8 );
PROVIDE( spi_flash_munmap = 0x40000bc4 );
PROVIDE( spi_flash_mmap_dump = 0x40000bd0 );
PROVIDE( spi_flash_check_and_flush_cache = 0x40000bdc );
PROVIDE( spi_flash_mmap_get_free_pages = 0x40000be8 );
PROVIDE( spi_flash_cache2phys = 0x40000bf4 );
PROVIDE( spi_flash_phys2cache = 0x40000c00 );
PROVIDE( spi_flash_disable_cache = 0x40000c0c );
PROVIDE( spi_flash_restore_cache = 0x40000c18 );
PROVIDE( spi_flash_cache_enabled = 0x40000c24 );
PROVIDE( spi_flash_enable_cache = 0x40000c30 );
PROVIDE( spi_cache_mode_switch = 0x40000c3c );
PROVIDE( spi_common_set_dummy_output = 0x40000c48 );
PROVIDE( spi_common_set_flash_cs_timing = 0x40000c54 );
PROVIDE( esp_rom_spi_set_address_bit_len = 0x40000c60 );
PROVIDE( esp_enable_cache_flash_wrap = 0x40000c6c );
PROVIDE( SPILock = 0x40000c78 );
PROVIDE( SPIMasterReadModeCnfig = 0x40000c84 );
PROVIDE( SPI_Common_Command = 0x40000c90 );
PROVIDE( SPI_WakeUp = 0x40000c9c );
PROVIDE( SPI_block_erase = 0x40000ca8 );
PROVIDE( SPI_chip_erase = 0x40000cb4 );
PROVIDE( SPI_init = 0x40000cc0 );
PROVIDE( SPI_page_program = 0x40000ccc );
PROVIDE( SPI_read_data = 0x40000cd8 );
PROVIDE( SPI_sector_erase = 0x40000ce4 );
PROVIDE( SelectSpiFunction = 0x40000cf0 );
PROVIDE( SetSpiDrvs = 0x40000cfc );
PROVIDE( Wait_SPI_Idle = 0x40000d08 );
PROVIDE( spi_dummy_len_fix = 0x40000d14 );
PROVIDE( Disable_QMode = 0x40000d20 );
PROVIDE( Enable_QMode = 0x40000d2c );


/* Functions */
ets_efuse_get_spiconfig = 0x40001f74;
ets_efuse_flash_octal_mode = 0x40002004;

/* Data (.data, .bss, .rodata) */
PROVIDE( rom_spiflash_legacy_funcs = 0x3fceffe8 );
PROVIDE( rom_spiflash_legacy_data = 0x3fceffe4 );
PROVIDE( g_flash_guard_ops = 0x3fceffec );
