// SPDX-License-Identifier: GPL-3.0-only
//
// Arpeggiator for the notes played live on the selected track (note pads or a keyboard on the USB MIDI port / DIN
// In). This file only keeps the notes and works out which one comes next; the sequencer times and sends them.

/// Play orders: up, down, up-down (ends not repeated), random, as played.
pub const MODES: u8 = 5;
pub const MODE_UP: u8 = 0;
pub const MODE_DOWN: u8 = 1;
pub const MODE_UP_DOWN: u8 = 2;
pub const MODE_RANDOM: u8 = 3;
pub const MODE_AS_PLAYED: u8 = 4;

/// Step times in quarter clock pulses (24 PPQN), slowest first: 1/4, 1/4T, 1/8, 1/8T, 1/16, 1/16T, 1/32, 1/32T.
pub const RATE_QP: [u32; 8] = [96, 64, 48, 32, 24, 16, 12, 8];
/// 1/16 (pad 5).
pub const DEFAULT_RATE: u8 = 4;
pub const MAX_OCTAVES: u8 = 4;

/// A rate is a triplet when it does not divide a straight step evenly.
pub const fn rate_is_triplet(rate: u8) -> bool {
    rate % 2 == 1
}

const MAX_NOTES: usize = 16;

#[derive(Copy, Clone)]
pub struct Arp {
    pub on: bool,
    pub latch: bool,
    pub mode: u8,
    /// 1-4.
    pub octaves: u8,
    /// Index into RATE_QP.
    pub rate: u8,
    /// The notes being arpeggiated, in the order they were played: (note, velocity, key still held).
    notes: [(u8, u8, bool); MAX_NOTES],
    count: usize,
    /// Steps played since the arp started (where in the sequence we are).
    pos: usize,
    /// Free-running timing (sequencer stopped): when the next note is due.
    pub next_us: u64,
    /// Clock-synced timing (sequencer playing): the step number last played.
    pub last_idx: u64,
}

impl Arp {
    pub const fn new() -> Self {
        Arp {
            on: false,
            latch: false,
            mode: MODE_UP,
            octaves: 1,
            rate: DEFAULT_RATE,
            notes: [(0, 0, false); MAX_NOTES],
            count: 0,
            pos: 0,
            next_us: 0,
            last_idx: 0,
        }
    }

    /// Notes to play (on, with at least one note).
    pub fn active(&self) -> bool {
        self.on && self.count > 0
    }

    pub fn has(&self, note: u8) -> bool {
        self.notes[..self.count].iter().any(|n| n.0 == note)
    }

    fn any_held(&self) -> bool {
        self.notes[..self.count].iter().any(|n| n.2)
    }

    /// A key went down. Returns true when this starts the arp (it had no notes).
    pub fn press(&mut self, note: u8, velocity: u8) -> bool {
        // Latched: the first key of a new hand (nothing held) replaces the latched notes.
        if self.latch && !self.any_held() {
            self.count = 0;
        }
        let started = self.count == 0;
        if started {
            self.pos = 0;
        }
        if let Some(n) = self.notes[..self.count].iter_mut().find(|n| n.0 == note) {
            *n = (note, velocity, true);
        } else if self.count < MAX_NOTES {
            self.notes[self.count] = (note, velocity, true);
            self.count += 1;
        }
        started
    }

    /// A key came up: the note leaves the arp unless latched.
    pub fn release(&mut self, note: u8) {
        if let Some(i) = self.notes[..self.count].iter().position(|n| n.0 == note) {
            if self.latch {
                self.notes[i].2 = false;
            } else {
                self.remove(i);
            }
        }
    }

    fn remove(&mut self, i: usize) {
        self.notes.copy_within(i + 1..self.count, i);
        self.count -= 1;
    }

    /// Latch off: notes whose keys are no longer held stop.
    pub fn set_latch(&mut self, latch: bool) {
        self.latch = latch;
        if !latch {
            let mut i = 0;
            while i < self.count {
                if self.notes[i].2 { i += 1 } else { self.remove(i) }
            }
        }
    }

    pub fn clear(&mut self) {
        self.count = 0;
        self.pos = 0;
    }

    /// The next note to play and its velocity. `random(n)` returns 0..n. None when there is nothing to play.
    pub fn next(&mut self, random: &mut dyn FnMut(usize) -> usize) -> Option<(u8, u8)> {
        if self.count == 0 {
            return None;
        }
        // The base notes in play order, then repeated an octave higher for each extra octave (notes above 127 left
        // out).
        let mut base = [(0u8, 0u8); MAX_NOTES];
        for (b, n) in base.iter_mut().zip(self.notes[..self.count].iter()) {
            *b = (n.0, n.1);
        }
        let base = &mut base[..self.count];
        match self.mode {
            MODE_DOWN => base.sort_unstable_by(|a, b| b.0.cmp(&a.0)),
            MODE_AS_PLAYED => {}
            _ => base.sort_unstable_by(|a, b| a.0.cmp(&b.0)),
        }
        let mut seq = [(0u8, 0u8); MAX_NOTES * MAX_OCTAVES as usize];
        let mut len = 0;
        for oct in 0..self.octaves.clamp(1, MAX_OCTAVES) {
            for &(n, v) in base.iter() {
                let note = n as u16 + 12 * oct as u16;
                if note <= 127 {
                    seq[len] = (note as u8, v);
                    len += 1;
                }
            }
        }
        if self.mode == MODE_DOWN {
            // Down over the octaves too: start at the top octave.
            seq[..len].sort_unstable_by(|a, b| b.0.cmp(&a.0));
        }
        if len == 0 {
            return None;
        }
        let i = match self.mode {
            MODE_RANDOM => random(len),
            MODE_UP_DOWN if len > 1 => {
                let period = 2 * len - 2;
                let k = self.pos % period;
                if k < len { k } else { period - k }
            }
            _ => self.pos % len,
        };
        self.pos = self.pos.wrapping_add(1);
        Some(seq[i])
    }
}
