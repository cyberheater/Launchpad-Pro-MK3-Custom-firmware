// SPDX-License-Identifier: GPL-3.0-only
//
// 8-track step sequencer.
//
// Layout (Programmer-mode XY indices):
//   Grid rows 8..5   steps 1-32 of the selected track's current pattern (row 8 = steps 1-8). Tap toggles a step.
//                    Hold a step + tap a note pad = set that step's note.
//   Grid rows 4..1   32 chromatic notes from the track's base note (row 1 = lowest). Tap = choose the note
//                    new steps get, and preview it. Up / Down (80 / 70) move the note range by an octave.
//   101-108          track select (the row under the grid).
//   1-8              track mutes.
//   Hold a track button: steps 1-16 show/set its MIDI channel; note row 1 pads 1-4 set its output
//                    (USB, DIN 1, DIN 2, DIN 1+2).
//   Hold a step:     right-hand column = velocity bar (top = 127 ... bottom = 16), bottom row = gate length in
//                    eighths of a step (1 = shortest, 8 = whole step); a lit step tapped and released without an edit
//                    turns off. Step brightness follows velocity.
//   Patterns (89)    hold: the grid shows 8 patterns (rows, A = top) for each of the 8 tracks (columns): current =
//                    track colour, chained = medium, has notes = dim, queued = white. Tap a pattern to switch to it
//                    (at the end of the playing pattern, or at once when stopped); hold one and tap another in the
//                    same column to chain the range. + Duplicate (50) held: copy the track's current pattern there.
//                    + Clear held: clear that pattern.
//   Hold Shift:      step pads show swing (pad 1 = 50 % straight ... pad 26 = 75 %); tap one to set it.
//   Hold a step + Shift: right-hand column = probability bar (top = 100 % ... bottom = 12 %).
//   Hold a track button: note rows 2-4 = transpose (-12 ... +11 semitones; the dim pad is 0).
//   Play (20)        start / stop.        Shift (90) + Up / Down: tempo +1 / -1 BPM (+10 / -10 with Fixed Length held).
//   Fixed Length (30) held + step pad: set the pattern length to that step.
//   Clear (60) held + step pad: clear the step; + track button: clear the track's current pattern.
//   Note (94)        back to the note/performance app; playback continues there (Sequencer button lit green).
//   Projects (98)    tap: save to the current project slot. Hold: the track row shows the 8 slots (white = current,
//                    dim = saved, off = empty); tap one to load it (an empty slot just becomes the save target).
//                    The Projects LED is green when everything is saved, amber with unsaved changes.
//                    The last used slot is loaded automatically the first time the sequencer opens.
//
// Timing: a 24 PPQN clock derived from the system timer (not from tick counting), or an external MIDI clock
// (USB MIDI port / DIN In: FA start, FB continue, FC stop, F8 pulses; clock passed on to both DIN outs; Play LED
// cyan while following; back to internal 500 ms after the last pulse). One step = a 16th note = 6 clock pulses.
// MIDI clock / start / stop go to USB and both DIN outputs.

use crate::app::apptrait::App;
use crate::app::events::{AftertouchEvent, MidiEvent, SurfaceEvent};
use crate::app::AppId;
use crate::driver;
use crate::sys::led;
use crate::sys::midi::MidiPort;

mod project;

const TRACKS: usize = project::TRACKS;
const PATTERNS: usize = project::PATTERNS;
const STEPS: usize = project::STEPS;
const MAX_OFFS: usize = 48;

const BTN_SHIFT: u8 = 90;
const BTN_NOTE_MODE: u8 = 94;
const BTN_SEQUENCER: u8 = 97;
const BTN_UP: u8 = 80;
const BTN_DOWN: u8 = 70;
const BTN_CLEAR: u8 = 60;
const BTN_DUPLICATE: u8 = 50;
const BTN_FIXED_LENGTH: u8 = 30;
const BTN_PLAY: u8 = 20;
const BTN_PROJECTS: u8 = 98;
const BTN_PATTERNS: u8 = 89;

const TRACK_COLOURS: [u32; TRACKS] =
    [0xFF2020, 0xFF8000, 0xFFE000, 0x40FF40, 0x00E0E0, 0x2060FF, 0xA040FF, 0xFF40C0];
const OUTPUT_COLOURS: [u32; 4] = [0xFFFFFF, 0x00FF60, 0x00A0FF, 0x00FFFF];

/// Velocity bar on the right-hand column while a step is held: top button = loudest.
const VELOCITY_LEVELS: [u8; 8] = [127, 112, 96, 80, 64, 48, 32, 16];
/// Probability bar (Shift + held step): top button = always.
const PROB_LEVELS: [u8; 8] = [100, 87, 75, 62, 50, 37, 25, 12];
/// Transpose pads: note pad index - TRANSPOSE_ZERO_PAD = semitones (pads 8-31 -> -12 ... +11).
const TRANSPOSE_ZERO_PAD: i8 = 20;

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum Output {
    Usb,
    Din1,
    Din2,
    Din12,
}

impl Output {
    const ALL: [Output; 4] = [Output::Usb, Output::Din1, Output::Din2, Output::Din12];
}

