// SPDX-License-Identifier: GPL-3.0-only
//
// Sequencer project storage in the driver's user storage area.
//
// Version 6 (current): 32 projects of up to 30 KB at offset 0 (project n at n * SLOT_SIZE), current-project record
// at META6_OFFSET. 16 song-wide patterns (every track is on the same pattern), and only the steps that are in use
// are stored, so a project normally takes a few KB:
//   0    "LPS6", u8 version 6, u16 bpm, u8 swing, u8 flags, u32 record length (incl. CRC), 3 reserved = 16 bytes
//   16   8 tracks x 16 bytes of settings (as below; pattern / chain bytes now count 0-15)
//   144  for each track, for each of its 16 patterns: u8 length, u8 stored-step count n, then n x (u8 step
//        index, 11 step bytes as below). A step is stored when it is on or differs from an empty step.
//   then  "PR" + 8 tracks x 16 patterns x u8 step rate in half pulses per step (added later: older firmware stops
//        reading before it; projects without it give every pattern the track's rate from its settings byte 10)
//   end  u32 CRC-32 (IEEE) of everything before it
// The older versions below hold 8 projects of 8 per-track patterns; they are converted once (see migration in mod.rs).
// Version 5: 64 steps per pattern. 8 slots of 48 KB in the two gaps the older formats leave free: slots 0-3
// at V5_LOW (0x10000-0x40000), slots 4-7 at V5_HIGH (0xC0000-0xF0000). Same layout as version 4 below except
// 64 steps per pattern (8 patterns x 705 bytes, 8 tracks x 5656 bytes, CRC at 45264).
// Version 4 (read): 32 steps per pattern, 8 slots of 32 KB at V4_BASE; the last-used-slot record stays at META_OFFSET.
//   0    "LPS2", u8 version 4, u16 bpm, u8 swing (50-75 %), u8 flags (bit 1 = follow
//        external clock), 7 reserved                                       = 16 bytes
//   16   8 tracks x 2840 bytes:
//          16 bytes settings: channel, output, muted, note, base, pattern, chain start, chain end, transpose (i8),
//                             0 (was a test-build rate), step rate in half clock pulses per step
//                             (0 in older projects = 1/16), solo, 4 reserved
//          8 patterns x 353 bytes: length, then 32 steps x 11 bytes
//                             (on << 7 | note 1, notes 2-8 (0xFF = none), velocity, gate in 8ths of a step (1-255), probability 1-100 %)
//   22736 u32 CRC-32 (IEEE) of bytes 0..22736
// Versions 3 and 2 (read): 8 slots of 16 KB at V2_BASE, same layout with 4 notes (7-byte steps) or 1 note
//   (4-byte steps: on << 7 | note, velocity, gate, prob). A slot with a valid v4 record wins over its old copy.
// Version 1 (read-only import): 8 slots of 4 KB at offset 0, "LPS1", one pattern per track, 2 bytes per step.

use core::cell::UnsafeCell;

use crate::driver;

/// Projects (version 6).
pub const SLOTS: usize = 32;
pub const TRACKS: usize = 8;
/// Song-wide patterns per project (version 6).
pub const PATTERNS: usize = 16;
pub const STEPS: usize = 64;
/// Projects and per-track patterns in the older formats (versions 1-5).
pub const OLD_SLOTS: usize = 8;
pub const OLD_PATTERNS: usize = 8;

/// Version 6 slots: 32 x 30 KB from offset 0, then the current-project record.
pub const SLOT_SIZE6: u32 = 0x7800;
const META6_OFFSET: u32 = SLOTS as u32 * SLOT_SIZE6;
const MAGIC6: [u8; 4] = *b"LPS6";
const META6_MAGIC: [u8; 4] = *b"LPSC";
pub const VERSION6: u8 = 6;
const _: () = assert!(META6_OFFSET + 0x1000 <= 0x10_0000);

const V2_BASE: u32 = 0x0004_0000;
const V2_SLOT_SIZE: u32 = 0x4000;
const META_OFFSET: u32 = V2_BASE + OLD_SLOTS as u32 * V2_SLOT_SIZE;
const V4_BASE: u32 = 0x0008_0000;
const V4_SLOT_SIZE: u32 = 0x8000;
const V5_LOW: u32 = 0x0001_0000;
const V5_HIGH: u32 = 0x000C_0000;
const SLOT_SIZE: u32 = 0xC000;
const MAGIC: [u8; 4] = *b"LPS2";
const META_MAGIC: [u8; 4] = *b"LPSN";

pub const MAX_NOTES: usize = 8;
pub const NO_NOTE: u8 = 0xFF;

