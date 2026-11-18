#!/usr/bin/env -S cargo +nightly -Zscript
---
[package]
edition = "2024"
---

//! Updates `ROM_DATA_TABLES` in `src/chip/*.rs` from the ROM ELFs in an esp-rom-elfs checkout.
//!
//! Usage: `cargo +nightly -Zscript tools/rom-tables.rs <esp-rom-elfs checkout>`
//!
//! Revisions come from the checkout's release list, with the corrections in `FIXES`.
//! Entries in a chip file that no ELF covers are kept, and reported.

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

/// `(chip, revision, ELF)` entries that replace or extend the release config.
const FIXES: &[(&str, u32, &str)] = &[
    // The release list maps the ECO5 ROM to v1.2 and the ECO3 ROM to v1.4. esp-idf links the ECO3
    // ROM from v1.2.
    ("esp32c5", 102, "esp32c5eco3/eagle.hp.rom.out"),
    ("esp32c5", 104, "esp32c5eco5/eagle.hp.rom.out"),
    // Pre-v1.0 ROM, not in the release.
    ("esp32c61", 0, "esp32c61/eagle.hp.rom.out"),
];

#[derive(Clone, PartialEq)]
struct Table {
    data_start: u32,
    data_end: u32,
    bss_start: u32,
    bss_end: u32,
}

struct Entry {
    min_revision: u32,
    table: Table,
    comments: Vec<String>,
    source: Option<String>,
}