#[derive(Copy, Clone)]
struct Step {
    on: bool,
    note: u8,
    velocity: u8,
    /// Gate length in eighths of a step, 1-8 (8 = whole step).
    gate: u8,
    /// Chance the step plays, 1-100 %.
    prob: u8,
}

impl Step {
    const OFF: Step = Step { on: false, note: 60, velocity: 100, gate: 4, prob: 100 };
}

#[derive(Copy, Clone)]
struct Pattern {
    steps: [Step; STEPS],
    length: u8,
}

impl Pattern {
    const EMPTY: Pattern = Pattern { steps: [Step::OFF; STEPS], length: 16 };

    fn has_notes(&self) -> bool {
        self.steps.iter().any(|s| s.on)
    }
}

struct Track {
    patterns: [Pattern; PATTERNS],
    /// Pattern playing / being edited.
    pattern: u8,
    chain_start: u8,
    chain_end: u8,
    /// Pattern range to switch to at the end of the current pattern.
    queued: Option<(u8, u8)>,
    channel: u8, // 0-15
    output: Output,
    muted: bool,
    note: u8, // note given to newly enabled steps
    base: u8, // lowest note of the note area
    transpose: i8,
    position: u8, // next step to play
    playing_step: Option<u8>,
}

impl Track {
    const fn new(channel: u8) -> Self {
        Track {
            patterns: [Pattern::EMPTY; PATTERNS],
            pattern: 0,
            chain_start: 0,
            chain_end: 0,
            queued: None,
            channel,
            output: Output::Usb,
            muted: false,
            note: 60,
            base: 48,
            transpose: 0,
            position: 0,
            playing_step: None,
        }
    }

    fn pat(&self) -> &Pattern {
        &self.patterns[self.pattern as usize % PATTERNS]
    }

    fn pat_mut(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern as usize % PATTERNS]
    }

    /// At the end of a pattern: take a queued pattern/chain, or move along the current chain.
    fn next_pattern(&mut self) {
        if let Some((a, b)) = self.queued.take() {
            self.chain_start = a;
            self.chain_end = b;
            self.pattern = a;
        } else if self.chain_end > self.chain_start {
            self.pattern = if self.pattern < self.chain_start || self.pattern >= self.chain_end {
                self.chain_start
            } else {
                self.pattern + 1
            };
        }
    }
}

#[derive(Copy, Clone)]
struct NoteOff {
    due_us: u64,
    output: Output,
    channel: u8,
    note: u8,
}

pub struct SequencerApp {
    tracks: [Track; TRACKS],
    selected: usize,
    playing: bool,
    bpm: u16,
    /// Swing in percent (50 = straight, up to 75): delays every second 16th.
    swing: u8,
    rng: u32,
    /// Swung odd 16th waiting for its time: (due, step length).
    pending_step: Option<(u64, u64)>,
    next_clock_us: u64,
    clock_count: u32,
    offs: [Option<NoteOff>; MAX_OFFS],
    held_step: Option<u8>,
    /// The held step was lit when pressed: turn it off on release unless it was edited meanwhile.
    step_pending_off: bool,
    step_edited: bool,
    held_track: Option<usize>,
    preview: Option<(u8, Output, u8)>,
    shift: bool,
    fixed_length: bool,
    clear: bool,
    duplicate: bool,
    patterns_held: bool,
    pattern_hold: [Option<u8>; TRACKS],
    dirty: bool,
    switch: Option<AppId>,
    loaded: bool,
    slot: usize,
    slot_saved: [bool; project::SLOTS],
    projects_held: bool,
    projects_used: bool,
    saved_crc: u32,
    save_flash_until: u64,
    /// Following an external MIDI clock (USB MIDI port or DIN In); falls back to internal after 500 ms of silence.
    ext_active: bool,
    ext_last_pulse_us: u64,
    ext_interval_us: u64,
}

/// Microseconds since boot. Uses the system timer where embassy-time exists; the Launchpad S / Mini MK1 builds
/// have none, so there the clock advances by 1 ms per app tick (coarser, but those devices have no DIN anyway).
#[cfg(not(feature = "launchpad-s-and-mini-common"))]
fn now_us() -> u64 {
    embassy_time::Instant::now().as_micros()
}

#[cfg(feature = "launchpad-s-and-mini-common")]
static TICK_US: core::sync::atomic::AtomicU32 = core::sync::atomic::AtomicU32::new(0);

#[cfg(feature = "launchpad-s-and-mini-common")]
fn now_us() -> u64 {
    TICK_US.load(core::sync::atomic::Ordering::Relaxed) as u64
}

fn step_index(step: usize) -> u8 {
    ((8 - step / 8) * 10 + step % 8 + 1) as u8
}

fn note_pad_index(pad: usize) -> u8 {
    ((1 + pad / 8) * 10 + pad % 8 + 1) as u8
}

/// Grid index -> (step number) for rows 8..5.
fn step_from_index(index: u8) -> Option<usize> {
    let (row, col) = ((index / 10) as usize, (index % 10) as usize);
    if (5..=8).contains(&row) && (1..=8).contains(&col) { Some((8 - row) * 8 + col - 1) } else { None }
}

/// Grid index -> note pad number for rows 1..4.
fn note_pad_from_index(index: u8) -> Option<usize> {
    let (row, col) = ((index / 10) as usize, (index % 10) as usize);
    if (1..=4).contains(&row) && (1..=8).contains(&col) { Some((row - 1) * 8 + col - 1) } else { None }
}

