MEMORY {
    /* SRAM1 + 0x4000 cache + 0x400 vectors */
    IRAM : ORIGIN = 0x4002C400, LENGTH = 0x10000
    RWDATA : ORIGIN = 0x3FFBE000, LENGTH = 0x21000
}

INCLUDE "loader.x"

/**
 * ESP32-S2 ROM address table (except symbols from libgcc and libc)
 * Generated for ROM with MD5sum: 0a2c7ec5109c17884606d23b47045796
 *
 * These are all weak symbols that could be overwritten in ESP-IDF.
 */

PROVIDE ( ets_delay_us = 0x4000d888 );
PROVIDE ( ets_efuse_get_spiconfig = 0x4000e4a0 );
PROVIDE ( s_cdcacm_old_rts = 0x3ffffd34 );
PROVIDE ( SelectSpiFunction = 0x40015d08 );
PROVIDE ( SelectSpiQIO = 0x40015b88 );
PROVIDE ( SendMsg = 0x40012d0c );
PROVIDE ( send_packet = 0x40012cc8 );
PROVIDE ( set_rtc_memory_crc = 0x40010010 );
PROVIDE ( SetSpiDrvs = 0x40015c18 );
PROVIDE ( sig_matrix = 0x3ffffd57 );
PROVIDE ( software_reset = 0x40010068 );
PROVIDE ( software_reset_cpu = 0x40010080 );
PROVIDE ( SPI_block_erase = 0x4001623c );
PROVIDE ( spi_cache_mode_switch = 0x40016a00 );
PROVIDE ( SPI_chip_erase = 0x400161b8 );
PROVIDE ( SPIClkConfig = 0x400170a0 );
PROVIDE ( SPI_Common_Command = 0x400162e8 );
PROVIDE ( spi_common_set_flash_cs_timing = 0x40016c0c );
PROVIDE ( spi_dummy_len_fix = 0x40015b50 );
PROVIDE ( SPI_Encrypt_Write = 0x400177e0 );
PROVIDE ( SPI_Encrypt_Write_Dest = 0x400176cc );
PROVIDE ( SPIEraseArea = 0x40017470 );
PROVIDE ( SPIEraseBlock = 0x4001710c );
PROVIDE ( SPIEraseChip = 0x400170ec );
PROVIDE ( SPIEraseSector = 0x4001716c );
PROVIDE ( esp_rom_spiflash_attach = 0x40017004 );
PROVIDE ( spi_flash_boot_attach = 0x40016fc0 );
PROVIDE ( spi_flash_check_suspend_cb = 0x3ffffd58 );
PROVIDE ( SPI_flashchip_data = 0x3ffffd3c );
PROVIDE ( spi_flash_set_check_suspend_cb = 0x40015b3c );
PROVIDE ( SPI_init = 0x40016ce8 );
PROVIDE ( SPILock = 0x40016ed4 );
PROVIDE ( SPIMasterReadModeCnfig = 0x40017014 );
PROVIDE ( SPI_page_program = 0x400165a8 );
PROVIDE ( SPIParamCfg = 0x40017500 );
PROVIDE ( SPIRead = 0x4001728c );
PROVIDE ( SPI_read_data = 0x40015ed8 );
PROVIDE ( SPIReadModeCnfig = 0x40016f1c );
PROVIDE ( SPI_read_status = 0x40016084 );
PROVIDE ( SPI_read_status_high = 0x40016284 );
PROVIDE ( SPI_sector_erase = 0x400161ec );
PROVIDE ( spi_slave_download = 0x4001998c );
PROVIDE ( spi_slave_rom_check_conn = 0x40019724 );
PROVIDE ( spi_slave_rom_init = 0x40019774 );
PROVIDE ( spi_slave_rom_init_hw = 0x40019b5c );
PROVIDE ( spi_slave_rom_intr_enable = 0x40019b3c );
PROVIDE ( spi_slave_rom_rxdma_load = 0x40019da8 );
PROVIDE ( spi_slave_rom_txdma_load = 0x40019e3c );
PROVIDE ( SPIUnlock = 0x40016e88 );
PROVIDE ( SPI_user_command_read = 0x40015fc8 );
PROVIDE ( SPI_Wait_Idle = 0x40016680 );
PROVIDE ( SPI_WakeUp = 0x400160f4 );
PROVIDE ( SPIWrite = 0x400171cc );
PROVIDE ( SPI_write_enable = 0x4001655c );
PROVIDE ( SPI_Write_Encrypt_Disable = 0x40017694 );
PROVIDE ( SPI_Write_Encrypt_Enable = 0x40017678 );
PROVIDE ( SPI_write_status = 0x400162a4 );
PROVIDE ( tdefl_compress = 0x400041dc );
PROVIDE ( tdefl_compress_buffer = 0x40004938 );
PROVIDE ( tdefl_compress_mem_to_mem = 0x40004a50 );
PROVIDE ( tdefl_compress_mem_to_output = 0x40004a30 );
PROVIDE ( tdefl_get_adler32 = 0x40004a28 );
PROVIDE ( tdefl_get_prev_return_status = 0x40004a20 );
PROVIDE ( tdefl_init = 0x40004954 );
PROVIDE ( tdefl_write_image_to_png_file_in_memory = 0x40004a64 );
PROVIDE ( tdefl_write_image_to_png_file_in_memory_ex = 0x40004a58 );
PROVIDE ( tinfl_decompress = 0x40003000 );
PROVIDE ( tinfl_decompress_mem_to_callback = 0x400041a8 );
PROVIDE ( tinfl_decompress_mem_to_mem = 0x40004168 );

