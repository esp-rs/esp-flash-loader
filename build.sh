#!/bin/bash

set -euo pipefail

# macOS base64 doesn't take -w argument and defaults to a single line.
if [[ $(uname) = "Darwin" ]]; then
    BASE64_FLAGS=""
else
    BASE64_FLAGS="-w0"
fi

BASE_ADDR=0x40390000

cargo build --release
ELF=target/riscv32imc-unknown-none-elf/release/esp-flashloader

rust-objdump --disassemble $ELF > target/disassembly.s
rust-objdump -x $ELF > target/dump.txt
rust-nm $ELF -n > target/nm.txt

function bin {
    rust-objcopy $ELF -O binary - | base64 $BASE64_FLAGS
}

function sym {
    echo $(((0x$(rust-nm $ELF | grep -w $1 | cut -d ' ' -f 1) + 1)-$BASE_ADDR))
}

function data_offset {
    rust-objcopy $ELF -O binary - | wc -c
}

cat <<EOF
    instructions: $(bin)
    pc_init: $(sym Init)
    pc_uninit: $(sym UnInit)
    pc_program_page: $(sym ProgramPage)
    pc_erase_sector: $(sym EraseSector)
    pc_erase_all: $(sym EraseChip)
    data_section_offset: $(data_offset)
EOF