/// Grid index -> (track, pattern) in the patterns view (columns = tracks, rows = patterns, A at the top).
fn pattern_from_index(index: u8) -> Option<(usize, u8)> {
    let (row, col) = ((index / 10) as usize, (index % 10) as usize);
    if (1..=8).contains(&row) && (1..=8).contains(&col) { Some((col - 1, (8 - row) as u8)) } else { None }
}

/// Scales a colour by n/256.
fn scale(c: u32, n: u32) -> u32 {
    let n = n.min(256);
    let r = ((c >> 16) & 0xFF) * n / 256;
    let g = ((c >> 8) & 0xFF) * n / 256;
    let b = (c & 0xFF) * n / 256;
    (r << 16) | (g << 8) | b
}

fn dim(c: u32, shift: u32) -> u32 {
    let r = ((c >> 16) & 0xFF) >> shift;
    let g = ((c >> 8) & 0xFF) >> shift;
    let b = (c & 0xFF) >> shift;
    (r << 16) | (g << 8) | b
}

fn send(output: Output, data: &[u8]) {
    match output {
        Output::Usb => driver::send_midi(MidiPort::Midi, data),
        Output::Din1 => driver::send_midi(MidiPort::Din, data),
        Output::Din2 => driver::send_midi(MidiPort::Din2, data),
        Output::Din12 => {
            driver::send_midi(MidiPort::Din, data);
            driver::send_midi(MidiPort::Din2, data);
        }
    }
}

fn send_all(data: &[u8]) {
    driver::send_midi(MidiPort::Midi, data);
    driver::send_midi(MidiPort::Din, data);
    driver::send_midi(MidiPort::Din2, data);
}

impl SequencerApp {
    pub const fn new() -> Self {
        SequencerApp {
            tracks: [
                Track::new(0),
                Track::new(1),
                Track::new(2),
                Track::new(3),
                Track::new(4),
                Track::new(5),
                Track::new(6),
                Track::new(7),
            ],
            selected: 0,
            playing: false,
            bpm: 120,
            swing: 50,
            rng: 0x1234_5678,
            pending_step: None,
            next_clock_us: 0,
            clock_count: 0,
            offs: [None; MAX_OFFS],
            held_step: None,
            step_pending_off: false,
            step_edited: false,
            held_track: None,
            preview: None,
            shift: false,
            fixed_length: false,
            clear: false,
            duplicate: false,
            patterns_held: false,
            pattern_hold: [None; TRACKS],
            dirty: true,
            switch: None,
            loaded: false,
            slot: 0,
            slot_saved: [false; project::SLOTS],
            projects_held: false,
            projects_used: false,
            saved_crc: 0,
            save_flash_until: 0,
            ext_active: false,
            ext_last_pulse_us: 0,
            ext_interval_us: 20_833,
        }
    }

    fn clock_interval_us(&self) -> u64 {
        60_000_000 / (self.bpm as u64 * 24)
    }

    fn random_percent(&mut self) -> u8 {
        // xorshift32
        let mut x = self.rng;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.rng = x;
        (x % 100) as u8 + 1
    }

    // ---------------- transport ----------------

    fn start(&mut self) {
        for t in self.tracks.iter_mut() {
            if let Some((a, b)) = t.queued.take() {
                t.chain_start = a;
                t.chain_end = b;
            }
            t.pattern = t.chain_start;
            t.position = 0;
            t.playing_step = None;
        }
        self.rng ^= now_us() as u32;
        self.clock_count = 0;
        self.next_clock_us = now_us();
        self.playing = true;
        self.send_transport(0xFA);
        self.dirty = true;
    }

    /// Transport/clock bytes: to every output when we are the clock master; when following an external clock only
    /// to the DIN outputs (passing the clock on), never back to USB where it may have come from.
    fn send_transport(&self, byte: u8) {
        if self.ext_active {
            driver::send_midi(MidiPort::Din, &[byte]);
            driver::send_midi(MidiPort::Din2, &[byte]);
        } else {
            send_all(&[byte]);
        }
    }

    /// MIDI real-time messages from the USB MIDI port or DIN In (routed here by the app host whatever app is active).
    pub fn on_realtime(&mut self, status: u8) {
        let now = now_us();
        match status {
            0xF8 => {
                if self.ext_active && self.ext_last_pulse_us != 0 {
                    let d = now.saturating_sub(self.ext_last_pulse_us);
                    if (2_000..200_000).contains(&d) {
                        self.ext_interval_us = (self.ext_interval_us * 3 + d) / 4;
                    }
                }
                self.ext_active = true;
                self.ext_last_pulse_us = now;
                self.send_transport(0xF8);
                if self.playing {
                    self.pulse(now, self.ext_interval_us);
                }
            }
            0xFA => {
                self.ext_active = true;
                self.ext_last_pulse_us = now;
                if self.playing {
                    self.stop_notes();
                }
                self.start();
            }
            0xFB => {
                self.ext_active = true;
                self.ext_last_pulse_us = now;
                self.playing = true;
                self.send_transport(0xFB);
                self.dirty = true;
            }
            0xFC => {
                if self.playing {
                    self.stop();
                }
            }
            _ => {}
        }
    }