fn main() -> ExitCode {
    let Some(rom_elfs) = std::env::args().nth(1).map(PathBuf::from) else {
        eprintln!("usage: rom-tables <esp-rom-elfs checkout>");
        return ExitCode::FAILURE;
    };
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    let releases = match read_release_config(&rom_elfs) {
        Ok(releases) => releases,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let mut failed = false;
    for file in fs::read_dir(repo.join("src/chip")).unwrap() {
        let path = file.unwrap().path();
        let chip = path.file_stem().unwrap().to_str().unwrap().to_string();
        if let Err(e) = update_chip(repo, &rom_elfs, &chip, releases.get(&chip)) {
            eprintln!("{chip}: error: {e}");
            failed = true;
        }
    }

    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

/// Returns the ROM ELF of each chip revision, by chip.
fn read_release_config(rom_elfs: &Path) -> Result<BTreeMap<String, BTreeMap<u32, String>>, String> {
    let path = rom_elfs.join(".gitlab/ci/chips-to-release.yml");
    let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;

    let mut releases: BTreeMap<String, BTreeMap<u32, String>> = BTreeMap::new();
    let mut add = |chip: &str, rev: u32, elf: &str| {
        releases.entry(chip.to_string()).or_default().insert(rev, elf.to_string());
    };

    // Each matrix item starts with `CHIP_NAME` and has a `ROM_ELF`. `CHIP_REV` defaults to 0.
    let mut item: Option<(String, u32, Option<String>)> = None;
    let mut finish = |item: Option<(String, u32, Option<String>)>| -> Result<(), String> {
        if let Some((chip, rev, elf)) = item {
            let elf = elf.ok_or(format!("{chip} rev {rev} has no ROM_ELF"))?;
            add(&chip, rev, &elf);
        }
        Ok(())
    };
    for line in text.lines() {
        let line = line.trim().trim_start_matches("- ");
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        let value = value.trim().trim_start_matches("[\"").trim_end_matches("\"]");
        match key {
            "CHIP_NAME" => {
                finish(item.take())?;
                item = Some((value.to_string(), 0, None));
            }
            "CHIP_REV" => {
                let rev = value.parse().map_err(|_| format!("bad CHIP_REV {value}"))?;
                item.as_mut().ok_or("CHIP_REV before CHIP_NAME")?.1 = rev;
            }
            "ROM_ELF" => item.as_mut().ok_or("ROM_ELF before CHIP_NAME")?.2 = Some(value.to_string()),
            _ => {}
        }
    }
    finish(item.take())?;
    drop(finish);

    for (chip, rev, elf) in FIXES {
        add(chip, *rev, elf);
    }

    Ok(releases)
}

fn update_chip(
    repo: &Path,
    rom_elfs: &Path,
    chip: &str,
    releases: Option<&BTreeMap<u32, String>>,
) -> Result<(), String> {
    let path = repo.join(format!("src/chip/{chip}.rs"));
    let source = fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let (start, end) = find_tables(&source)?;
    let current = parse_entries(&source[start..end])?;
    let entry_size = parse_const(&source, "pub const ROM_TABLE_ENTRY_SIZE: u32 = ")?;
    let memory = LoaderMemory::read(repo, chip)?;

    let mut entries = BTreeMap::new();
    for entry in current.iter() {
        entries.insert(
            entry.min_revision,
            Entry {
                min_revision: entry.min_revision,
                table: entry.table.clone(),
                comments: entry.comments.clone(),
                source: None,
            },
        );
    }
    for (rev, elf) in releases.into_iter().flatten() {
        let rom = Rom::read(&rom_elfs.join(elf))?;
        let table = rom.table()?;
        rom.check(chip, elf, &table, entry_size, &memory)?;
        let comments = match eco_label(elf) {
            Some(label) => vec![label],
            None => entries.get(rev).map(|e: &Entry| e.comments.clone()).unwrap_or_default(),
        };
        entries.insert(
            *rev,
            Entry {
                min_revision: *rev,
                table,
                comments,
                source: Some(elf.clone()),
            },
        );
    }

    // A table that repeats the previous one changes nothing.
    let mut new: Vec<Entry> = Vec::new();
    for entry in entries.into_values() {
        if let Some(last) = new.last_mut().filter(|last| last.table == entry.table) {
            if last.source.is_none() && entry.source.is_some() {
                last.source = entry.source;
                if !entry.comments.is_empty() {
                    last.comments = entry.comments;
                }
            }
            continue;
        }
        new.push(entry);
    }

    for entry in new.iter() {
        let from = entry.source.as_deref().unwrap_or("kept, no ROM ELF");
        println!("{chip} rev {}: {from}", entry.min_revision);
    }

    let unchanged = new.len() == current.len()
        && new.iter().zip(current.iter()).all(|(new, current)| {
            new.min_revision == current.min_revision && new.table == current.table
        });
    if unchanged {
        return Ok(());
    }

    let updated = format!("{}{}{}", &source[..start], render(&new), &source[end..]);
    fs::write(&path, updated).map_err(|e| e.to_string())?;
    println!("{chip}: updated {}", path.display());
    Ok(())
}

/// Returns `ECO<n>` if the ELF's directory names an ECO, like `esp32c5eco3`.
fn eco_label(elf: &str) -> Option<String> {
    let dir = elf.split('/').next()?;
    let (_, eco) = dir.rsplit_once("eco")?;
    eco.parse::<u32>().ok().map(|eco| format!("ECO{eco}"))
}

const TABLES_START: &str = "pub const ROM_DATA_TABLES: RomDataTables = ";

/// Returns the byte range of the `ROM_DATA_TABLES` initializer, without the `;`.
fn find_tables(source: &str) -> Result<(usize, usize), String> {
    let start = source.find(TABLES_START).ok_or("no ROM_DATA_TABLES")? + TABLES_START.len();
    let len = source[start..].find("];").ok_or("ROM_DATA_TABLES does not end with `];`")?;
    Ok((start, start + len + 1))
}

fn parse_entries(text: &str) -> Result<Vec<Entry>, String> {
    let mut entries = Vec::new();
    let mut comments = Vec::new();
    let mut fields = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim().trim_start_matches("&[");
        if let Some(comment) = line.strip_prefix("//") {
            comments.push(comment.trim().to_string());
        } else if let Some((key, value)) = line.split_once(':') {
            fields.insert(key.trim().to_string(), parse_number(value.trim_end_matches(','))?);
        }
        if line.starts_with('}') {
            let field = |name: &str| fields.get(name).copied().ok_or(format!("entry has no {name}"));
            entries.push(Entry {
                min_revision: field("min_revision")?,
                table: Table {
                    data_start: field("data_start")?,
                    data_end: field("data_end")?,
                    bss_start: field("bss_start")?,
                    bss_end: field("bss_end")?,
                },
                comments: std::mem::take(&mut comments),
                source: None,
            });
            fields.clear();
        }
    }
    Ok(entries)
}

fn render(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return "&[]".to_string();
    }

    let single = entries.len() == 1;
    let indent = if single { "" } else { "    " };
    let mut out = String::from(if single { "&[" } else { "&[\n" });
    for entry in entries {
        if !single {
            out += indent;
        }
        out += "RomDataTable {\n";
        for comment in entry.comments.iter() {
            out += &format!("{indent}    // {comment}\n");
        }
        let Table { data_start, data_end, bss_start, bss_end } = &entry.table;
        out += &format!("{indent}    min_revision: {},\n", entry.min_revision);
        out += &format!("{indent}    data_start: {},\n", hex(*data_start));
        out += &format!("{indent}    data_end: {},\n", hex(*data_end));
        out += &format!("{indent}    bss_start: {},\n", hex(*bss_start));
        out += &format!("{indent}    bss_end: {},\n", hex(*bss_end));
        out += &format!("{indent}}}");
        if !single {
            out += ",\n";
        }
    }
    out += "]";
    out
}

