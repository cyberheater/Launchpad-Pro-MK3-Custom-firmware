// SPDX-License-Identifier: GPL-3.0-only
//
// 8-track step sequencer.
//
// Layout (Programmer-mode XY indices):
//   Grid rows 8..5   steps 1-32 of the selected track's current pattern (row 8 = steps 1-8). Tap toggles a step.
//                    Hold a step + tap a note pad = set that step's note; tap more pads in the same hold to build a
//                    chord (up to 8 notes; tap a lit one again to remove it). Holding a step shows all its notes.
//   Grid rows 4..1   32 chromatic notes from the track's base note (row 1 = lowest). Tap = choose the note
//                    new steps get, and preview it. Up / Down (80 / 70) move the note range by an octave.
//   101-108          track select (the row under the grid).
//   1-8              track mutes.
//   Hold a track button: steps 1-16 show/set its MIDI channel; note row 1 pads 1-4 set its output
//                    (USB, DIN 1, DIN 2, DIN 1+2; new tracks start on DIN 1, channel = track number).
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
//   Shift held:      the note area shows the tempo (orange bar = hundreds, then tens and units in white), and the
//                    Sends button (its printed Shift label is "Tap") is tap tempo: tap it in time; the average of the
//                    last few taps sets the BPM (a 2 s pause starts over; ignored while following an external clock).
//   Fixed Length (30) held + step pad: set the pattern length to that step.
//   Clear (60) held + step pad: clear the step; + track button: clear the track's current pattern.
//   Note (94)        back to the note/performance app; playback continues there (Sequencer button lit green).
//   Record (10)      tap: arm / disarm live recording (red). While armed and playing, notes played on the note pads or
//                    arriving on the USB MIDI port / DIN In (e.g. a keyboard) are written into the selected track's
//                    current pattern at the nearest step, with their velocity and held length as the gate (up to one
//                    step). A note recorded just ahead of a step is not replayed on that pass (you already heard it).
//                    Notes played together land on the same step as a chord (up to 8; loudest velocity kept).
//   MIDI thru        a keyboard on the USB MIDI port / DIN In plays the selected track: notes, aftertouch, controllers,
//                    program change and pitch bend are re-sent on that track's output and channel. Echoes from a
//                    keyboard with MIDI Thru are recognised and dropped (no loops, no double recording).
//   Projects (98)    tap: save to the current project slot. Hold: the track row shows the 8 slots (white = current,
//                    dim = saved, off = empty); tap one to load it (an empty slot just becomes the save target).
//                    The Projects LED is green when everything is saved, amber with unsaved changes.
//                    The last used slot is loaded automatically the first time the sequencer opens.
//
// Timing: a 24 PPQN clock derived from the system timer (not from tick counting), or an external MIDI clock
// (USB MIDI port / DIN In: FA start, FB continue, FC stop, F8 pulses; clock passed on to both DIN outs; Play LED
// cyan while following; back to internal 500 ms after the last pulse). One step = a 16th note = 6 clock pulses.
// Shift + Play toggles following an external clock (while Shift is held, Play is cyan = follow, orange = ignore
// incoming clock / start / stop and always use the internal tempo); the setting is saved with the project.
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
const MAX_OFFS: usize = 160;

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
const BTN_RECORD: u8 = 10;
const BTN_PATTERNS: u8 = 89;
/// The "Sends" button under the track row: its printed Shift function is "Tap".
const BTN_TAP: u8 = 6;
/// Taps averaged for tap tempo, and the gap after which tapping starts over.
const TAP_HISTORY: usize = 5;
const TAP_RESET_US: u64 = 2_000_000;

/// 3 x 4 digits for the BPM readout, rows top to bottom, bit 2 = left column.
const DIGITS: [[u8; 4]; 10] = [
    [0b111, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b111], // 1
    [0b111, 0b011, 0b100, 0b111], // 2
    [0b111, 0b011, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001], // 4
    [0b111, 0b110, 0b001, 0b111], // 5
    [0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010], // 7
    [0b111, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001], // 9
];