    fn stop_notes(&mut self) {
        for slot in self.offs.iter_mut() {
            if let Some(off) = slot.take() {
                send(off.output, &[0x80 | off.channel, off.note, 0]);
            }
        }
    }

    fn stop(&mut self) {
        self.playing = false;
        self.pending_step = None;
        self.stop_notes();
        for t in self.tracks.iter_mut() {
            t.playing_step = None;
        }
        self.send_transport(0xFC);
        self.dirty = true;
    }

    /// One clock pulse (24 PPQN). Even 16ths play on pulse 0 of their 8th-note pair; odd 16ths are due at pulse 6,
    /// delayed by swing (50 % = straight, 75 % = triplet-ish), timed to the millisecond rather than to whole pulses.
    fn pulse(&mut self, now: u64, interval: u64) {
        let in_pair = self.clock_count % 12;
        let pair_us = interval * 12;
        let swing_us = pair_us * (self.swing.clamp(50, 75) as u64 - 50) / 100;
        if in_pair == 0 {
            if let Some((due, len)) = self.pending_step.take() {
                self.play_step(due.min(now), len);
            }
            self.play_step(now, pair_us / 2 + swing_us);
        } else if in_pair == 6 {
            let len = pair_us / 2 - swing_us;
            if swing_us < 1000 {
                self.play_step(now, len);
            } else {
                self.pending_step = Some((now + swing_us, len));
            }
        }
        self.clock_count = self.clock_count.wrapping_add(1);
    }

    fn run_clock(&mut self, now: u64) {
        if self.ext_active {
            if now.saturating_sub(self.ext_last_pulse_us) < 500_000 {
                return; // steps advance on incoming clock pulses
            }
            // External clock went away: carry on with the internal clock from here.
            self.ext_active = false;
            self.next_clock_us = now;
            self.dirty = true;
        }
        if !self.playing {
            return;
        }
        let interval = self.clock_interval_us();
        // After a long stall, resynchronise instead of firing a burst of steps.
        if now > self.next_clock_us + 100_000 {
            self.next_clock_us = now;
        }
        while now >= self.next_clock_us {
            let pulse_time = self.next_clock_us;
            send_all(&[0xF8]);
            self.pulse(pulse_time, interval);
            self.next_clock_us += interval;
        }
    }

    fn play_step(&mut self, time: u64, step_us: u64) {
        for t in 0..TRACKS {
            let (pos, step, muted, output, channel, transpose) = {
                let tr = &self.tracks[t];
                let pos = tr.position as usize % STEPS;
                (pos, tr.pat().steps[pos], tr.muted, tr.output, tr.channel, tr.transpose)
            };
            if step.on && !muted && (step.prob >= 100 || self.random_percent() <= step.prob) {
                let note = (step.note as i16 + transpose as i16).clamp(0, 127) as u8;
                self.note_off_now(output, channel, note);
                send(output, &[0x90 | channel, note, step.velocity]);
                let gate_us = (step_us * step.gate.clamp(1, 8) as u64 / 8).saturating_sub(200).max(1000);
                self.schedule_off(NoteOff { due_us: time + gate_us, output, channel, note });
            }
            let tr = &mut self.tracks[t];
            tr.playing_step = Some(pos as u8);
            let len = tr.pat().length.clamp(1, STEPS as u8) as usize;
            tr.position = ((pos + 1) % len) as u8;
            if tr.position == 0 {
                tr.next_pattern();
            }
        }
        self.dirty = true;
    }

    fn schedule_off(&mut self, off: NoteOff) {
        if let Some(slot) = self.offs.iter_mut().find(|s| s.is_none()) {
            *slot = Some(off);
        } else {
            send(off.output, &[0x80 | off.channel, off.note, 0]);
        }
    }

    /// Ends a still-sounding copy of the same note before retriggering it.
    fn note_off_now(&mut self, output: Output, channel: u8, note: u8) {
        for slot in self.offs.iter_mut() {
            if let Some(off) = *slot {
                if off.output == output && off.channel == channel && off.note == note {
                    send(output, &[0x80 | channel, note, 0]);
                    *slot = None;
                }
            }
        }
    }

    fn run_note_offs(&mut self, now: u64) {
        for slot in self.offs.iter_mut() {
            if let Some(off) = *slot {
                if now >= off.due_us {
                    send(off.output, &[0x80 | off.channel, off.note, 0]);
                    *slot = None;
                }
            }
        }
    }

    // ---------------- input ----------------

    fn on_step_pad(&mut self, step: usize, pressed: bool) {
        if !pressed {
            if self.held_step == Some(step as u8) {
                if self.step_pending_off && !self.step_edited {
                    self.tracks[self.selected].pat_mut().steps[step].on = false;
                }
                self.held_step = None;
                self.dirty = true;
            }
            return;
        }
        if let Some(t) = self.held_track {
            if step < 16 {
                self.tracks[t].channel = step as u8;
            }
            self.dirty = true;
            return;
        }
        if self.shift && self.held_step.is_none() {
            if step < 26 {
                self.swing = 50 + step as u8;
            }
            self.dirty = true;
            return;
        }
        let track = &mut self.tracks[self.selected];
        let note = track.note;
        if self.fixed_length {
            track.pat_mut().length = step as u8 + 1;
            if track.position > step as u8 {
                track.position = 0;
            }
        } else if self.clear {
            track.pat_mut().steps[step] = Step::OFF;
        } else {
            let s = &mut track.pat_mut().steps[step];
            self.step_pending_off = s.on;
            self.step_edited = false;
            if !s.on {
                *s = Step { on: true, note, ..Step::OFF };
            }
            self.held_step = Some(step as u8);
        }
        self.dirty = true;
    }