fn hex(value: u32) -> String {
    if value == 0 { "0".to_string() } else { format!("{value:#X}").replace("0X", "0x") }
}

fn parse_number(text: &str) -> Result<u32, String> {
    let text = text.trim().replace('_', "");
    let parsed = match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16),
        None => text.parse(),
    };
    parsed.map_err(|_| format!("bad number `{text}`"))
}

fn parse_const(source: &str, prefix: &str) -> Result<u32, String> {
    let start = source.find(prefix).ok_or(format!("no `{prefix}`"))? + prefix.len();
    let len = source[start..].find(';').ok_or(format!("`{prefix}` has no `;`"))?;
    parse_number(&source[start..start + len])
}

/// Evaluates a linker script expression made of numbers, `+` and `-`.
fn parse_sum(text: &str) -> Result<u32, String> {
    let mut total = 0u32;
    let mut sign = 1i64;
    for token in text.replace('+', " + ").replace('-', " - ").split_whitespace() {
        match token {
            "+" => sign = 1,
            "-" => sign = -1,
            _ => total = (total as i64 + sign * parse_number(token)? as i64) as u32,
        }
    }
    Ok(total)
}

/// The data bus address ranges that the loader occupies.
struct LoaderMemory {
    regions: Vec<(&'static str, u32, u32)>,
}

impl LoaderMemory {
    fn read(repo: &Path, chip: &str) -> Result<Self, String> {
        let path = repo.join(format!("ld/{chip}.x"));
        let script = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let mut regions = Vec::new();
        for name in ["IRAM", "RWDATA"] {
            let line = script
                .lines()
                .find(|line| line.trim_start().starts_with(name) && line.contains("ORIGIN"))
                .ok_or(format!("{}: no {name} region", path.display()))?;
            let (_, rest) = line.split_once("ORIGIN").unwrap();
            let (origin, length) = rest.split_once("LENGTH").ok_or(format!("{name} has no LENGTH"))?;
            let origin = parse_sum(origin.trim_start_matches([' ', '=']).trim_end_matches([' ', ',']))?;
            let length = parse_sum(length.trim_start_matches([' ', '=']))?;
            let (start, end) = to_data_bus(chip, origin, origin + length);
            regions.push((name, start, end));
        }
        Ok(LoaderMemory { regions })
    }

    fn overlap(&self, start: u32, end: u32) -> Option<&'static str> {
        self.regions
            .iter()
            .find(|(_, region_start, region_end)| start < *region_end && *region_start < end)
            .map(|(name, _, _)| *name)
    }
}