pub const VERSION: u8 = 5;
pub const HEADER: usize = 16;
pub const TRACK_SETTINGS: usize = 16;
pub const STEP_BYTES: usize = MAX_NOTES + 3;
pub const PATTERN_BYTES: usize = 1 + STEPS * STEP_BYTES;
pub const TRACK_BYTES: usize = TRACK_SETTINGS + OLD_PATTERNS * PATTERN_BYTES;
pub const BODY: usize = HEADER + TRACKS * TRACK_BYTES;
pub const RECORD: usize = BODY + 4;
const _: () = assert!(RECORD as u32 <= SLOT_SIZE);
// The low slots end where the v2/v3 area starts; the high slots start after the v4 area and stay inside 1 MB.
const _: () = assert!(V5_LOW + 4 * SLOT_SIZE <= V2_BASE);
const _: () = assert!(V4_BASE + OLD_SLOTS as u32 * V4_SLOT_SIZE <= V5_HIGH);
const _: () = assert!(V5_HIGH + 4 * SLOT_SIZE <= 0x10_0000);

fn slot_addr(slot: usize) -> u32 {
    if slot < 4 { V5_LOW + slot as u32 * SLOT_SIZE } else { V5_HIGH + (slot as u32 - 4) * SLOT_SIZE }
}

/// Layout of a stored record by version: (step bytes, pattern bytes, track bytes, notes per step, steps per pattern).
pub const fn layout(version: u8) -> (usize, usize, usize, usize, usize) {
    let notes = match version {
        2 => 1,
        3 => 4,
        _ => MAX_NOTES,
    };
    let steps = if version >= 5 { STEPS } else { 32 };
    let step = if notes == 1 { 4 } else { notes + 3 };
    let pattern = 1 + steps * step;
    let track = TRACK_SETTINGS + OLD_PATTERNS * pattern;
    (step, pattern, track, notes, steps)
}

const fn body(version: u8) -> usize {
    HEADER + TRACKS * layout(version).2
}

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

/// Bytes in the shared buffer: an old version-5 record (45 KB), which is more than a version-6 slot.
pub const BUF: usize = if RECORD > SLOT_SIZE6 as usize { RECORD } else { SLOT_SIZE6 as usize };

/// One shared record buffer (45 KB) for save/load; the sequencer only runs on the main loop.
struct Buffer(UnsafeCell<[u8; BUF]>);
unsafe impl Sync for Buffer {}
static BUFFER: Buffer = Buffer(UnsafeCell::new([0; BUF]));

pub fn with_buffer<R>(f: impl FnOnce(&mut [u8; BUF]) -> R) -> R {
    f(unsafe { &mut *BUFFER.0.get() })
}

/// The old-format view of the buffer.
pub fn old_record(buf: &mut [u8; BUF]) -> &mut [u8; RECORD] {
    (&mut buf[..RECORD]).try_into().unwrap()
}

pub fn available() -> bool {
    driver::user_storage_size() >= 0x10_0000
}

/// Header byte 8, bit 1: follow an external MIDI clock (Shift + Play). Clear = use the internal clock, the default; older
/// projects never set it, so they load on the internal clock. (Bit 0, "ignore external clock" in earlier builds, is
/// no longer used.)
pub const FLAG_FOLLOW_CLOCK: u8 = 0x02;
/// Marks the per-pattern step-rate block after the patterns (version 6 records from v0.8 on).
pub const RATES_MAGIC: [u8; 2] = *b"PR";

fn valid(rec: &[u8; RECORD]) -> bool {
    let body = match rec[4] {
        2 | 3 | 4 | VERSION => body(rec[4]),
        _ => return false,
    };
    rec[0..4] == MAGIC
        && u32::from_le_bytes([rec[body], rec[body + 1], rec[body + 2], rec[body + 3]]) == crc32(&rec[..body])
}

// ---------------- version 6 ----------------

pub fn header6(bpm: u16, swing: u8, flags: u8) -> [u8; HEADER] {
    let mut h = [0u8; HEADER];
    h[0..4].copy_from_slice(&MAGIC6);
    h[4] = VERSION6;
    h[5..7].copy_from_slice(&bpm.to_le_bytes());
    h[7] = swing;
    h[8] = flags;
    h
}

/// Finishes a version-6 record of `body` bytes in `buf`: stores the length and appends the CRC. Returns the record
/// length, or None if it does not fit a slot.
pub fn seal6(buf: &mut [u8; BUF], body: usize) -> Option<usize> {
    let len = body + 4;
    if len > SLOT_SIZE6 as usize {
        return None;
    }
    buf[9..13].copy_from_slice(&(len as u32).to_le_bytes());
    let crc = crc32(&buf[..body]);
    buf[body..len].copy_from_slice(&crc.to_le_bytes());
    Some(len)
}

/// Header check only (for the project grid).
pub fn slot_used6(slot: usize) -> bool {
    let mut h = [0u8; 8];
    slot < SLOTS && driver::read_user_storage(slot as u32 * SLOT_SIZE6, &mut h) && h[0..4] == MAGIC6 && h[4] == VERSION6
}