/// BPM readout on the 8 x 4 note area (`col` 0-7 left to right, `row` 0-3 top to bottom): column 1 is the hundreds
/// as a bar (1-3 pads from the bottom), columns 2-4 the tens digit, 6-8 the units digit.
fn bpm_pixel(bpm: u16, col: usize, row: usize) -> u32 {
    const DIGIT: u32 = 0xFFFFFF;
    const HUNDREDS: u32 = 0xFF8000;
    let (h, t, u) = ((bpm / 100) as usize, (bpm / 10 % 10) as usize, (bpm % 10) as usize);
    let lit = |d: usize, x: usize| DIGITS[d][row] >> (2 - x) & 1 != 0;
    match col {
        0 => if 3 - row < h { HUNDREDS } else { 0x080400 },
        1..=3 => if (h > 0 || t > 0) && lit(t, col - 1) { DIGIT } else { 0 },
        5..=7 => if lit(u, col - 5) { DIGIT } else { 0 },
        _ => 0,
    }
}

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

/// Notes per step (a chord). Unused entries hold NO_NOTE.
const MAX_NOTES: usize = project::MAX_NOTES;
const NO_NOTE: u8 = project::NO_NOTE;

#[derive(Copy, Clone)]
struct Step {
    on: bool,
    /// 1-8 notes, in the order they were added; notes[0] is always set.
    notes: [u8; MAX_NOTES],
    velocity: u8,
    /// Gate length in eighths of a step, 1-8 (8 = whole step).
    gate: u8,
    /// Chance the step plays, 1-100 %.
    prob: u8,
}

impl Step {
    const OFF: Step = Step { on: false, notes: Step::chord(60), velocity: 100, gate: 4, prob: 100 };

    /// A note list holding just `note`.
    const fn chord(note: u8) -> [u8; MAX_NOTES] {
        let mut notes = [NO_NOTE; MAX_NOTES];
        notes[0] = note;
        notes
    }

    fn single(note: u8) -> Step {
        Step { on: true, notes: Step::chord(note), ..Step::OFF }
    }