    fn on_note_pad(&mut self, pad: usize, pressed: bool, value: u8) {
        if let Some(t) = self.held_track {
            if pressed && pad < 4 {
                self.tracks[t].output = Output::ALL[pad];
            } else if pressed && pad >= 8 {
                self.tracks[t].transpose = pad as i8 - TRANSPOSE_ZERO_PAD;
            }
            self.dirty = true;
            return;
        }
        let sel = self.selected;
        let note = (self.tracks[sel].base as usize + pad).min(127) as u8;
        if pressed {
            if let Some(step) = self.held_step {
                let s = &mut self.tracks[sel].pat_mut().steps[step as usize];
                s.on = true;
                s.note = note;
                self.step_edited = true;
            }
            self.tracks[sel].note = note;
            let (output, channel) = (self.tracks[sel].output, self.tracks[sel].channel);
            if let Some((n, o, c)) = self.preview.take() {
                send(o, &[0x80 | c, n, 0]);
            }
            send(output, &[0x90 | channel, note, value.max(1).min(127)]);
            self.preview = Some((note, output, channel));
        } else if let Some((n, o, c)) = self.preview {
            if n == note {
                send(o, &[0x80 | c, n, 0]);
                self.preview = None;
            }
        }
        self.dirty = true;
    }

    fn on_track_button(&mut self, t: usize, pressed: bool) {
        if pressed {
            if self.clear {
                *self.tracks[t].pat_mut() = Pattern::EMPTY;
            } else {
                self.selected = t;
                self.held_track = Some(t);
            }
        } else if self.held_track == Some(t) {
            self.held_track = None;
        }
        self.dirty = true;
    }

    fn on_pattern_pad(&mut self, t: usize, p: u8, pressed: bool) {
        if !pressed {
            if self.pattern_hold[t] == Some(p) {
                self.pattern_hold[t] = None;
            }
            return;
        }
        self.dirty = true;
        if self.duplicate {
            let tr = &mut self.tracks[t];
            tr.patterns[p as usize] = *tr.pat();
            return;
        }
        if self.clear {
            self.tracks[t].patterns[p as usize] = Pattern::EMPTY;
            return;
        }
        self.selected = t;
        let range = match self.pattern_hold[t] {
            Some(first) if first != p => (first.min(p), first.max(p)),
            _ => {
                self.pattern_hold[t] = Some(p);
                (p, p)
            }
        };
        let tr = &mut self.tracks[t];
        if self.playing {
            tr.queued = Some(range);
        } else {
            tr.chain_start = range.0;
            tr.chain_end = range.1;
            tr.pattern = range.0;
            tr.position = 0;
            tr.queued = None;
        }
    }

    fn shift_octave(&mut self, up: bool) {
        if let Some(t) = self.held_track {
            let ch = &mut self.tracks[t].channel;
            *ch = if up { (*ch + 1).min(15) } else { ch.saturating_sub(1) };
        } else if self.shift {
            let delta: u16 = if self.fixed_length { 10 } else { 1 };
            self.bpm = if up { (self.bpm + delta).min(300) } else { self.bpm.saturating_sub(delta).max(30) };
        } else {
            let base = &mut self.tracks[self.selected].base;
            *base = if up { (*base + 12).min(96) } else { base.saturating_sub(12) };
        }
        self.dirty = true;
    }

    /// Clock, steps and note-offs. Called every 1 ms tick: by on_tick while the sequencer is the active app, and by
    /// the app host while another app is in front (background playback). Never touches the LEDs.
    pub fn run_engine(&mut self) {
        #[cfg(feature = "launchpad-s-and-mini-common")]
        TICK_US.fetch_add(1000, core::sync::atomic::Ordering::Relaxed);
        let now = now_us();
        self.run_clock(now);
        if let Some((due, len)) = self.pending_step {
            if !self.playing {
                self.pending_step = None;
            } else if now >= due {
                self.pending_step = None;
                self.play_step(due, len);
            }
        }
        self.run_note_offs(now);
        if self.save_flash_until != 0 && now >= self.save_flash_until {
            self.save_flash_until = 0;
            self.dirty = true;
        }
    }

    pub fn is_playing(&self) -> bool {
        self.playing
    }

    // ---------------- projects ----------------

    /// Streams the project body (everything before the CRC) to `sink`, in the version-2 layout.
    fn emit(&self, sink: &mut dyn FnMut(&[u8])) {
        sink(&project::header(self.bpm, self.swing));
        for tr in self.tracks.iter() {
            let mut s = [0u8; project::TRACK_SETTINGS];
            s[0] = tr.channel;
            s[1] = Output::ALL.iter().position(|o| *o == tr.output).unwrap_or(0) as u8;
            s[2] = tr.muted as u8;
            s[3] = tr.note;
            s[4] = tr.base;
            s[5] = tr.pattern;
            s[6] = tr.chain_start;
            s[7] = tr.chain_end;
            s[8] = tr.transpose as u8;
            sink(&s);
            for pat in tr.patterns.iter() {
                let mut b = [0u8; project::PATTERN_BYTES];
                b[0] = pat.length;
                for (i, st) in pat.steps.iter().enumerate() {
                    b[1 + i * 4] = ((st.on as u8) << 7) | (st.note & 0x7f);
                    b[2 + i * 4] = st.velocity;
                    b[3 + i * 4] = st.gate;
                    b[4 + i * 4] = st.prob;
                }
                sink(&b);
            }
        }
    }