PROVIDE ( uart_tx_one_char = 0x40012b10 );

/**
 * SPI flash driver function, compatibility names.
 */

PROVIDE ( g_rom_spiflash_dummy_len_plus = dummy_len_plus);
PROVIDE ( g_ticks_per_us_pro = g_ticks_per_us );
PROVIDE ( g_rom_flashchip = SPI_flashchip_data );
PROVIDE ( g_rom_spiflash_chip = SPI_flashchip_data );
PROVIDE ( esp_rom_spiflash_config_param = SPIParamCfg );
PROVIDE ( esp_rom_spiflash_read = SPIRead );
PROVIDE ( esp_rom_spiflash_read_status = SPI_read_status );
PROVIDE ( esp_rom_spiflash_read_statushigh = SPI_read_status_high );
PROVIDE ( esp_rom_spiflash_read_user_cmd = SPI_user_command_read );
PROVIDE ( esp_rom_spiflash_write = SPIWrite );
PROVIDE ( esp_rom_spiflash_write_encrypted_disable = SPI_Write_Encrypt_Disable );
PROVIDE ( esp_rom_spiflash_write_encrypted_enable = SPI_Write_Encrypt_Enable );
PROVIDE ( esp_rom_spiflash_config_clk = SPIClkConfig );
PROVIDE ( esp_rom_spiflash_select_qio_pins = SelectSpiQIO );
PROVIDE ( esp_rom_spiflash_unlock = SPIUnlock );
PROVIDE ( esp_rom_spiflash_erase_chip = SPIEraseChip );
PROVIDE ( esp_rom_spiflash_erase_sector = SPIEraseSector );
PROVIDE ( esp_rom_spiflash_erase_block = SPIEraseBlock );
PROVIDE ( esp_rom_spiflash_wait_idle = SPI_Wait_Idle );
PROVIDE ( esp_rom_spiflash_config_readmode = SPIReadModeCnfig );
PROVIDE ( esp_rom_spiflash_erase_block = SPIEraseBlock );
PROVIDE ( esp_rom_spiflash_write_encrypted = SPI_Encrypt_Write );
PROVIDE ( esp_rom_spiflash_erase_area = SPIEraseArea );