/// Maps an address range in internal SRAM to the data bus, from `SOC_DIRAM_*` in esp-idf.
fn to_data_bus(chip: &str, start: u32, end: u32) -> (u32, u32) {
    let i_d_offset = match chip {
        "esp32s2" => 0x4002_0000 - 0x3FFB_0000,
        "esp32s3" => 0x4037_8000 - 0x3FC8_8000,
        "esp32c2" => 0x4038_0000 - 0x3FCA_0000,
        "esp32c3" => 0x4038_0000 - 0x3FC8_0000,
        _ => 0,
    };
    match chip {
        // SRAM1 is mapped to the instruction bus in reverse order.
        "esp32" if (0x400A_0000..0x400C_0000).contains(&start) => {
            (0x3FFE_0000 + (0x400C_0000 - end), 0x3FFE_0000 + (0x400C_0000 - start))
        }
        // The uncached alias of L2MEM.
        "esp32p4" if start >= 0x8000_0000 => (start - 0x4000_0000, end - 0x4000_0000),
        _ if start >= 0x4000_0000 && i_d_offset != 0 => (start - i_d_offset, end - i_d_offset),
        _ => (start, end),
    }
}

struct Section {
    addr: u32,
    data: Vec<u8>,
}

struct Rom {
    sections: Vec<Section>,
    symbols: BTreeMap<String, u32>,
}

impl Rom {
    fn read(path: &Path) -> Result<Self, String> {
        let elf = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if elf.get(..6) != Some(&[0x7F, b'E', b'L', b'F', 1, 1]) {
            return Err(format!("{}: not a 32-bit little endian ELF", path.display()));
        }
        let u16_at = |at: usize| u16::from_le_bytes(elf[at..at + 2].try_into().unwrap()) as usize;
        let u32_at = |at: usize| u32::from_le_bytes(elf[at..at + 4].try_into().unwrap());

        const PT_LOAD: u32 = 1;
        const SHT_SYMTAB: u32 = 2;

        // Segments are indexed by load address: initialized data is loaded from ROM.
        let phoff = u32_at(0x1C) as usize;
        let phentsize = u16_at(0x2A);
        let phnum = u16_at(0x2C);
        let sections = (0..phnum)
            .map(|i| -> [u32; 8] {
                std::array::from_fn(|word| u32_at(phoff + i * phentsize + word * 4))
            })
            .filter(|p| p[0] == PT_LOAD && p[4] != 0)
            .map(|p| Section {
                addr: p[3],
                data: elf[p[1] as usize..(p[1] + p[4]) as usize].to_vec(),
            })
            .collect();

        let shoff = u32_at(0x20) as usize;
        let shentsize = u16_at(0x2E);
        let shnum = u16_at(0x30);
        let headers: Vec<[u32; 10]> = (0..shnum)
            .map(|i| std::array::from_fn(|word| u32_at(shoff + i * shentsize + word * 4)))
            .collect();
        let bytes = |header: &[u32; 10]| {
            let offset = header[4] as usize;
            &elf[offset..offset + header[5] as usize]
        };

        let mut symbols = BTreeMap::new();
        for symtab in headers.iter().filter(|h| h[1] == SHT_SYMTAB) {
            let strtab = bytes(&headers[symtab[6] as usize]);
            for symbol in bytes(symtab).chunks_exact(16) {
                let name = u32::from_le_bytes(symbol[0..4].try_into().unwrap()) as usize;
                let value = u32::from_le_bytes(symbol[4..8].try_into().unwrap());
                let name = &strtab[name..];
                let name = &name[..name.iter().position(|&b| b == 0).unwrap()];
                symbols.insert(String::from_utf8_lossy(name).into_owned(), value);
            }
        }

        Ok(Rom { sections, symbols })
    }

    fn symbol(&self, names: &[&str]) -> Result<u32, String> {
        names
            .iter()
            .find_map(|name| self.symbols.get(*name).copied())
            .ok_or(format!("no symbol {}", names.join(" or ")))
    }