    fn current_crc(&self) -> u32 {
        let mut crc = project::Crc::new();
        self.emit(&mut |b| crc.update(b));
        crc.value()
    }

    fn load_record(&mut self, rec: &[u8; project::RECORD]) {
        self.bpm = u16::from_le_bytes([rec[5], rec[6]]).clamp(30, 300);
        self.swing = rec[7].clamp(50, 75);
        for (t, tr) in self.tracks.iter_mut().enumerate() {
            let base = project::HEADER + t * project::TRACK_BYTES;
            let s = &rec[base..base + project::TRACK_SETTINGS];
            tr.channel = s[0] & 0x0f;
            tr.output = Output::ALL[(s[1] as usize).min(3)];
            tr.muted = s[2] != 0;
            tr.note = s[3] & 0x7f;
            tr.base = s[4].min(96);
            tr.pattern = s[5] % PATTERNS as u8;
            tr.chain_start = s[6] % PATTERNS as u8;
            tr.chain_end = (s[7] % PATTERNS as u8).max(tr.chain_start);
            tr.transpose = (s[8] as i8).clamp(-24, 24);
            for (p, pat) in tr.patterns.iter_mut().enumerate() {
                let o = base + project::TRACK_SETTINGS + p * project::PATTERN_BYTES;
                let b = &rec[o..o + project::PATTERN_BYTES];
                pat.length = b[0].clamp(1, STEPS as u8);
                for (i, st) in pat.steps.iter_mut().enumerate() {
                    st.on = b[1 + i * 4] & 0x80 != 0;
                    st.note = b[1 + i * 4] & 0x7f;
                    st.velocity = b[2 + i * 4].clamp(1, 127);
                    st.gate = b[3 + i * 4].clamp(1, 8);
                    st.prob = b[4 + i * 4].clamp(1, 100);
                }
            }
            tr.queued = None;
            tr.position = 0;
            tr.playing_step = None;
        }
    }

    /// Imports a version-1 (single pattern) record into pattern A of each track.
    fn load_v1(&mut self, rec: &[u8; project::V1_RECORD]) {
        self.bpm = u16::from_le_bytes([rec[5], rec[6]]).clamp(30, 300);
        self.swing = 50;
        for (t, tr) in self.tracks.iter_mut().enumerate() {
            let b = &rec[8 + t * project::V1_TRACK_BYTES..8 + (t + 1) * project::V1_TRACK_BYTES];
            *tr = Track::new(t as u8);
            let pat = &mut tr.patterns[0];
            pat.length = b[0].clamp(1, STEPS as u8);
            tr.channel = b[1] & 0x0f;
            tr.output = Output::ALL[(b[2] as usize).min(3)];
            tr.muted = b[3] != 0;
            tr.note = b[4] & 0x7f;
            tr.base = b[5].min(96);
            for (i, st) in tr.patterns[0].steps.iter_mut().enumerate() {
                st.on = b[6 + i * 2] & 0x80 != 0;
                st.note = b[6 + i * 2] & 0x7f;
                st.velocity = b[7 + i * 2].clamp(1, 127);
            }
        }
    }

    fn scan_slots(&mut self) {
        let mut saved = [false; project::SLOTS];
        project::with_buffer(|rec| {
            for (s, flag) in saved.iter_mut().enumerate() {
                *flag = project::read_slot(s, rec);
            }
        });
        self.slot_saved = saved;
    }

    /// First entry after boot: load the last used project.
    fn load_initial(&mut self) {
        self.loaded = true;
        if !project::available() {
            self.saved_crc = self.current_crc();
            return;
        }
        self.scan_slots();
        let slot = project::last_slot();
        self.load_slot(slot, false);
    }

    fn load_slot(&mut self, slot: usize, remember: bool) {
        self.slot = slot;
        let loaded = project::with_buffer(|rec| {
            if project::read_slot(slot, rec) {
                if self.playing {
                    self.stop();
                }
                self.load_record(rec);
                true
            } else {
                false
            }
        });
        if loaded {
            self.saved_crc = self.current_crc();
        } else {
            let mut v1 = [0u8; project::V1_RECORD];
            if project::read_v1_slot(slot, &mut v1) {
                if self.playing {
                    self.stop();
                }
                self.load_v1(&v1);
                self.saved_crc = 0; // not yet saved in the new format
            } else {
                // Empty slot: keep the current project; it becomes the save target ("save as").
                self.saved_crc = 0;
            }
        }
        if remember {
            project::set_last_slot(slot);
        }
        self.dirty = true;
    }