    fn notes(&self) -> impl Iterator<Item = u8> + '_ {
        self.notes.iter().copied().filter(|&n| n != NO_NOTE)
    }

    fn has(&self, note: u8) -> bool {
        self.notes.contains(&note)
    }

    /// Adds a note to the chord (no-op if present); when full, the newest note is replaced.
    fn add(&mut self, note: u8) {
        if self.has(note) {
            return;
        }
        match self.notes.iter_mut().find(|n| **n == NO_NOTE) {
            Some(slot) => *slot = note,
            None => self.notes[MAX_NOTES - 1] = note,
        }
    }

    /// Removes a note, keeping at least one.
    fn remove(&mut self, note: u8) {
        if self.notes().count() <= 1 {
            return;
        }
        let mut kept = [NO_NOTE; MAX_NOTES];
        for (slot, n) in kept.iter_mut().zip(self.notes().filter(|&n| n != note)) {
            *slot = n;
        }
        self.notes = kept;
    }
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
            // New tracks play out of DIN 1 (the point of a standalone sequencer); change per track as needed.
            output: Output::Din1,
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
    /// When its note-on went out (to recognise a keyboard echoing it straight back).
    sent_us: u64,
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
    /// When the preview note went out.
    preview_us: u64,
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
    /// Ignore incoming MIDI clock / start / stop (Shift + Play toggles; saved with the project).
    ext_ignore: bool,
    /// When we last sent F8 / FA / FB / FC: the same byte arriving within RT_ECHO_US of that is our own echo.
    rt_sent_us: core::cell::Cell<[u64; 4]>,
    /// Tap tempo (Shift + Sends/Tap): times of the recent taps, newest last; 0 = none.
    taps: [u64; TAP_HISTORY],
    /// Live recording armed (Record button); records while playing.
    record_armed: bool,
    /// Time and length of the last played step (for quantising recorded notes).
    last_step_us: u64,
    last_step_len_us: u64,
    /// Notes being recorded, waiting for their note-off: (note, track, step, start time).
    rec_held: [Option<(u8, u8, u8, u64)>; 8],
    /// Per track: a step just recorded ahead of the playhead, not to be replayed on this pass.
    skip_once: [Option<u8>; TRACKS],
    /// Last recorded step (track, step, time): notes landing on it within a step's time join it as a chord.
    rec_step: Option<(u8, u8, u64)>,
    /// A note pad has been tapped during the current step hold (the first tap replaces the chord, later ones add).
    hold_notes_set: bool,
    /// Keyboard notes being passed through: (note, output, channel), so note-offs follow their note-ons.
    thru_held: [Option<(u8, Output, u8)>; 16],
    /// Last non-note message passed through ((kind, data1, data2), time): its echo is dropped.
    thru_last: ((u8, u8, u8), u64),
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
            preview_us: 0,
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
            ext_ignore: false,
            rt_sent_us: core::cell::Cell::new([0; 4]),
            taps: [0; TAP_HISTORY],
            record_armed: false,
            last_step_us: 0,
            last_step_len_us: 125_000,
            rec_held: [None; 8],
            skip_once: [None; TRACKS],
            rec_step: None,
            hold_notes_set: false,
            thru_held: [None; 16],
            thru_last: ((0, 0, 0), 0),
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
        self.note_rt_sent(byte);
        if self.ext_active {
            driver::send_midi(MidiPort::Din, &[byte]);
            driver::send_midi(MidiPort::Din2, &[byte]);
        } else {
            send_all(&[byte]);
        }
    }

    fn rt_index(byte: u8) -> Option<usize> {
        match byte {
            0xF8 => Some(0),
            0xFA => Some(1),
            0xFB => Some(2),
            0xFC => Some(3),
            _ => None,
        }
    }

    fn note_rt_sent(&self, byte: u8) {
        if let Some(i) = Self::rt_index(byte) {
            let mut t = self.rt_sent_us.get();
            t[i] = now_us();
            self.rt_sent_us.set(t);
        }
    }

    /// MIDI real-time messages from the USB MIDI port or DIN In (routed here by the app host whatever app is active).
    /// A synth with MIDI Thru on DIN In echoes our own clock straight back; following that would make the clock chase
    /// itself and run away. So a byte arriving within RT_ECHO_US of us sending the same byte is ignored. (Our clock
    /// pulses are >= 8 ms apart even at 300 BPM, and a real external clock that is ignored once soon takes over.)
    pub fn on_realtime(&mut self, status: u8) {
        const RT_ECHO_US: u64 = 6_000;
        if self.ext_ignore {
            return;
        }
        let now = now_us();
        if let Some(i) = Self::rt_index(status) {
            if now.saturating_sub(self.rt_sent_us.get()[i]) < RT_ECHO_US {
                return;
            }
        }
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

    /// Shift + Play: follow / ignore an external MIDI clock. Turning it off while following carries on at once with
    /// the internal clock at the current BPM.
    fn toggle_ext_ignore(&mut self) {
        self.ext_ignore = !self.ext_ignore;
        if self.ext_ignore && self.ext_active {
            self.ext_active = false;
            self.next_clock_us = now_us();
        }
        self.dirty = true;
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
        self.rec_held = [None; 8];
        self.skip_once = [None; TRACKS];
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
            self.note_rt_sent(0xF8);
            send_all(&[0xF8]);
            self.pulse(pulse_time, interval);
            self.next_clock_us += interval;
        }
    }

    fn play_step(&mut self, time: u64, step_us: u64) {
        self.last_step_us = time;
        self.last_step_len_us = step_us;
        for t in 0..TRACKS {
            let (pos, step, muted, output, channel, transpose) = {
                let tr = &self.tracks[t];
                let pos = tr.position as usize % STEPS;
                (pos, tr.pat().steps[pos], tr.muted, tr.output, tr.channel, tr.transpose)
            };
            let skip = self.skip_once[t] == Some(pos as u8);
            if skip {
                self.skip_once[t] = None;
            }
            if step.on && !muted && !skip && (step.prob >= 100 || self.random_percent() <= step.prob) {
                let gate_us = (step_us * step.gate.clamp(1, 8) as u64 / 8).saturating_sub(200).max(1000);
                for n in step.notes() {
                    let note = (n as i16 + transpose as i16).clamp(0, 127) as u8;
                    self.note_off_now(output, channel, note);
                    send(output, &[0x90 | channel, note, step.velocity]);
                    self.schedule_off(NoteOff { due_us: time + gate_us, sent_us: now_us(), output, channel, note });
                }
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

    // ---------------- live recording ----------------

    /// Writes a played note into the selected track at the nearest step (if recording).
    fn record_note_on(&mut self, note: u8, velocity: u8) {
        if !(self.record_armed && self.playing) {
            return;
        }
        let t = self.selected;
        let now = now_us();
        let tr = &mut self.tracks[t];
        let len = tr.pat().length.clamp(1, STEPS as u8);
        let Some(last) = tr.playing_step else { return };
        // Early in the current step: it belongs to the step that just played; late: to the next one.
        let late = now.saturating_sub(self.last_step_us) >= self.last_step_len_us / 2;
        let step = if late { (last + 1) % len } else { last };
        let velocity = velocity.clamp(1, 127);
        let chord = matches!(self.rec_step, Some((rt, rs, at)) if rt == t as u8 && rs == step
            && now.saturating_sub(at) < self.last_step_len_us.max(1));
        let s = &mut tr.pat_mut().steps[step as usize];
        if chord && s.on {
            // Another note of a chord played on this step: add it.
            s.add(note);
            s.velocity = s.velocity.max(velocity);
        } else {
            *s = Step { velocity, ..Step::single(note) };
        }
        self.rec_step = Some((t as u8, step, now));
        if late {
            self.skip_once[t] = Some(step);
        }
        if let Some(slot) = self.rec_held.iter_mut().find(|s| s.is_none()) {
            *slot = Some((note, t as u8, step, now));
        }
        self.dirty = true;
    }

    /// Note released: its held length becomes the gate (in eighths of a step, 1-8).
    fn record_note_off(&mut self, note: u8) {
        let now = now_us();
        let step_len = self.last_step_len_us.max(1);
        for slot in self.rec_held.iter_mut() {
            if let Some((n, t, step, start)) = *slot {
                if n == note {
                    *slot = None;
                    let gate = ((now.saturating_sub(start) * 8 + step_len / 2) / step_len).clamp(1, 8) as u8;
                    let s = &mut self.tracks[t as usize].pat_mut().steps[step as usize];
                    if s.on && s.has(note) {
                        s.gate = gate;
                    }
                }
            }
        }
    }

    // ---------------- MIDI thru ----------------
    //
    // A keyboard on DIN In / the USB MIDI port plays the selected track's sound: its notes and controllers are
    // re-sent on that track's output and channel (whatever channel the keyboard sends on). Keyboards with MIDI Thru
    // echo what we send back to us, so: a note-on for a note thru is already holding, or for a note the sequencer sent
    // on that channel in the last 30 ms, is an echo and is ignored (also not recorded); a note-off for a note thru is
    // not holding is ignored; a controller identical to the last one forwarded within 100 ms is ignored.

    /// Is this incoming note our own output coming back (sequencer playback or a pad preview)? An echo arrives within
    /// a few ms of what we sent, so only a note sent in the last ECHO_WINDOW_US counts; the same note played on purpose
    /// later (e.g. doubling a part that is playing) still gets through.
    fn is_echo(&self, channel: u8, note: u8) -> bool {
        const ECHO_WINDOW_US: u64 = 30_000;
        let now = now_us();
        let recent = |t: u64| now.saturating_sub(t) < ECHO_WINDOW_US;
        self.offs.iter().flatten().any(|o| o.channel == channel && o.note == note && recent(o.sent_us))
            || matches!(self.preview, Some((n, _, c)) if n == note && c == channel && recent(self.preview_us))
    }

    /// Returns false when the note was ignored as an echo.
    fn thru_note_on(&mut self, in_channel: u8, note: u8, velocity: u8) -> bool {
        if self.thru_held.iter().flatten().any(|h| h.0 == note) || self.is_echo(in_channel, note) {
            return false;
        }
        let tr = &self.tracks[self.selected];
        let (output, channel) = (tr.output, tr.channel);
        send(output, &[0x90 | channel, note, velocity]);
        match self.thru_held.iter_mut().find(|h| h.is_none()) {
            Some(slot) => *slot = Some((note, output, channel)),
            None => send(output, &[0x80 | channel, note, 0]), // more keys held than we track: cut it short
        }
        true
    }

    /// Note-off goes where its note-on went, even if another track was selected meanwhile. False = not ours.
    fn thru_note_off(&mut self, note: u8) -> bool {
        for slot in self.thru_held.iter_mut() {
            if let Some((n, output, channel)) = *slot {
                if n == note {
                    *slot = None;
                    send(output, &[0x80 | channel, note, 0]);
                    return true;
                }
            }
        }
        false
    }

    /// Aftertouch, controllers, program change, channel pressure, pitch bend: to the selected track's channel.
    fn thru_other(&mut self, event: MidiEvent) {
        let now = now_us();
        let key = (event.status & 0xF0, event.data1, event.data2);
        if self.thru_last.0 == key && now.saturating_sub(self.thru_last.1) < 100_000 {
            return; // our own message echoed back
        }
        self.thru_last = (key, now);
        let tr = &self.tracks[self.selected];
        let status = key.0 | tr.channel;
        if matches!(key.0, 0xC0 | 0xD0) {
            send(tr.output, &[status, event.data1]);
        } else {
            send(tr.output, &[status, event.data1, event.data2]);
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
            self.hold_notes_set = false;
            if !s.on {
                *s = Step::single(note);
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
                // First note pad in this hold sets the step's note; further ones toggle notes in its chord.
                if !self.hold_notes_set {
                    s.notes = Step::single(note).notes;
                    self.hold_notes_set = true;
                } else if s.has(note) {
                    s.remove(note);
                } else {
                    s.add(note);
                }
                s.on = true;
                self.step_edited = true;
            }
            self.tracks[sel].note = note;
            let (output, channel) = (self.tracks[sel].output, self.tracks[sel].channel);
            if let Some((n, o, c)) = self.preview.take() {
                send(o, &[0x80 | c, n, 0]);
            }
            send(output, &[0x90 | channel, note, value.max(1).min(127)]);
            self.preview = Some((note, output, channel));
            self.preview_us = now_us();
            if self.held_step.is_none() {
                self.record_note_on(note, value.max(1));
            }
        } else {
            if let Some((n, o, c)) = self.preview {
                if n == note {
                    send(o, &[0x80 | c, n, 0]);
                    self.preview = None;
                }
            }
            self.record_note_off(note);
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

    /// Tap tempo: the average gap between the last few taps sets the tempo (internal clock only).
    fn tap(&mut self) {
        let now = now_us();
        let last = self.taps[TAP_HISTORY - 1];
        if last == 0 || now.saturating_sub(last) > TAP_RESET_US {
            self.taps = [0; TAP_HISTORY];
        }
        self.taps.rotate_left(1);
        self.taps[TAP_HISTORY - 1] = now;
        let first = self.taps.iter().position(|&t| t != 0).unwrap_or(TAP_HISTORY - 1);
        let gaps = (TAP_HISTORY - 1 - first) as u64;
        if gaps >= 1 && !self.ext_active {
            let interval = (now - self.taps[first]) / gaps;
            self.bpm = (60_000_000 / interval.max(1)).clamp(30, 300) as u16;
        }
        self.dirty = true;
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

    /// Streams the project body (everything before the CRC) to `sink`, in the current (version-4) layout.
    fn emit(&self, sink: &mut dyn FnMut(&[u8])) {
        sink(&project::header(self.bpm, self.swing, if self.ext_ignore { project::FLAG_IGNORE_CLOCK } else { 0 }));
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
                    let o = 1 + i * project::STEP_BYTES;
                    b[o] = ((st.on as u8) << 7) | (st.notes[0] & 0x7f);
                    b[o + 1..o + MAX_NOTES].copy_from_slice(&st.notes[1..]);
                    b[o + MAX_NOTES] = st.velocity;
                    b[o + MAX_NOTES + 1] = st.gate;
                    b[o + MAX_NOTES + 2] = st.prob;
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
        self.ext_ignore = rec[8] & project::FLAG_IGNORE_CLOCK != 0;
        if self.ext_ignore {
            self.ext_active = false;
        }
        // Notes stored per step: 1 in version 2, 4 in version 3, 8 in version 4.
        let (step_bytes, pattern_bytes, track_bytes, stored_notes) = project::layout(rec[4]);
        for (t, tr) in self.tracks.iter_mut().enumerate() {
            let base = project::HEADER + t * track_bytes;
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
                let o = base + project::TRACK_SETTINGS + p * pattern_bytes;
                let b = &rec[o..o + pattern_bytes];
                pat.length = b[0].clamp(1, STEPS as u8);
                for (i, st) in pat.steps.iter_mut().enumerate() {
                    let s = &b[1 + i * step_bytes..1 + (i + 1) * step_bytes];
                    st.on = s[0] & 0x80 != 0;
                    st.notes = Step::chord(s[0] & 0x7f);
                    for (k, &n) in s[1..stored_notes].iter().enumerate() {
                        if n <= 127 {
                            st.notes[1 + k] = n;
                        }
                    }
                    let rest = &s[stored_notes..];
                    st.velocity = rest[0].clamp(1, 127);
                    st.gate = rest[1].clamp(1, 8);
                    st.prob = rest[2].clamp(1, 100);
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
        self.ext_ignore = false;
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
                st.notes = Step::single(b[6 + i * 2] & 0x7f).notes;
                st.velocity = b[7 + i * 2].clamp(1, 127);
            }
        }
    }

    fn scan_slots(&mut self) {
        // Headers only: reading and checking every full record took noticeably long on entry.
        for (s, flag) in self.slot_saved.iter_mut().enumerate() {
            *flag = project::slot_used(s);
        }
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
            // Held lit step: all its chord notes; otherwise the note new steps get.
            let shown = match self.held_step {
                Some(st) if pat.steps[st as usize].on => pat.steps[st as usize].notes,
                _ => Step::single(track.note).notes,
            };
            for pad in 0..32 {
                if swing_view {
                    // Shift held: the note area shows the tempo.
                    led::set(note_pad_index(pad), bpm_pixel(self.bpm, pad % 8, 3 - pad / 8));
                    continue;
                }
                let note = track.base as usize + pad;
                let c = if note > 127 {
                    0
                } else if shown.contains(&(note as u8)) {
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
        if self.shift {
            // Sends/Tap: white for a moment after a tap, amber otherwise.
            let tapped = now_us().saturating_sub(self.taps[TAP_HISTORY - 1]) < 120_000;
            led::set(BTN_TAP, if tapped { 0xFFFFFF } else { 0x603000 });
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
        let play = if self.shift && held_step.is_none() {
            // Shift held: Play shows the clock-follow setting (cyan = follows external clock, orange = ignores it).
            if self.ext_ignore { 0xFF6000 } else { 0x00C0C0 }
        } else if self.playing {
            if beat { on } else { off }
        } else if self.ext_active {
            0x103030
        } else {
            0x101010
        };
        led::set(BTN_PLAY, play);
        led::set(BTN_SEQUENCER, 0xFFFFFF);
        led::set(BTN_NOTE_MODE, 0x202020);
        led::set(BTN_UP, 0x202020);
        led::set(BTN_DOWN, 0x202020);
        led::set(BTN_CLEAR, if self.clear { 0xFF4040 } else { 0x200808 });
        led::set(BTN_DUPLICATE, if self.duplicate { 0xFFFFFF } else { 0x202020 });
        led::set(BTN_FIXED_LENGTH, if self.fixed_length { 0xFFFFFF } else { 0x202020 });
        led::set(BTN_RECORD, if self.record_armed { if self.playing { 0xFF0000 } else { 0xA00000 } } else { 0x200000 });
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
        } else if i == BTN_TAP && self.shift {
            if event.pressed {
                self.tap();
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
                BTN_RECORD if event.pressed => {
                    self.record_armed = !self.record_armed;
                    self.dirty = true;
                }
                BTN_PLAY if event.pressed && self.shift => self.toggle_ext_ignore(),
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

    fn on_midi(&mut self, event: MidiEvent) {
        if !matches!(event.port, MidiPort::Midi | MidiPort::Din) {
            return;
        }
        let (kind, channel) = (event.status & 0xF0, event.status & 0x0F);
        let note = event.data1 & 0x7F;
        match kind {
            0x90 if event.data2 > 0 => {
                if self.thru_note_on(channel, note, event.data2) {
                    self.record_note_on(note, event.data2);
                }
            }
            0x80 | 0x90 => {
                if self.thru_note_off(note) {
                    self.record_note_off(note);
                }
            }
            0xA0..=0xE0 => self.thru_other(event),
            _ => {}
        }
    }

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