    fn bytes(&self, addr: u32, len: u32) -> Option<&[u8]> {
        self.sections.iter().find_map(|section| {
            let offset = addr.checked_sub(section.addr)? as usize;
            section.data.get(offset..offset + len as usize)
        })
    }

    fn words(&self, start: u32, end: u32) -> Result<Vec<u32>, String> {
        let bytes = self
            .bytes(start, end - start)
            .ok_or(format!("{start:#X}..{end:#X} is not in the ELF"))?;
        Ok(bytes.chunks_exact(4).map(|w| u32::from_le_bytes(w.try_into().unwrap())).collect())
    }

    fn table(&self) -> Result<Table, String> {
        Ok(Table {
            data_start: self.symbol(&["_data_start", "_data_table_start"])?,
            data_end: self.symbol(&["_data_end", "_data_table_end"])?,
            bss_start: self.symbol(&["_bss_table_start"])?,
            bss_end: self.symbol(&["_bss_table_end"])?,
        })
    }

    /// Checks that the tables have the chip's entry size, and that the RAM they initialize does
    /// not overlap the loader.
    fn check(
        &self,
        chip: &str,
        elf: &str,
        table: &Table,
        entry_size: u32,
        memory: &LoaderMemory,
    ) -> Result<(), String> {
        let data = self.words(table.data_start, table.data_end)?;
        let bss = self.words(table.bss_start, table.bss_end)?;
        let data_words = entry_size as usize / 4;
        let bss_words = data_words - 1;
        if data.len() % data_words != 0 || bss.len() % bss_words != 0 {
            return Err(format!("{elf}: tables are not made of {entry_size} byte entries"));
        }
        // In 16 byte entries, the last word selects the cores and is 0 or 1. In 12 byte entries
        // the same position holds an address.
        let has_core_flag = |words: &[u32], size: usize| {
            !words.is_empty() && words.chunks_exact(size).all(|e| e[size - 1] <= 1)
        };
        if (entry_size == 16) != has_core_flag(&data, 4) && !data.is_empty() {
            return Err(format!("{elf}: data table entries are not {entry_size} bytes"));
        }

        // `rom::init_rom_data` copies and zeroes whole words, up to the next word boundary.
        let mut ranges: Vec<(u32, u32)> = data
            .chunks_exact(data_words)
            .map(|e| (e[0], e[1]))
            .chain(bss.chunks_exact(bss_words).map(|e| (e[0], e[1])))
            .filter(|(start, end)| start != end)
            .collect();
        for (start, end) in ranges.iter_mut() {
            if *start > *end || *start % 4 != 0 {
                return Err(format!("{elf}: bad RAM range {start:#X}..{end:#X}"));
            }
            *end = end.next_multiple_of(4);
        }
        ranges.sort();
        for pair in ranges.windows(2) {
            if pair[0].1 > pair[1].0 {
                return Err(format!(
                    "{elf}: word-rounded RAM ranges {:#X}..{:#X} and {:#X}..{:#X} overlap",
                    pair[0].0, pair[0].1, pair[1].0, pair[1].1
                ));
            }
        }

        let (mut low, mut high) = (u32::MAX, 0);
        let mut overlaps = Vec::new();
        for (start, end) in ranges {
            let (start, end) = to_data_bus(chip, start, end);
            if let Some(region) = memory.overlap(start, end) {
                overlaps.push(format!("{start:#X}..{end:#X} overlaps the loader's {region}"));
            }
            low = low.min(start);
            high = high.max(end);
        }
        if !overlaps.is_empty() {
            return Err(format!("{elf}: ROM RAM {}", overlaps.join(", ")));
        }
        for (start, end) in data.chunks_exact(data_words).map(|e| (e[2], e[1] - e[0])) {
            if self.bytes(start, end).is_none() {
                return Err(format!("{elf}: initial data at {start:#X} is not in the ELF"));
            }
        }
        println!("{chip}: {elf} initializes {low:#X}..{high:#X}");
        Ok(())
    }
}