    fn save(&mut self) {
        if !project::available() {
            return;
        }
        let (ok, crc) = project::with_buffer(|rec| {
            let mut pos = 0;
            self.emit(&mut |b| {
                rec[pos..pos + b.len()].copy_from_slice(b);
                pos += b.len();
            });
            project::seal(rec);
            (project::write_slot(self.slot, rec), project::crc32(&rec[..project::BODY]))
        });
        if ok {
            self.slot_saved[self.slot] = true;
            self.saved_crc = crc;
            self.save_flash_until = now_us() + 400_000;
        }
        self.dirty = true;
    }

    // ---------------- display ----------------

    fn draw(&mut self) {
        let sel = self.selected;
        let colour = TRACK_COLOURS[sel];

        if self.patterns_held {
            for t in 0..TRACKS {
                let tr = &self.tracks[t];
                let tc = TRACK_COLOURS[t];
                for p in 0..PATTERNS as u8 {
                    let queued = tr.queued.map_or(false, |(a, b)| (a..=b).contains(&p));
                    let c = if queued {
                        0xFFFFFF
                    } else if p == tr.pattern {
                        tc
                    } else if (tr.chain_start..=tr.chain_end).contains(&p) {
                        scale(tc, 110)
                    } else if tr.patterns[p as usize].has_notes() {
                        dim(tc, 3)
                    } else {
                        0x050505
                    };
                    led::set((8 - p) * 10 + t as u8 + 1, c);
                }
            }
        } else if let Some(t) = self.held_track {
            // Track settings: channel on steps 1-16, outputs on note row 1.
            for step in 0..STEPS {
                let c = if step < 16 {
                    if step as u8 == self.tracks[t].channel { 0xFFFFFF } else { 0x101010 }
                } else {
                    0
                };
                led::set(step_index(step), c);
            }
            let transpose_pad = (self.tracks[t].transpose + TRANSPOSE_ZERO_PAD) as usize;
            for pad in 0..32 {
                let c = if pad < 4 {
                    let o = OUTPUT_COLOURS[pad];
                    if Output::ALL[pad] == self.tracks[t].output { o } else { dim(o, 3) }
                } else if pad >= 8 {
                    if pad == transpose_pad {
                        0xFFE000
                    } else if pad == TRANSPOSE_ZERO_PAD as usize {
                        0x303030
                    } else {
                        0x080600
                    }
                } else {
                    0
                };
                led::set(note_pad_index(pad), c);
            }
        } else {
            let track = &self.tracks[sel];
            let pat = track.pat();
            let swing_view = self.shift && self.held_step.is_none();
            for step in 0..STEPS {
                let s = pat.steps[step];
                let c = if swing_view {
                    if step >= 26 { 0 } else if step as u8 <= self.swing - 50 { 0xFFA000 } else { 0x100800 }
                } else if track.playing_step == Some(step as u8) && self.playing {
                    0xFFFFFF
                } else if step as u8 >= pat.length {
                    0
                } else if s.on {
                    scale(colour, 64 + s.velocity as u32 * 3 / 2)
                } else if step % 4 == 0 {
                    0x181818
                } else {
                    0x080808
                };
                led::set(step_index(step), c);
            }
            let shown_note = match self.held_step {
                Some(st) if pat.steps[st as usize].on => pat.steps[st as usize].note,
                _ => track.note,
            };
            for pad in 0..32 {
                let note = track.base as usize + pad;
                let c = if note > 127 {
                    0
                } else if note as u8 == shown_note {
                    colour
                } else if note % 12 == 0 {
                    0x303060
                } else if matches!(note % 12, 1 | 3 | 6 | 8 | 10) {
                    0x040404
                } else {
                    0x141414
                };
                led::set(note_pad_index(pad), c);
            }
        }

        for t in 0..TRACKS {
            let tc = TRACK_COLOURS[t];
            let track_led = if self.projects_held {
                if t == self.slot { 0xFFFFFF } else if self.slot_saved[t] { 0x303030 } else { 0 }
            } else if t == sel {
                tc
            } else {
                dim(tc, 3)
            };
            led::set(101 + t as u8, track_led);
            let tr = &self.tracks[t];
            let playing_note = self.playing && tr.playing_step.map_or(false, |p| tr.pat().steps[p as usize].on);
            led::set(1 + t as u8, if tr.muted { 0x300000 } else if playing_note { 0x40FF40 } else { 0x084008 });
        }

        // Step edit bars: velocity on the right-hand column, gate on the bottom row (replaces the mute row).
        let held = if self.held_track.is_none() && !self.patterns_held { self.held_step } else { None };
        let held_step = held.map(|st| self.tracks[sel].pat().steps[st as usize]);
        for k in 0..8u8 {
            let idx = (8 - k) * 10 + 9;
            let c = match held_step {
                Some(s) if self.shift => {
                    if PROB_LEVELS[k as usize] <= s.prob { 0x00E0FF } else { 0x080808 }
                }
                Some(s) => {
                    if VELOCITY_LEVELS[k as usize] <= s.velocity { colour } else { 0x080808 }
                }
                None if idx == BTN_PATTERNS => {
                    if self.patterns_held { 0xFFFFFF } else { 0x303030 }
                }
                None => 0,
            };
            led::set(idx, c);
        }
        if let Some(s) = held_step {
            for n in 1..=8u8 {
                led::set(n, if n <= s.gate { 0xFFFFFF } else { 0x101010 });
            }
        }

        let beat = self.playing && (self.clock_count.wrapping_sub(1) / 24) % 4 == 0;
        let (on, off) = if self.ext_active { (0x40FFFF, 0x106060) } else { (0x40FF40, 0x106010) };
        led::set(BTN_PLAY, if self.playing { if beat { on } else { off } } else if self.ext_active { 0x103030 } else { 0x101010 });
        led::set(BTN_SEQUENCER, 0xFFFFFF);
        led::set(BTN_NOTE_MODE, 0x202020);
        led::set(BTN_UP, 0x202020);
        led::set(BTN_DOWN, 0x202020);
        led::set(BTN_CLEAR, if self.clear { 0xFF4040 } else { 0x200808 });
        led::set(BTN_DUPLICATE, if self.duplicate { 0xFFFFFF } else { 0x202020 });
        led::set(BTN_FIXED_LENGTH, if self.fixed_length { 0xFFFFFF } else { 0x202020 });
        let projects = if !project::available() {
            0
        } else if now_us() < self.save_flash_until {
            0xFFFFFF
        } else if self.current_crc() == self.saved_crc {
            0x00C000
        } else {
            0xFF8000
        };
        led::set(BTN_PROJECTS, projects);
        self.dirty = false;
    }
}

