// SPDX-License-Identifier: GPL-3.0-only
//
// Sequencer project storage in the driver's user storage area.
//
// Version 2 (current): 8 slots of 16 KB at V2_BASE, meta record after them.
//   0    "LPS2", u8 version 2, u16 bpm, u8 swing (50-75 %), 8 reserved      = 16 bytes
//   16   8 tracks x 1048 bytes:
//          16 bytes settings: channel, output, muted, note, base, pattern, chain start, chain end, transpose (i8),
//                             7 reserved
//          8 patterns x 129 bytes: length, then 32 steps x 4 bytes
//                             (on << 7 | note, velocity, gate 1-8, probability 1-100 %)
//   8400 u32 CRC-32 (IEEE) of bytes 0..8400
// Version 1 (read-only import): 8 slots of 4 KB at offset 0, "LPS1", one pattern per track, 2 bytes per step.

use core::cell::UnsafeCell;

use crate::driver;

pub const SLOTS: usize = 8;
pub const TRACKS: usize = 8;
pub const PATTERNS: usize = 8;
pub const STEPS: usize = 32;

const V2_BASE: u32 = 0x0004_0000;
const SLOT_SIZE: u32 = 0x4000;
const META_OFFSET: u32 = V2_BASE + SLOTS as u32 * SLOT_SIZE;
const MAGIC: [u8; 4] = *b"LPS2";
const META_MAGIC: [u8; 4] = *b"LPSN";

pub const HEADER: usize = 16;
pub const TRACK_SETTINGS: usize = 16;
pub const PATTERN_BYTES: usize = 1 + STEPS * 4;
pub const TRACK_BYTES: usize = TRACK_SETTINGS + PATTERNS * PATTERN_BYTES;
pub const BODY: usize = HEADER + TRACKS * TRACK_BYTES;
pub const RECORD: usize = BODY + 4;

// ---- v1 (import only) ----
const V1_SLOT_SIZE: u32 = 4096;
const V1_MAGIC: [u8; 4] = *b"LPS1";
const V1_META_MAGIC: [u8; 4] = *b"LPSM";
pub const V1_TRACK_BYTES: usize = 70;
const V1_BODY: usize = 8 + 8 * V1_TRACK_BYTES;
pub const V1_RECORD: usize = V1_BODY + 4;

const CRC_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut c = i as u32;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 { (c >> 1) ^ 0xedb8_8320 } else { c >> 1 };
            k += 1;
        }
        table[i] = c;
        i += 1;
    }
    table
};

/// Incremental CRC-32 (IEEE).
#[derive(Copy, Clone)]
pub struct Crc(u32);

impl Crc {
    pub const fn new() -> Self {
        Crc(0xffff_ffff)
    }
    pub fn update(&mut self, data: &[u8]) {
        let mut c = self.0;
        for &b in data {
            c = CRC_TABLE[((c ^ b as u32) & 0xff) as usize] ^ (c >> 8);
        }
        self.0 = c;
    }
    pub fn value(self) -> u32 {
        !self.0
    }
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut c = Crc::new();
    c.update(data);
    c.value()
}

/// One shared record buffer (8 KB) for save/load; the sequencer only runs on the main loop.
struct Buffer(UnsafeCell<[u8; RECORD]>);
unsafe impl Sync for Buffer {}
static BUFFER: Buffer = Buffer(UnsafeCell::new([0; RECORD]));

pub fn with_buffer<R>(f: impl FnOnce(&mut [u8; RECORD]) -> R) -> R {
    f(unsafe { &mut *BUFFER.0.get() })
}

pub fn available() -> bool {
    driver::user_storage_size() >= META_OFFSET + 16
}

pub fn header(bpm: u16, swing: u8) -> [u8; HEADER] {
    let mut h = [0u8; HEADER];
    h[0..4].copy_from_slice(&MAGIC);
    h[4] = 2;
    h[5..7].copy_from_slice(&bpm.to_le_bytes());
    h[7] = swing;
    h
}

pub fn seal(rec: &mut [u8; RECORD]) {
    let crc = crc32(&rec[..BODY]);
    rec[BODY..RECORD].copy_from_slice(&crc.to_le_bytes());
}

fn valid(rec: &[u8; RECORD]) -> bool {
    rec[0..4] == MAGIC
        && rec[4] == 2
        && u32::from_le_bytes([rec[BODY], rec[BODY + 1], rec[BODY + 2], rec[BODY + 3]]) == crc32(&rec[..BODY])
}

pub fn read_slot(slot: usize, rec: &mut [u8; RECORD]) -> bool {
    slot < SLOTS && driver::read_user_storage(V2_BASE + slot as u32 * SLOT_SIZE, rec) && valid(rec)
}

pub fn write_slot(slot: usize, rec: &[u8; RECORD]) -> bool {
    slot < SLOTS && driver::write_user_storage(V2_BASE + slot as u32 * SLOT_SIZE, rec)
}

/// Version-1 record from the earlier single-pattern format, if one exists in this slot.
pub fn read_v1_slot(slot: usize, rec: &mut [u8; V1_RECORD]) -> bool {
    slot < SLOTS
        && driver::read_user_storage(slot as u32 * V1_SLOT_SIZE, rec)
        && rec[0..4] == V1_MAGIC
        && rec[4] == 1
        && u32::from_le_bytes([rec[V1_BODY], rec[V1_BODY + 1], rec[V1_BODY + 2], rec[V1_BODY + 3]])
            == crc32(&rec[..V1_BODY])
}

pub fn last_slot() -> usize {
    let mut meta = [0u8; 8];
    if driver::read_user_storage(META_OFFSET, &mut meta) && meta[0..4] == META_MAGIC && (meta[4] as usize) < SLOTS {
        return meta[4] as usize;
    }
    if driver::read_user_storage(SLOTS as u32 * V1_SLOT_SIZE, &mut meta) && meta[0..4] == V1_META_MAGIC
        && (meta[4] as usize) < SLOTS
    {
        return meta[4] as usize;
    }
    0
}

pub fn set_last_slot(slot: usize) {
    let mut meta = [0u8; 8];
    meta[0..4].copy_from_slice(&META_MAGIC);
    meta[4] = slot as u8;
    let _ = driver::write_user_storage(META_OFFSET, &meta);
}