/// Reads a version-6 project; returns its length (CRC included) if it is there and intact.
pub fn read_slot6(slot: usize, buf: &mut [u8; BUF]) -> Option<usize> {
    if slot >= SLOTS || !driver::read_user_storage(slot as u32 * SLOT_SIZE6, &mut buf[..HEADER]) {
        return None;
    }
    if buf[0..4] != MAGIC6 || buf[4] != VERSION6 {
        return None;
    }
    let len = u32::from_le_bytes([buf[9], buf[10], buf[11], buf[12]]) as usize;
    if !(HEADER + 4..=SLOT_SIZE6 as usize).contains(&len) || !driver::read_user_storage(slot as u32 * SLOT_SIZE6, &mut buf[..len]) {
        return None;
    }
    let body = len - 4;
    (u32::from_le_bytes([buf[body], buf[body + 1], buf[body + 2], buf[body + 3]]) == crc32(&buf[..body])).then_some(len)
}

pub fn write_slot6(slot: usize, rec: &[u8]) -> bool {
    slot < SLOTS && rec.len() <= SLOT_SIZE6 as usize && driver::write_user_storage(slot as u32 * SLOT_SIZE6, rec)
}

/// The current project (version 6), or None before the old projects have been converted.
pub fn current6() -> Option<usize> {
    let mut meta = [0u8; 8];
    (driver::read_user_storage(META6_OFFSET, &mut meta) && meta[0..4] == META6_MAGIC).then(|| meta[4] as usize % SLOTS)
}

pub fn set_current6(slot: usize) {
    let mut meta = [0u8; 8];
    meta[0..4].copy_from_slice(&META6_MAGIC);
    meta[4] = slot as u8;
    let _ = driver::write_user_storage(META6_OFFSET, &meta);
}

/// Where old project `slot` lives (start, length), in the version `read_slot` / `read_v1_slot` would use, if any.
pub fn old_source(slot: usize) -> Option<(u32, u32)> {
    let mut h = [0u8; 8];
    let has = |addr: u32, h: &mut [u8; 8], ok: &dyn Fn(&[u8; 8]) -> bool| driver::read_user_storage(addr, h) && ok(h);
    if slot >= OLD_SLOTS {
        return None;
    }
    if has(slot_addr(slot), &mut h, &|h| h[0..4] == MAGIC && h[4] == VERSION) {
        return Some((slot_addr(slot), RECORD as u32));
    }
    if has(V4_BASE + slot as u32 * V4_SLOT_SIZE, &mut h, &|h| h[0..4] == MAGIC && h[4] == 4) {
        return Some((V4_BASE + slot as u32 * V4_SLOT_SIZE, body(4) as u32 + 4));
    }
    if has(V2_BASE + slot as u32 * V2_SLOT_SIZE, &mut h, &|h| h[0..4] == MAGIC && matches!(h[4], 2 | 3)) {
        return Some((V2_BASE + slot as u32 * V2_SLOT_SIZE, V2_SLOT_SIZE));
    }
    if has(slot as u32 * V1_SLOT_SIZE, &mut h, &|h| h[0..4] == V1_MAGIC && h[4] == 1) {
        return Some((slot as u32 * V1_SLOT_SIZE, V1_RECORD as u32));
    }
    None
}

// ---------------- versions 1-5 (read, for the one-time conversion) ----------------

/// Reads a slot: the current-format record if there is one, else an older v4, v3 or v2 record from its old area.
pub fn read_slot(slot: usize, rec: &mut [u8; RECORD]) -> bool {
    if slot >= OLD_SLOTS {
        return false;
    }
    if driver::read_user_storage(slot_addr(slot), rec) && rec[4] == VERSION && valid(rec) {
        return true;
    }
    let v4 = &mut rec[..body(4) + 4];
    if driver::read_user_storage(V4_BASE + slot as u32 * V4_SLOT_SIZE, v4) && rec[4] == 4 && valid(rec) {
        return true;
    }
    let old = &mut rec[..V2_SLOT_SIZE as usize];
    driver::read_user_storage(V2_BASE + slot as u32 * V2_SLOT_SIZE, old) && matches!(rec[4], 2 | 3) && valid(rec)
}

/// Version-1 record from the earlier single-pattern format, if one exists in this slot.
pub fn read_v1_slot(slot: usize, rec: &mut [u8; V1_RECORD]) -> bool {
    slot < OLD_SLOTS
        && driver::read_user_storage(slot as u32 * V1_SLOT_SIZE, rec)
        && rec[0..4] == V1_MAGIC
        && rec[4] == 1
        && u32::from_le_bytes([rec[V1_BODY], rec[V1_BODY + 1], rec[V1_BODY + 2], rec[V1_BODY + 3]])
            == crc32(&rec[..V1_BODY])
}

/// The last used project in the old formats.
pub fn last_slot() -> usize {
    let mut meta = [0u8; 8];
    if driver::read_user_storage(META_OFFSET, &mut meta) && meta[0..4] == META_MAGIC && (meta[4] as usize) < OLD_SLOTS {
        return meta[4] as usize;
    }
    if driver::read_user_storage(OLD_SLOTS as u32 * V1_SLOT_SIZE, &mut meta) && meta[0..4] == V1_META_MAGIC
        && (meta[4] as usize) < OLD_SLOTS
    {
        return meta[4] as usize;
    }
    0
}