impl App for SequencerApp {
    fn on_enter(&mut self) {
        led::clear();
        if !self.loaded {
            self.load_initial();
        }
        self.held_step = None;
        self.held_track = None;
        self.dirty = true;
        self.draw();
    }

    fn on_exit(&mut self) {
        if let Some((n, o, c)) = self.preview.take() {
            send(o, &[0x80 | c, n, 0]);
        }
        // Playback continues in the background (the app host keeps calling run_engine); only release held controls.
        self.held_step = None;
        self.held_track = None;
        self.projects_held = false;
        self.patterns_held = false;
        self.pattern_hold = [None; TRACKS];
        self.shift = false;
        self.clear = false;
        self.duplicate = false;
        self.fixed_length = false;
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        let i = event.index;
        if let (true, Some((t, p))) = (self.patterns_held, pattern_from_index(i)) {
            self.on_pattern_pad(t, p, event.pressed);
        } else if let Some(step) = step_from_index(i) {
            self.on_step_pad(step, event.pressed);
        } else if let Some(pad) = note_pad_from_index(i) {
            self.on_note_pad(pad, event.pressed, event.value);
        } else if (101..=108).contains(&i) && self.projects_held {
            if event.pressed {
                self.projects_used = true;
                self.load_slot((i - 101) as usize, true);
            }
        } else if (101..=108).contains(&i) {
            self.on_track_button((i - 101) as usize, event.pressed);
        } else if let (Some(step), true, true) = (self.held_step, i % 10 == 9, (1..=8).contains(&(i / 10))) {
            if event.pressed {
                let shift = self.shift;
                let s = &mut self.tracks[self.selected].pat_mut().steps[step as usize];
                if shift {
                    s.prob = PROB_LEVELS[(8 - i / 10) as usize];
                } else {
                    s.velocity = VELOCITY_LEVELS[(8 - i / 10) as usize];
                }
                self.step_edited = true;
                self.dirty = true;
            }
        } else if let (Some(step), true) = (self.held_step, (1..=8).contains(&i)) {
            if event.pressed {
                self.tracks[self.selected].pat_mut().steps[step as usize].gate = i;
                self.step_edited = true;
                self.dirty = true;
            }
        } else if (1..=8).contains(&i) {
            if event.pressed {
                let t = &mut self.tracks[(i - 1) as usize];
                t.muted = !t.muted;
                self.dirty = true;
            }
        } else {
            match i {
                BTN_SHIFT => {
                    self.shift = event.pressed;
                    self.dirty = true;
                }
                BTN_FIXED_LENGTH => {
                    self.fixed_length = event.pressed;
                    self.dirty = true;
                }
                BTN_CLEAR => {
                    self.clear = event.pressed;
                    self.dirty = true;
                }
                BTN_DUPLICATE => {
                    self.duplicate = event.pressed;
                    self.dirty = true;
                }
                BTN_PATTERNS => {
                    self.patterns_held = event.pressed;
                    if !event.pressed {
                        self.pattern_hold = [None; TRACKS];
                    }
                    self.dirty = true;
                }
                BTN_PLAY if event.pressed => {
                    if self.playing { self.stop() } else { self.start() }
                }
                BTN_UP if event.pressed => self.shift_octave(true),
                BTN_DOWN if event.pressed => self.shift_octave(false),
                BTN_NOTE_MODE if event.pressed => self.switch = Some(AppId::Performance),
                BTN_PROJECTS => {
                    if event.pressed {
                        self.projects_held = true;
                        self.projects_used = false;
                    } else {
                        self.projects_held = false;
                        if !self.projects_used {
                            self.save();
                        }
                    }
                    self.dirty = true;
                }
                _ => {}
            }
        }
    }

    fn on_midi(&mut self, _event: MidiEvent) {}

    fn on_aftertouch(&mut self, _event: AftertouchEvent) {}

    fn on_tick(&mut self) {
        self.run_engine();
        if self.dirty {
            self.draw();
        }
    }

    fn take_requested_app_switch(&mut self) -> Option<AppId> {
        self.switch.take()
    }
}
