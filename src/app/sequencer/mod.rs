// SPDX-License-Identifier: GPL-3.0-only
//
// 8-track step sequencer.
//
// Layout (Programmer-mode XY indices):
//   Grid rows 8..5   32 steps of the selected track's current pattern (row 8 = steps 1-8): steps 1-32, or 33-64
//                    after ▶ (91 / 92 = ◀ / ▶ switch the page; the other page's arrow is green while the playhead is
//                    on it). Patterns have up to 64 steps. Tap toggles a step.
//                    Hold a step + tap a note pad = set that step's note; tap more pads in the same hold to build a
//                    chord (up to 8 notes; tap a lit one again to remove it). Holding a step shows all its notes.
//   Grid rows 4..1   32 chromatic notes from the track's base note (row 1 = lowest). Tap = choose the note
//                    new steps get, and preview it. Up / Down (80 / 70) move the note range by an octave.
//   101-108          track select (the row under the grid).
//   Mute (2) / Solo (3)  arm a mode (the button flashes; press it again to disarm) in which the track buttons
//                    mute / solo tracks at once. Shift + Mute / Solo: queued mode (flashes with white) - the change
//                    waits until that track's pattern next starts from its first step (at once when stopped); the
//                    track button flashes white meanwhile, and tapping it again cancels. With any track soloed only
//                    soloed tracks play (a muted one stays silent). Solo is saved with the project.
//   Hold a track button: steps 1-16 show/set its MIDI channel; note row 1 pads 1-4 set its output
//                    (USB, DIN 1, DIN 2, DIN 1+2; new tracks start on DIN 1, channel = track number).
//   Hold a step:     right-hand column = velocity bar (top = 127 ... bottom = 16), bottom row = gate length in
//                    eighths of a step (1 = shortest, 8 = whole step; a longer recorded note shows all 8); a lit step tapped and released without an edit
//                    turns off. Step brightness follows velocity.
//   Patterns (89)    tap: latch the pattern view open (white; tap again to close); or hold it, use the pattern
//                    pads and let go (closes on release). Open: the top two grid rows show the selected track's 16 patterns (1-8 on the
//                    top row, 9-16 below). Each track has its own patterns and plays and chains them on its own:
//                    current = white, chained = orange, has notes = dim orange, queued = cyan. Tap a pattern to switch
//                    the selected track to it (at the end of its playing pattern, or at once when stopped; an empty
//                    pattern takes the length of the current one); hold one and tap another to chain the range.
//                    + Duplicate (50) held: copy the current pattern there. + Clear held: clear that pattern.
//                    With Shift held, a pattern pad (switch, chain, copy, clear) acts on every track at once.
//   Hold Shift:      step pads show swing (pad 1 = 50 % straight ... pad 26 = 75 %); tap one to set it.
//   Hold a step + Shift: right-hand column = probability bar (top = 100 % ... bottom = 12 %).
//   Mutation (39)    tap: shuffle the selected track's part of the current pattern (about a quarter of its lit steps
//                    move to another step or swap notes with another lit step; the button turns purple). Long press
//                    (0.6 s): back to the part as it was before the first tap (the button flashes white). The
//                    original is kept in memory only, per track, for the pattern last mutated.
//   Probability (49) tap: probability page (button cyan; tap again to close). Top two rows = 16 levels, left to
//                    right: 100 % (every time), 94, 88 ... 12, 6 % (hardly ever). Rows 3-6 = the selected track's
//                    steps (◀ / ▶ = 1-32 / 33-64), lit by how likely they are to play: tap a lit step to select it
//                    (white; its level lights white), then tap a level to set it. ◀ / ▶ move 8 steps at a time (steps 1-8 ... 57-64).
//   Pattern Settings (69) tap: page for the selected track's current pattern (button orange; tap again to close):
//                    tap a step pad to make it the last step (◀ / ▶ = steps 1-32 / 33-64); the bottom note row sets
//                    the pattern's step rate (same order and colours as below). Each pattern has its own rate; when a
//                    chain moves to a pattern at another rate, it starts where the last step of the one before ends.
//   Hold a track button: note rows 2-4 = transpose (-12 ... +11 semitones; the dim pad is 0); steps 17-24 = step
//                    rate of all the track's patterns, slowest left: 1/1, 1/4, 1/4T, 1/8, 1/8T, 1/16 (default),
//                    1/16T, 1/32 (green = straight, purple = triplet). Swing only applies at 1/16; other rates play
//                    straight. A new (empty) pattern takes the length and rate of the one before.
//   Play (20)        start / stop. With Record armed and the count-in on (Setup page, pad 21), Play first counts
//                    one bar in: the grid flashes red on 4 beats at the current tempo, then playback and recording
//                    start (not while following an external clock; Play again during the count-in cancels it).        Shift (90) + Up / Down: tempo +1 / -1 BPM (+10 / -10 with Fixed Length held).
//   Shift held:      the note area shows the tempo (orange bar = hundreds, then tens and units in white), and the
//                    Sends button (its printed Shift label is "Tap") is tap tempo: tap it in time; the average of the
//                    last few taps sets the BPM (a 2 s pause starts over; ignored while following an external clock).
//   Fixed Length (30) held + step pad: set the pattern length to that step (33-64 on page 2).
//   Duplicate (50) held: tap a lit step to copy it (lights cyan), then tap steps to paste it (notes, length, velocity,
//                    probability); works across both pages.
//   Clear (60) held + step pad: clear the step; + track button: clear the track's current pattern (its length stays).
//   Note (94)        back to the note/performance app; playback continues there (Sequencer button lit green).
//   Record (10)      tap: arm / disarm live recording (red). Record Arm (bottom row, first button) does the same and
//                    lights with it. While armed and playing, notes played on the note pads or
//                    arriving on the USB MIDI port / DIN In (e.g. a keyboard) are written into the selected track's
//                    current pattern at the nearest step, with their velocity and held length as the gate (a note held
//                    over several steps keeps sounding for as long as it was held). A note recorded just ahead of a step is not replayed on that pass (you already heard it).
//                    Notes played together land on the same step as a chord (up to 8; loudest velocity kept).
//                    Armed while stopped = step recording: the cursor (white) starts on step 1; each note played goes on
//                    the cursor step (notes held together = a chord) and the cursor moves on when they are released.
//                    Tap a step to move the cursor (leaving the steps between as they were). After the pattern's last
//                    step Record disarms.
//   MIDI thru        a keyboard on the USB MIDI port / DIN In plays the selected track: notes, aftertouch, controllers
//                    and pitch bend are re-sent on that track's output and channel (program change and bank select are
//                    dropped). Echoes from a
//                    keyboard with MIDI Thru are recognised and dropped (no loops, no double recording).
//   Projects (98)    tap: save the current project. Hold: the top four grid rows show the 32 projects (white =
//                    current, dim = saved, off = empty); tap one to switch to it - the current project is saved first,
//                    and an empty one opens as a new, blank project. The Projects LED is green when everything is
//                    saved, amber with unsaved changes. The current project is opened when the sequencer first opens.
//                    Projects store only the steps in use (up to 30 KB each, about 2,500 used steps).
//
// Timing: a 24 PPQN clock derived from the system timer (not from tick counting) - the default; incoming clock /
// start / stop are ignored. Shift + Play switches to following an external MIDI clock (USB MIDI port / DIN In: FA
// start, FB continue, FC stop, F8 pulses; clock passed on to both DIN outs; back to internal 500 ms after the last
// pulse) and back. Play is green on the internal clock, orange while following an external one; while Shift is held
// it shows the setting (green = internal, orange = external). Saved with the project. One step = a 16th = 6 pulses.
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
/// Right-hand column, 5th from the top (printed "Probability"): opens the probability page.
const BTN_PROBABILITY: u8 = 49;
/// Right-hand column: Pattern Settings (latched page: the selected track's pattern length and step rate).
const BTN_PATTERN_SETTINGS: u8 = 69;
/// Right-hand column, 6th from the top (printed "Mutation"): tap = shuffle the selected track's part, long press =
/// back to how it was before the first shuffle.
const BTN_MUTATE: u8 = 39;
const MUTATE_LONG_US: u64 = 600_000;
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
/// Probability page: 16 levels on the top two rows (level 1 = every time ... level 16 = hardly ever).
const PROB_LEVELS16: [u8; 16] = [100, 94, 88, 81, 75, 69, 62, 56, 50, 44, 38, 31, 25, 19, 12, 6];

/// Probability page: grid index of level `lv` (0-15) on the top two rows, and of step pad `pad` (0-31) on rows 6-3.
fn prob_level_index(lv: usize) -> u8 {
    ((8 - lv / 8) * 10 + lv % 8 + 1) as u8
}

fn prob_step_index(pad: usize) -> u8 {
    ((6 - pad / 8) * 10 + pad % 8 + 1) as u8
}

/// The probability page level (0-15) nearest to a step's probability.
fn prob_level(prob: u8) -> usize {
    (0..16).min_by_key(|&i| (PROB_LEVELS16[i] as i16 - prob as i16).abs()).unwrap_or(0)
}
/// Transpose pads: note pad index - TRANSPOSE_ZERO_PAD = semitones (pads 8-31 -> -12 ... +11).
const TRANSPOSE_ZERO_PAD: i8 = 20;
/// Per-track step rates, in half clock pulses (24 PPQN) per step, slowest first (step pads 17-24 while a track
/// button is held): 1/1, 1/4, 1/4 triplet, 1/8, 1/8 triplet, 1/16, 1/16 triplet, 1/32. Triplets are the ones not
/// divisible by 3.
const RATE_HALF_PULSES: [u32; 8] = [192, 48, 32, 24, 16, 12, 8, 6];
const DEFAULT_RATE: u8 = 5;
/// Longest gate, in eighths of a step (recorded notes held over several steps).
const MAX_GATE: u8 = 255;
const RATE_PAD: usize = 16;
/// The step area shows 32 steps at a time: page 1 = steps 1-32, page 2 = steps 33-64 (◀ / ▶ switch).
const PAGE_STEPS: usize = 32;
const BTN_LEFT: u8 = 91;
/// Bottom row: Mute (2nd) and Solo (3rd) arm a mode in which the track buttons mute / solo tracks.
/// Bottom row, first button (printed "Record Arm"): arms / disarms recording, like Record (10).
const BTN_REC_ARM: u8 = 1;
const BTN_MUTE: u8 = 2;
const BTN_SOLO: u8 = 3;

/// What the track buttons do while Mute or Solo is armed. Queued = Shift + Mute / Solo: the change waits until the
/// track's pattern next starts from its first step.
#[derive(Copy, Clone, PartialEq)]
enum Arm {
    Off,
    Mute { queued: bool },
    Solo { queued: bool },
}
const BTN_RIGHT: u8 = 92;

/// Step rate from track settings byte 10 (half pulses per step), or from byte 9 of the v0.2 test build (rate index
/// + 1 in the old order 1/32, 1/16T, 1/16, 1/8T, 1/8, 1/4T, 1/4, 1/2; 1/2 becomes 1/1).
fn rate_from_saved(half_pulses: u8, old: u8) -> u8 {
    const OLD_HALF_PULSES: [u8; 8] = [6, 8, 12, 16, 24, 32, 48, 96];
    let hp = match (half_pulses, old) {
        (0, o @ 1..=8) => OLD_HALF_PULSES[o as usize - 1],
        (h, _) => h,
    };
    match RATE_HALF_PULSES.iter().position(|&r| r == hp as u32) {
        Some(i) => i as u8,
        None if hp < 6 => (RATE_HALF_PULSES.len() - 1) as u8, // 1/64 of an earlier test build
        None if hp == 96 => 0,
        None => DEFAULT_RATE,
    }
}

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
    /// Step rate: index into RATE_HALF_PULSES (DEFAULT_RATE = 16ths). Each pattern has its own.
    rate: u8,
}

impl Pattern {
    const EMPTY: Pattern = Pattern { steps: [Step::OFF; STEPS], length: 16, rate: DEFAULT_RATE };

    /// Clears every step but keeps the pattern's length.
    fn clear(&mut self) {
        self.steps = [Step::OFF; STEPS];
    }

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
    /// When any track is soloed, only soloed tracks play (a muted track stays silent).
    solo: bool,
    /// Queued mute / solo toggles (Shift + Mute / Solo), applied when the pattern next starts from its first step.
    pending_mute: bool,
    pending_solo: bool,
    note: u8, // note given to newly enabled steps
    base: u8, // lowest note of the note area
    transpose: i8,
    position: u8, // next step to play
    playing_step: Option<u8>,
    /// After moving to a pattern with a different step rate: the clock pulse its first step is due at (the end of
    /// the last step of the pattern before), so the new rate does not start early.
    resume_pulse: u32,
    /// Time and length of the track's last played step (for quantising recorded notes).
    last_step_us: u64,
    step_len_us: u64,
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
            solo: false,
            pending_mute: false,
            pending_solo: false,
            note: 60,
            base: 48,
            transpose: 0,
            position: 0,
            playing_step: None,
            resume_pulse: 0,
            last_step_us: 0,
            step_len_us: 125_000,
        }
    }

    fn pat(&self) -> &Pattern {
        &self.patterns[self.pattern as usize % PATTERNS]
    }

    fn pat_mut(&mut self) -> &mut Pattern {
        &mut self.patterns[self.pattern as usize % PATTERNS]
    }

    /// Applies queued mute / solo changes (at the start of the pattern, or at once when stopped).
    fn apply_pending(&mut self) {
        if self.pending_mute {
            self.muted = !self.muted;
        }
        if self.pending_solo {
            self.solo = !self.solo;
        }
        self.pending_mute = false;
        self.pending_solo = false;
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
    /// A step of a non-16th rate due between two clock pulses: (due, step length, rate).
    pending_half: Option<(u64, u64, u8)>,
    /// Step page shown: 0 = steps 1-32, 1 = steps 33-64.
    page: u8,
    /// Mute / Solo mode for the track buttons, and the flash phase of the armed button (redrawn when it changes).
    arm: Arm,
    blink: bool,
    /// Duplicate held: the step being copied to every step tapped next.
    copy_src: Option<u8>,
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
    /// Patterns button physically down, and whether a pattern pad was used during this press: a plain tap latches
    /// the pattern view open (tap again to close); holding it and using a pattern pad closes it on release.
    patterns_down: bool,
    patterns_used: bool,
    /// Probability page (Probability button, latched): top two rows = 16 levels (100 % ... 6 %), rows 3-6 = the 32
    /// steps of the page shown; prob_sel = the step whose level the top rows show and set.
    prob_view: bool,
    prob_sel: Option<u8>,
    /// Pattern Settings page (latched): step pads set the length of the selected track's pattern (◀ / ▶ = steps
    /// 1-32 / 33-64), the bottom row its step rate.
    pset_view: bool,
    /// Per track: the pattern (index, contents) as it was before the first Mutate tap; a long press restores it.
    /// Kept in memory only (not saved); a new snapshot is taken when the track mutates a different pattern.
    mutate_orig: [Option<(u8, Pattern)>; TRACKS],
    /// When Mutate was pressed (0 = not held), and whether this press has already restored (long press).
    mutate_down_us: u64,
    mutate_long_done: bool,
    /// Mutate lights white until then after a restore.
    mutate_flash_until: u64,
    /// Count-in (Record armed + Play, when switched on in Setup): when it started (0 = none) and the last drawn
    /// phase (beat * 2 + lit), so the grid redraws only when the flash changes.
    count_in_start_us: u64,
    count_in_phase: u8,
    /// Pattern pad held in the patterns view (a second one tapped meanwhile chains the range).
    pattern_hold: Option<u8>,
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
    /// Ignore incoming MIDI clock / start / stop: the default (internal clock). Shift + Play switches to following an
    /// external clock; saved with the project.
    ext_ignore: bool,
    /// When we last sent F8 / FA / FB / FC: the same byte arriving within RT_ECHO_US of that is our own echo.
    rt_sent_us: core::cell::Cell<[u64; 4]>,
    /// Tap tempo (Shift + Sends/Tap): times of the recent taps, newest last; 0 = none.
    taps: [u64; TAP_HISTORY],
    /// Live recording armed (Record button); records while playing.
    record_armed: bool,
    /// Notes being recorded, waiting for their note-off: (note, track, step, start time).
    rec_held: [Option<(u8, u8, u8, u64)>; 8],
    /// Time of the last clock pulse and the interval it ran at (to measure recorded note lengths in pulses).
    last_pulse_us: u64,
    pulse_interval_us: u64,
    /// Per track: a step just recorded ahead of the playhead, not to be replayed on this pass.
    skip_once: [Option<u8>; TRACKS],
    /// Last recorded step (track, step, time): notes landing on it within a step's time join it as a chord.
    rec_step: Option<(u8, u8, u64)>,
    /// Step recording (Record armed while stopped): the step the next note goes to, and the notes held on it now
    /// (the cursor moves on when they are all released, so notes held together make a chord).
    step_cursor: u8,
    step_rec_notes: [u8; MAX_NOTES],
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

/// Grid index -> pad number counted from the top-left (row 8 = 0-7, row 7 = 8-15, ...): patterns 0-15 and
/// projects 0-31 are numbered this way.
fn grid_number(index: u8) -> Option<u8> {
    let (row, col) = ((index / 10) as usize, (index % 10) as usize);
    if (1..=8).contains(&row) && (1..=8).contains(&col) { Some(((8 - row) * 8 + col - 1) as u8) } else { None }
}

/// LED index of grid pad `n` counted from the top-left (0-63): patterns 0-15, projects 0-31.
fn top16_index(n: usize) -> u8 {
    ((8 - n / 8) * 10 + n % 8 + 1) as u8
}

fn is_grid(index: u8) -> bool {
    (1..=8).contains(&(index / 10)) && (1..=8).contains(&(index % 10))
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

/// A step as stored: on << 7 | note 1, notes 2-8 (0xFF = none), velocity, gate, probability.
fn step_bytes(st: &Step) -> [u8; project::STEP_BYTES] {
    let mut b = [0u8; project::STEP_BYTES];
    b[0] = ((st.on as u8) << 7) | (st.notes[0] & 0x7f);
    b[1..MAX_NOTES].copy_from_slice(&st.notes[1..]);
    b[MAX_NOTES] = st.velocity;
    b[MAX_NOTES + 1] = st.gate;
    b[MAX_NOTES + 2] = st.prob;
    b
}

/// A stored step with `notes` note bytes (1, 4 or 8), then velocity, gate, probability.
fn step_from_bytes(s: &[u8], notes: usize) -> Step {
    let mut st = Step::OFF;
    st.on = s[0] & 0x80 != 0;
    st.notes = Step::chord(s[0] & 0x7f);
    for (k, &n) in s[1..notes].iter().enumerate() {
        if n <= 127 {
            st.notes[1 + k] = n;
        }
    }
    let rest = &s[notes..];
    st.velocity = rest[0].clamp(1, 127);
    st.gate = rest[1].max(1);
    st.prob = rest[2].clamp(1, 100);
    st
}

/// The Setup page's Clock Out switch: send MIDI clock, start, continue and stop (on by default).
fn clock_out_enabled() -> bool {
    crate::sys::settings::with(|s| s.clock_out != 0)
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
            pending_half: None,
            page: 0,
            arm: Arm::Off,
            blink: false,
            copy_src: None,
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
            patterns_down: false,
            patterns_used: false,
            prob_view: false,
            prob_sel: None,
            pset_view: false,
            mutate_orig: [None; TRACKS],
            mutate_down_us: 0,
            mutate_long_done: false,
            mutate_flash_until: 0,
            count_in_start_us: 0,
            count_in_phase: 0xFF,
            pattern_hold: None,
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
            ext_ignore: true,
            rt_sent_us: core::cell::Cell::new([0; 4]),
            taps: [0; TAP_HISTORY],
            record_armed: false,
            rec_held: [None; 8],
            last_pulse_us: 0,
            pulse_interval_us: 20_833,
            skip_once: [None; TRACKS],
            rec_step: None,
            step_cursor: 0,
            step_rec_notes: [NO_NOTE; MAX_NOTES],
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

    /// Random number below n (n > 0).
    fn random_below(&mut self, n: usize) -> usize {
        self.random_percent();
        (self.rng % n as u32) as usize
    }

    // ---------------- mutate ----------------

    /// Mutate tap: rearranges the selected track's part of the current pattern. About one in four of its lit steps
    /// (at least one) is either moved to another step (swapping places with whatever is there) or swaps its notes
    /// with another lit step, so the part keeps the same hits and notes in a new arrangement. The part as it was
    /// before the first tap is kept for a long press to restore.
    fn mutate(&mut self) {
        let t = self.selected;
        let index = self.tracks[t].pattern;
        let mut pat = *self.tracks[t].pat();
        let len = pat.length.clamp(1, STEPS as u8) as usize;
        let mut lit = [0u8; STEPS];
        let mut n = 0;
        for (i, st) in pat.steps[..len].iter().enumerate() {
            if st.on {
                lit[n] = i as u8;
                n += 1;
            }
        }
        if n == 0 || len < 2 {
            return;
        }
        if !matches!(self.mutate_orig[t], Some((p, _)) if p == index) {
            self.mutate_orig[t] = Some((index, pat));
        }
        for _ in 0..(n / 4).max(1) {
            // Positions of lit steps change as we go: pick a currently lit step each time.
            let on: usize = pat.steps[..len].iter().filter(|st| st.on).count();
            let mut k = self.random_below(on);
            let a = pat.steps[..len].iter().position(|st| {
                if st.on {
                    if k == 0 {
                        return true;
                    }
                    k -= 1;
                }
                false
            });
            let Some(a) = a else { break };
            if on >= 2 && self.random_percent() <= 50 {
                // Swap notes with another lit step (rhythm stays, melody changes).
                let mut k = self.random_below(on - 1);
                let b = pat.steps[..len].iter().enumerate().position(|(i, st)| {
                    if st.on && i != a {
                        if k == 0 {
                            return true;
                        }
                        k -= 1;
                    }
                    false
                });
                if let Some(b) = b {
                    let notes = pat.steps[a].notes;
                    pat.steps[a].notes = pat.steps[b].notes;
                    pat.steps[b].notes = notes;
                }
            } else {
                // Move the step to another position, swapping with what is there.
                let mut b = self.random_below(len - 1);
                if b >= a {
                    b += 1;
                }
                pat.steps.swap(a, b);
            }
        }
        *self.tracks[t].pat_mut() = pat;
        self.dirty = true;
    }

    /// Mutate long press: back to the part as it was before the first tap (if the track is still on that pattern).
    fn mutate_restore(&mut self) {
        let t = self.selected;
        if let Some((index, pat)) = self.mutate_orig[t] {
            if index == self.tracks[t].pattern {
                *self.tracks[t].pat_mut() = pat;
                self.mutate_orig[t] = None;
                self.mutate_flash_until = now_us() + 300_000;
            }
        }
        self.dirty = true;
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
            t.resume_pulse = 0;
            t.apply_pending();
        }
        self.rng ^= now_us() as u32;
        self.clock_count = 0;
        self.next_clock_us = now_us();
        self.playing = true;
        self.send_transport(0xFA);
        self.dirty = true;
    }

    /// Transport/clock bytes: to every output when we are the clock master; when following an external clock only
    /// to the DIN outputs (passing the clock on), never back to USB where it may have come from. Nothing is sent when
    /// Clock Out is switched off in Setup.
    fn send_transport(&self, byte: u8) {
        if !clock_out_enabled() {
            return;
        }
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
        self.pending_half = None;
        self.step_record_reset();
        self.stop_notes();
        for t in self.tracks.iter_mut() {
            t.playing_step = None;
        }
        self.send_transport(0xFC);
        self.dirty = true;
    }

    /// One clock pulse (24 PPQN). Tracks at the default 16th rate: even 16ths play on pulse 0 of their 8th-note pair;
    /// odd 16ths are due at pulse 6, delayed by swing (50 % = straight, 75 % = triplet-ish), timed to the millisecond
    /// rather than to whole pulses. Tracks at other rates play straight.
    fn pulse(&mut self, now: u64, interval: u64) {
        let count = self.clock_count;
        let in_pair = count % 12;
        let pair_us = interval * 12;
        let swing_us = pair_us * (self.swing.clamp(50, 75) as u64 - 50) / 100;
        if in_pair == 0 {
            if let Some((due, len)) = self.pending_step.take() {
                self.play_step(due.min(now), len, DEFAULT_RATE);
            }
            self.play_step(now, pair_us / 2 + swing_us, DEFAULT_RATE);
        } else if in_pair == 6 {
            let len = pair_us / 2 - swing_us;
            if swing_us < 1000 {
                self.play_step(now, len, DEFAULT_RATE);
            } else {
                self.pending_step = Some((now + swing_us, len));
            }
        }
        // Other rates, straight. A step is due every `hp` half-pulses from the start; one landing half way between
        // two pulses (1/64 = 1.5 pulses) is timed half a clock interval after this pulse.
        if let Some((due, len, r)) = self.pending_half.take() {
            self.play_step(due.min(now), len, r);
        }
        let half = 2 * count as u64;
        for (r, &hp) in RATE_HALF_PULSES.iter().enumerate() {
            if r as u8 == DEFAULT_RATE {
                continue;
            }
            let hp = hp as u64;
            let len = interval * hp / 2;
            if half % hp == 0 {
                self.play_step(now, len, r as u8);
            } else if (half + 1) % hp == 0 {
                self.pending_half = Some((now + interval / 2, len, r as u8));
            }
        }
        self.clock_count = count.wrapping_add(1);
        self.last_pulse_us = now;
        self.pulse_interval_us = interval.max(1);
    }

    /// Clock position in 1/256ths of a pulse: pulses so far plus how far we are towards the next one.
    fn clock_pos(&self, now: u64) -> u64 {
        let frac = (now.saturating_sub(self.last_pulse_us) * 256 / self.pulse_interval_us).min(255);
        self.clock_count as u64 * 256 + frac
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
            if clock_out_enabled() {
                self.note_rt_sent(0xF8);
                send_all(&[0xF8]);
            }
            self.pulse(pulse_time, interval);
            self.next_clock_us += interval;
        }
    }

    /// Plays the next step of every track running at `rate`.
    fn play_step(&mut self, time: u64, step_us: u64, rate: u8) {
        let any_solo = self.tracks.iter().any(|t| t.solo);
        for t in 0..TRACKS {
            if self.tracks[t].pat().rate != rate
                || (self.tracks[t].resume_pulse.wrapping_sub(self.clock_count) as i32) > 0
            {
                continue;
            }
            let (pos, step, muted, output, channel, transpose) = {
                let tr = &mut self.tracks[t];
                tr.last_step_us = time;
                tr.step_len_us = step_us;
                let pos = tr.position as usize % STEPS;
                (pos, tr.pat().steps[pos], tr.muted || (any_solo && !tr.solo), tr.output, tr.channel, tr.transpose)
            };
            let skip = self.skip_once[t] == Some(pos as u8);
            if skip {
                self.skip_once[t] = None;
            }
            if step.on && !muted && !skip && (step.prob >= 100 || self.random_percent() <= step.prob) {
                let gate_us = (step_us * step.gate.max(1) as u64 / 8).saturating_sub(200).max(1000);
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
                tr.apply_pending();
                if tr.pat().rate != rate {
                    // The next pattern runs at another rate: it starts where this step ends (the next point on this
                    // rate's grid), not at the new rate's next point, which may come sooner.
                    let p = RATE_HALF_PULSES[rate as usize] / 2;
                    tr.resume_pulse = (self.clock_count / p + 1) * p;
                }
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
        if self.count_in_start_us != 0 {
            return;
        }
        if self.record_armed && !self.playing {
            self.step_record_note_on(note, velocity);
            return;
        }
        if !(self.record_armed && self.playing) {
            return;
        }
        let t = self.selected;
        let now = now_us();
        let tr = &mut self.tracks[t];
        let len = tr.pat().length.clamp(1, STEPS as u8);
        let Some(last) = tr.playing_step else { return };
        // Early in the current step: it belongs to the step that just played; late: to the next one.
        let step_len = tr.step_len_us.max(1);
        let late = now.saturating_sub(tr.last_step_us) >= step_len / 2;
        let step = if late { (last + 1) % len } else { last };
        let velocity = velocity.clamp(1, 127);
        let chord = matches!(self.rec_step, Some((rt, rs, at)) if rt == t as u8 && rs == step
            && now.saturating_sub(at) < step_len);
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
        let pos = self.clock_pos(now);
        if let Some(slot) = self.rec_held.iter_mut().find(|s| s.is_none()) {
            *slot = Some((note, t as u8, step, pos));
        }
        self.dirty = true;
    }

    /// Note released: its held length becomes the gate, in eighths of a step: 1 up to MAX_GATE (about 32 steps), so a
    /// held note can last over several steps.
    fn record_note_off(&mut self, note: u8) {
        self.step_record_note_off(note);
        let pos = self.clock_pos(now_us());
        for slot in self.rec_held.iter_mut() {
            if let Some((n, t, step, start)) = *slot {
                if n == note {
                    *slot = None;
                    // Held length in clock pulses (exact whatever the tempo or clock source), in eighths of a step.
                    let step_256ths = RATE_HALF_PULSES[self.tracks[t as usize].pat().rate as usize] as u64 * 128;
                    let held = pos.saturating_sub(start) * 8;
                    let gate = ((held + step_256ths / 2) / step_256ths).clamp(1, MAX_GATE as u64) as u8;
                    let s = &mut self.tracks[t as usize].pat_mut().steps[step as usize];
                    if s.on && s.has(note) {
                        s.gate = gate;
                    }
                }
            }
        }
    }

    // ---------------- step recording ----------------

    /// Record armed while stopped: the note goes on the cursor step (replacing what was there; notes played while it
    /// is still held join it as a chord), with its velocity and the default gate.
    fn step_record_note_on(&mut self, note: u8, velocity: u8) {
        let tr = &mut self.tracks[self.selected];
        let len = tr.pat().length.clamp(1, STEPS as u8);
        if self.step_cursor >= len {
            self.step_cursor = 0;
        }
        let velocity = velocity.clamp(1, 127);
        let first = self.step_rec_notes.iter().all(|&n| n == NO_NOTE);
        let s = &mut tr.pat_mut().steps[self.step_cursor as usize];
        if first {
            *s = Step { velocity, ..Step::single(note) };
        } else {
            s.add(note);
            s.velocity = s.velocity.max(velocity);
        }
        if !self.step_rec_notes.contains(&note) {
            if let Some(slot) = self.step_rec_notes.iter_mut().find(|n| **n == NO_NOTE) {
                *slot = note;
            }
        }
        self.dirty = true;
    }

    /// All the notes on the cursor step released: move to the next step. After the pattern's last step, recording
    /// ends (Record disarms) and the cursor goes back to step 1.
    fn step_record_note_off(&mut self, note: u8) {
        let Some(slot) = self.step_rec_notes.iter_mut().find(|n| **n == note) else { return };
        *slot = NO_NOTE;
        if self.step_rec_notes.iter().any(|&n| n != NO_NOTE) {
            return;
        }
        let len = self.tracks[self.selected].pat().length.clamp(1, STEPS as u8);
        self.step_cursor += 1;
        if self.step_cursor >= len {
            self.step_cursor = 0;
            self.record_armed = false;
        }
        self.page = self.step_cursor / PAGE_STEPS as u8;
        self.dirty = true;
    }

    fn step_record_reset(&mut self) {
        self.step_cursor = 0;
        self.step_rec_notes = [NO_NOTE; MAX_NOTES];
    }

    /// Record (10) or Record Arm (bottom row 1): arm / disarm recording.
    fn toggle_record(&mut self) {
        self.record_armed = !self.record_armed;
        self.step_record_reset();
        if self.record_armed && !self.playing {
            self.page = 0;
        }
        self.dirty = true;
    }

    fn step_recording(&self) -> bool {
        self.record_armed && !self.playing && self.count_in_start_us == 0
    }

    // ---------------- count-in ----------------

    /// One bar of four beats at the current tempo.
    fn beat_us(&self) -> u64 {
        60_000_000 / self.bpm.max(1) as u64
    }

    /// Play pressed while stopped: with Record armed, the count-in switched on in Setup and no external clock
    /// running, count one bar in (the grid flashes red on each beat) and start on the next downbeat; otherwise
    /// start at once.
    fn play_pressed(&mut self) {
        if self.playing {
            self.stop();
        } else if self.count_in_start_us != 0 {
            self.count_in_start_us = 0; // Play again during the count-in cancels it
            self.dirty = true;
        } else if self.record_armed && !self.ext_active && crate::sys::settings::get().count_in != 0 {
            self.step_record_reset();
            self.count_in_start_us = now_us().max(1);
            self.count_in_phase = 0xFF;
            self.dirty = true;
        } else {
            self.start();
        }
    }

    fn run_count_in(&mut self, now: u64) {
        if self.count_in_start_us == 0 {
            return;
        }
        let beat = self.beat_us();
        let elapsed = now.saturating_sub(self.count_in_start_us);
        if elapsed >= 4 * beat {
            self.count_in_start_us = 0;
            self.start();
            return;
        }
        // Lit for the first 40 % of each beat.
        let phase = (elapsed / beat) as u8 * 2 + ((elapsed % beat) < beat * 2 / 5) as u8;
        if phase != self.count_in_phase {
            self.count_in_phase = phase;
            self.dirty = true;
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

    /// Aftertouch, controllers, channel pressure, pitch bend: to the selected track's channel (not program change or
    /// bank select).
    fn thru_other(&mut self, event: MidiEvent) {
        // Program changes and bank select (CC 0 / CC 32) are never passed on: changing presets on the keyboard must
        // not change the sound of whatever synth the selected track plays.
        if event.status & 0xF0 == 0xC0 || (event.status & 0xF0 == 0xB0 && matches!(event.data1, 0 | 32)) {
            return;
        }
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

    /// `pad` is 0-31 in the step area; on page 2 it edits steps 33-64 (except for the settings shown on the pads).
    fn on_step_pad(&mut self, pad: usize, pressed: bool) {
        let step = self.page as usize * PAGE_STEPS + pad;
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
            if pad < 16 {
                self.tracks[t].channel = pad as u8;
            } else if (RATE_PAD..RATE_PAD + RATE_HALF_PULSES.len()).contains(&pad) {
                // Holding a track sets the rate of all its patterns (Pattern Settings sets one pattern's).
                let r = (pad - RATE_PAD) as u8;
                for pat in self.tracks[t].patterns.iter_mut() {
                    pat.rate = r;
                }
            }
            self.dirty = true;
            return;
        }
        if self.shift && self.held_step.is_none() {
            if pad < 26 {
                self.swing = 50 + pad as u8;
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
        } else if self.duplicate {
            // Duplicate held: the first lit step tapped is copied; every step tapped after it gets a copy (notes,
            // length, velocity, probability). Releasing Duplicate forgets the copy.
            match self.copy_src {
                None if track.pat().steps[step].on => self.copy_src = Some(step as u8),
                Some(src) if src as usize != step => {
                    let copy = track.pat().steps[src as usize];
                    track.pat_mut().steps[step] = copy;
                }
                _ => {}
            }
        } else if self.record_armed && !self.playing {
            // Step recording: tapping a step moves the cursor there (skipped steps keep what they had).
            if step < track.pat().length as usize {
                self.step_cursor = step as u8;
                self.step_rec_notes = [NO_NOTE; MAX_NOTES];
            }
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

    /// A track button tapped while Mute or Solo is armed: toggle it now, or queue the toggle for the start of the
    /// track's pattern (Shift + Mute / Solo; at once when stopped). Tapping a queued track again cancels it.
    fn mute_solo_tap(&mut self, t: usize) {
        let playing = self.playing;
        let tr = &mut self.tracks[t];
        match self.arm {
            Arm::Mute { queued: true } if playing => tr.pending_mute = !tr.pending_mute,
            Arm::Solo { queued: true } if playing => tr.pending_solo = !tr.pending_solo,
            Arm::Mute { .. } => tr.muted = !tr.muted,
            Arm::Solo { .. } => tr.solo = !tr.solo,
            Arm::Off => {}
        }
        self.dirty = true;
    }

    fn on_track_button(&mut self, t: usize, pressed: bool) {
        if pressed {
            if self.clear {
                self.tracks[t].pat_mut().clear();
            } else {
                self.selected = t;
                self.held_track = Some(t);
            }
        } else if self.held_track == Some(t) {
            self.held_track = None;
        }
        self.dirty = true;
    }

    /// A pattern pad in the patterns view: acts on the selected track (every track with Shift).
    fn on_pattern_pad(&mut self, p: u8, pressed: bool) {
        let p = p.min(PATTERNS as u8 - 1);
        if !pressed {
            if self.pattern_hold == Some(p) {
                self.pattern_hold = None;
            }
            return;
        }
        self.dirty = true;
        // Each track has its own 16 patterns: a pattern pad acts on the selected track, or on every track with Shift.
        let (first, last) = if self.shift { (0, TRACKS) } else { (self.selected, self.selected + 1) };
        if self.duplicate {
            for tr in self.tracks[first..last].iter_mut() {
                tr.patterns[p as usize] = *tr.pat();
            }
            return;
        }
        if self.clear {
            for tr in self.tracks[first..last].iter_mut() {
                tr.patterns[p as usize].clear();
            }
            return;
        }
        let range = match self.pattern_hold {
            Some(first) if first != p => (first.min(p), first.max(p)),
            _ => {
                self.pattern_hold = Some(p);
                (p, p)
            }
        };
        let playing = self.playing;
        for tr in self.tracks[first..last].iter_mut() {
            // A pattern with no notes yet is new: it takes the length and rate of the track's current pattern.
            let (length, rate) = (tr.pat().length, tr.pat().rate);
            for q in range.0..=range.1 {
                let pat = &mut tr.patterns[q as usize];
                if !pat.has_notes() {
                    pat.length = length;
                    pat.rate = rate;
                }
            }
            if playing {
                tr.queued = Some(range);
            } else {
                tr.chain_start = range.0;
                tr.chain_end = range.1;
                tr.pattern = range.0;
                tr.position = 0;
                tr.queued = None;
            }
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
        self.run_count_in(now);
        self.run_clock(now);
        if let Some((due, len)) = self.pending_step {
            if !self.playing {
                self.pending_step = None;
            } else if now >= due {
                self.pending_step = None;
                self.play_step(due, len, DEFAULT_RATE);
            }
        }
        if let Some((due, len, r)) = self.pending_half {
            if !self.playing {
                self.pending_half = None;
            } else if now >= due {
                self.pending_half = None;
                self.play_step(due, len, r);
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

    /// Streams the project body (everything before the CRC) to `sink`, in the version-6 layout (the length field in
    /// the header is left 0; `project::seal6` fills it in).
    fn emit(&self, sink: &mut dyn FnMut(&[u8])) {
        sink(&project::header6(self.bpm, self.swing, if self.ext_ignore { 0 } else { project::FLAG_FOLLOW_CLOCK }));
        for tr in self.tracks.iter() {
            let mut s = [0u8; project::TRACK_SETTINGS];
            s[0] = tr.channel;
            s[1] = Output::ALL.iter().position(|o| *o == tr.output).unwrap_or(0) as u8;
            s[2] = tr.muted as u8;
            s[11] = tr.solo as u8;
            s[3] = tr.note;
            s[4] = tr.base;
            s[5] = tr.pattern;
            s[6] = tr.chain_start;
            s[7] = tr.chain_end;
            s[8] = tr.transpose as u8;
            s[10] = RATE_HALF_PULSES[tr.pat().rate as usize % RATE_HALF_PULSES.len()] as u8; // for older firmware
            sink(&s);
        }
        let empty = step_bytes(&Step::OFF);
        for tr in self.tracks.iter() {
            for pat in tr.patterns.iter() {
                let used = pat.steps.iter().filter(|st| step_bytes(st) != empty).count();
                sink(&[pat.length, used as u8]);
                for (i, st) in pat.steps.iter().enumerate() {
                    let b = step_bytes(st);
                    if b != empty {
                        sink(&[i as u8]);
                        sink(&b);
                    }
                }
            }
        }
        // Extension (older firmware stops reading before it): each pattern's step rate, in half pulses per step.
        sink(&project::RATES_MAGIC);
        for tr in self.tracks.iter() {
            for pat in tr.patterns.iter() {
                sink(&[RATE_HALF_PULSES[pat.rate as usize % RATE_HALF_PULSES.len()] as u8]);
            }
        }
    }

    /// Fingerprint of the song, for the Projects LED (unsaved changes) and to tell whether switching project needs a
    /// save. Runs on every redraw, so the 8,192 steps are folded in directly instead of going through `emit`.
    fn current_crc(&self) -> u32 {
        let mut crc = project::Crc::new();
        crc.update(&project::header6(self.bpm, self.swing, self.ext_ignore as u8));
        for tr in self.tracks.iter() {
            crc.update(&[
                tr.channel,
                tr.output as u8,
                tr.muted as u8,
                tr.solo as u8,
                tr.note,
                tr.base,
                tr.pattern,
                tr.chain_start,
                tr.chain_end,
                tr.transpose as u8,
            ]);
        }
        let mut h = crc.value();
        let mut mix = |w: u32| h = (h ^ w).wrapping_mul(0x0100_0193).rotate_left(5);
        for tr in self.tracks.iter() {
            for pat in tr.patterns.iter() {
                mix(pat.length as u32 | (pat.rate as u32) << 8);
                for st in pat.steps.iter() {
                    let n = &st.notes;
                    mix(u32::from_le_bytes([n[0], n[1], n[2], n[3]]));
                    mix(u32::from_le_bytes([n[4], n[5], n[6], n[7]]));
                    mix(u32::from_le_bytes([st.on as u8, st.velocity, st.gate, st.prob]));
                }
            }
        }
        h
    }

    fn load_header(&mut self, h: &[u8]) {
        self.bpm = u16::from_le_bytes([h[5], h[6]]).clamp(30, 300);
        self.swing = h[7].clamp(50, 75);
        self.ext_ignore = h[8] & project::FLAG_FOLLOW_CLOCK == 0;
        if self.ext_ignore {
            self.ext_active = false;
        }
    }

    fn load_settings(tr: &mut Track, s: &[u8], patterns: usize) {
        tr.channel = s[0] & 0x0f;
        tr.output = Output::ALL[(s[1] as usize).min(3)];
        tr.muted = s[2] != 0;
        tr.solo = s[11] != 0;
        tr.pending_mute = false;
        tr.pending_solo = false;
        tr.note = s[3] & 0x7f;
        tr.base = s[4].min(96);
        tr.pattern = s[5] % patterns as u8;
        tr.chain_start = s[6] % patterns as u8;
        tr.chain_end = (s[7] % patterns as u8).max(tr.chain_start);
        tr.transpose = (s[8] as i8).clamp(-24, 24);
        // Older projects have one rate per track: every pattern starts with it.
        let rate = rate_from_saved(s[10], s[9]);
        for pat in tr.patterns.iter_mut() {
            pat.rate = rate;
        }
        tr.resume_pulse = 0;
        tr.queued = None;
        tr.position = 0;
        tr.playing_step = None;
    }

    /// Loads a version-6 record of `len` bytes (CRC already checked). False if it is malformed.
    fn load_record6(&mut self, rec: &[u8], len: usize) -> bool {
        let body = &rec[..len - 4];
        let settings_end = project::HEADER + TRACKS * project::TRACK_SETTINGS;
        if body.len() < settings_end {
            return false;
        }
        self.load_header(body);
        for (t, tr) in self.tracks.iter_mut().enumerate() {
            let o = project::HEADER + t * project::TRACK_SETTINGS;
            Self::load_settings(tr, &body[o..o + project::TRACK_SETTINGS], PATTERNS);
        }
        let mut pos = settings_end;
        for tr in self.tracks.iter_mut() {
            for pat in tr.patterns.iter_mut() {
                pat.steps = [Step::OFF; STEPS];
                if pos + 2 > body.len() {
                    pat.length = 16;
                    continue;
                }
                pat.length = body[pos].clamp(1, STEPS as u8);
                let n = body[pos + 1] as usize;
                pos += 2;
                for _ in 0..n {
                    if pos + 1 + project::STEP_BYTES > body.len() {
                        break;
                    }
                    let i = body[pos] as usize;
                    if i < STEPS {
                        pat.steps[i] = step_from_bytes(&body[pos + 1..pos + 1 + project::STEP_BYTES], MAX_NOTES);
                    }
                    pos += 1 + project::STEP_BYTES;
                }
            }
        }
        // Per-pattern step rates (absent in projects saved before they existed: the track's rate stays).
        let m = project::RATES_MAGIC.len();
        if body.len() >= pos + m + TRACKS * PATTERNS && body[pos..pos + m] == project::RATES_MAGIC {
            pos += m;
            for tr in self.tracks.iter_mut() {
                for pat in tr.patterns.iter_mut() {
                    pat.rate = rate_from_saved(body[pos], 0);
                    pos += 1;
                }
            }
        }
        true
    }

    /// Loads an old (version 2-5) record: its 8 per-track patterns become patterns 1-8.
    fn load_record(&mut self, rec: &[u8; project::RECORD]) {
        self.load_header(rec);
        // Notes stored per step: 1 in version 2, 4 in version 3, 8 from version 4; 32 steps per pattern before version 5.
        let (step_bytes, pattern_bytes, track_bytes, stored_notes, stored_steps) = project::layout(rec[4]);
        for (t, tr) in self.tracks.iter_mut().enumerate() {
            let base = project::HEADER + t * track_bytes;
            Self::load_settings(tr, &rec[base..base + project::TRACK_SETTINGS], project::OLD_PATTERNS);
            for (p, pat) in tr.patterns.iter_mut().enumerate() {
                if p >= project::OLD_PATTERNS {
                    *pat = Pattern { rate: pat.rate, ..Pattern::EMPTY };
                    continue;
                }
                let o = base + project::TRACK_SETTINGS + p * pattern_bytes;
                let b = &rec[o..o + pattern_bytes];
                pat.length = b[0].clamp(1, STEPS as u8);
                for (i, st) in pat.steps.iter_mut().enumerate() {
                    *st = if i < stored_steps {
                        step_from_bytes(&b[1 + i * step_bytes..1 + (i + 1) * step_bytes], stored_notes)
                    } else {
                        Step::OFF
                    };
                }
            }
        }
    }

    /// Imports a version-1 (single pattern) record into pattern 1 of each track.
    fn load_v1(&mut self, rec: &[u8; project::V1_RECORD]) {
        self.bpm = u16::from_le_bytes([rec[5], rec[6]]).clamp(30, 300);
        self.swing = 50;
        self.ext_ignore = true;
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

    /// A new, empty project.
    fn load_blank(&mut self) {
        for (t, tr) in self.tracks.iter_mut().enumerate() {
            *tr = Track::new(t as u8);
        }
        self.bpm = 120;
        self.swing = 50;
        self.ext_ignore = true;
        self.ext_active = false;
    }

    fn scan_slots(&mut self) {
        // Headers only: reading and checking every full record took noticeably long on entry.
        for (s, flag) in self.slot_saved.iter_mut().enumerate() {
            *flag = project::slot_used6(s);
        }
    }

    /// First entry after boot: open the current project (converting the old 8 projects once, the first time).
    fn load_initial(&mut self) {
        self.loaded = true;
        if !project::available() {
            self.saved_crc = self.current_crc();
            return;
        }
        let slot = match project::current6() {
            Some(s) => s,
            None => self.migrate(),
        };
        self.scan_slots();
        self.load_slot(slot);
    }

    /// One-time conversion of the old projects (8 slots of 8 per-track patterns, spread over the 1 MB area) into
    /// version-6 projects 1-8, whose slots overlap the old areas. A project is written only once no unconverted
    /// project's data lies under its new slot (when that is possible), so nothing is overwritten before it has been
    /// read. Returns the project to open.
    fn migrate(&mut self) -> usize {
        let last = project::last_slot();
        let mut src = [None; project::OLD_SLOTS];
        for (j, s) in src.iter_mut().enumerate() {
            *s = project::old_source(j);
        }
        let overlaps = |a: (u32, u32), b: (u32, u32)| a.0 < b.0 + b.1 && b.0 < a.0 + a.1;
        let dest = |j: usize| (j as u32 * project::SLOT_SIZE6, project::SLOT_SIZE6);
        while let Some(first) = (0..project::OLD_SLOTS).find(|&j| src[j].is_some()) {
            let j = (0..project::OLD_SLOTS)
                .filter(|&j| src[j].is_some())
                .find(|&j| (0..project::OLD_SLOTS).all(|k| k == j || src[k].map_or(true, |s| !overlaps(dest(j), s))))
                .unwrap_or(first);
            let mut loaded = project::with_buffer(|buf| {
                let rec = project::old_record(buf);
                if project::read_slot(j, rec) {
                    self.load_record(rec);
                    true
                } else {
                    false
                }
            });
            if !loaded {
                let mut v1 = [0u8; project::V1_RECORD];
                if project::read_v1_slot(j, &mut v1) {
                    self.load_v1(&v1);
                    loaded = true;
                }
            }
            if loaded {
                let _ = self.write_project(j);
            }
            src[j] = None;
        }
        project::set_current6(last);
        last
    }

    /// Opens project `slot` (a blank project if it is empty) and makes it the current one.
    fn load_slot(&mut self, slot: usize) {
        let slot = slot % project::SLOTS;
        self.slot = slot;
        self.mutate_orig = [None; TRACKS]; // originals belong to the project being left
        let was_playing = self.playing;
        if self.playing {
            self.stop();
        }
        let loaded = project::with_buffer(|buf| match project::read_slot6(slot, buf) {
            Some(len) => self.load_record6(&buf[..], len),
            None => false,
        });
        if !loaded {
            self.load_blank();
        }
        // A new, untouched project counts as saved: switching away from it does not store an empty project.
        self.saved_crc = self.current_crc();
        if was_playing {
            self.start();
        }
        self.dirty = true;
    }

    /// Projects grid: save the current project if it changed, then open `slot`.
    fn switch_project(&mut self, slot: usize) {
        if slot == self.slot {
            return;
        }
        if self.current_crc() != self.saved_crc {
            self.save();
        }
        self.load_slot(slot);
        project::set_current6(self.slot);
    }

    /// Writes the song to project `slot`. Returns the CRC of what was written, or None (no storage, or too big).
    fn write_project(&mut self, slot: usize) -> Option<u32> {
        let crc = self.current_crc();
        project::with_buffer(|buf| {
            let mut pos = 0;
            let mut fits = true;
            self.emit(&mut |b| {
                if pos + b.len() + 4 <= project::SLOT_SIZE6 as usize {
                    buf[pos..pos + b.len()].copy_from_slice(b);
                } else {
                    fits = false;
                }
                pos += b.len();
            });
            let len = if fits { project::seal6(buf, pos) } else { None }?;
            project::write_slot6(slot, &buf[..len]).then_some(crc)
        })
    }

    fn save(&mut self) {
        if !project::available() {
            return;
        }
        if let Some(crc) = self.write_project(self.slot) {
            self.slot_saved[self.slot] = true;
            self.saved_crc = crc;
            self.save_flash_until = now_us() + 400_000;
            project::set_current6(self.slot);
        }
        self.dirty = true;
    }

    // ---------------- display ----------------

    fn draw(&mut self) {
        let sel = self.selected;
        let colour = TRACK_COLOURS[sel];

        if self.count_in_start_us != 0 {
            // Count-in: the whole grid flashes red on each beat.
            let c = if self.count_in_phase & 1 != 0 { 0xFF0000 } else { 0 };
            for n in 0..64 {
                led::set(top16_index(n), c);
            }
        } else if self.patterns_held || self.projects_held {
            // Top rows: the 16 patterns (two rows) or 32 projects (four rows); the rest of the grid dark.
            let shown = if self.projects_held { project::SLOTS } else { PATTERNS };
            for n in 0..64 {
                let c = if n >= shown {
                    0
                } else if self.projects_held {
                    if n == self.slot { 0xFFFFFF } else if self.slot_saved[n] { 0x303030 } else { 0x050505 }
                } else {
                    let tr = &self.tracks[sel];
                    let p = n as u8;
                    if tr.queued.map_or(false, |(a, b)| (a..=b).contains(&p)) {
                        0x00FFFF
                    } else if p == tr.pattern {
                        0xFFFFFF
                    } else if tr.chain_end > tr.chain_start && (tr.chain_start..=tr.chain_end).contains(&p) {
                        0xFF8000
                    } else if tr.patterns[n].has_notes() {
                        0x301400
                    } else {
                        0x050505
                    }
                };
                led::set(top16_index(n), c);
            }
        } else if self.prob_view {
            // Top two rows: the 16 levels (brighter = more likely), the selected step's level white. Rows 3-6: the
            // steps, lit by how likely they are to play (off steps faint, the selected step white, the playing step
            // in full colour). The bottom two rows are dark.
            let tr = &self.tracks[sel];
            let pat = tr.pat();
            let sel_level = self
                .prob_sel
                .filter(|&st| st < pat.length && pat.steps[st as usize].on)
                .map(|st| prob_level(pat.steps[st as usize].prob));
            for lv in 0..16 {
                let c = scale(0x00E0FF, 40 + PROB_LEVELS16[lv] as u32 * 216 / 100);
                let c = match sel_level {
                    Some(l) if l == lv => 0xFFFFFF,
                    Some(_) => c,
                    None => dim(c, 2),
                };
                led::set(prob_level_index(lv), c);
            }
            let first = self.page as usize * PAGE_STEPS;
            for pad in 0..PAGE_STEPS {
                let step = first + pad;
                let st = pat.steps[step];
                let c = if step >= pat.length as usize {
                    0
                } else if self.prob_sel == Some(step as u8) {
                    0xFFFFFF
                } else if self.playing && tr.playing_step == Some(step as u8) {
                    colour
                } else if st.on {
                    scale(colour, 24 + st.prob as u32 * 120 / 100)
                } else {
                    0x080808
                };
                led::set(prob_step_index(pad), c);
            }
            for pad in 0..16 {
                led::set(note_pad_index(pad), 0);
            }
        } else if self.pset_view {
            // Length on the step pads (the last step white, the playhead in full colour), step rate on the bottom row.
            let tr = &self.tracks[sel];
            let pat = tr.pat();
            let first = self.page as usize * PAGE_STEPS;
            for pad in 0..PAGE_STEPS {
                let step = first + pad;
                let c = if step + 1 == pat.length as usize {
                    0xFFFFFF
                } else if step >= pat.length as usize {
                    0x080808
                } else if self.playing && tr.playing_step == Some(step as u8) {
                    colour
                } else {
                    dim(colour, 3)
                };
                led::set(step_index(pad), c);
            }
            for pad in 0..32 {
                let c = if pad < RATE_HALF_PULSES.len() {
                    let triplet = RATE_HALF_PULSES[pad] % 3 != 0;
                    let on = if triplet { 0xA040FF } else { 0x20FF40 };
                    if pad as u8 == pat.rate {
                        on
                    } else if pad as u8 == DEFAULT_RATE {
                        dim(on, 2)
                    } else {
                        dim(on, 4)
                    }
                } else {
                    0
                };
                led::set(note_pad_index(pad), c);
            }
        } else if let Some(t) = self.held_track {
            // Track settings: channel on steps 1-16, step rate on steps 17-24, outputs on note row 1.
            for step in 0..PAGE_STEPS {
                let c = if step < 16 {
                    if step as u8 == self.tracks[t].channel { 0xFFFFFF } else { 0x101010 }
                } else if (RATE_PAD..RATE_PAD + RATE_HALF_PULSES.len()).contains(&step) {
                    let r = (step - RATE_PAD) as u8;
                    // Straight rates green, triplets purple; the default (1/16) a little brighter when not chosen.
                    let triplet = RATE_HALF_PULSES[r as usize] % 3 != 0;
                    let on = if triplet { 0xA040FF } else { 0x20FF40 };
                    if r == self.tracks[t].pat().rate {
                        on
                    } else if r == DEFAULT_RATE {
                        dim(on, 2)
                    } else {
                        dim(on, 4)
                    }
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
            let first = self.page as usize * PAGE_STEPS;
            for pad in 0..PAGE_STEPS {
                let step = first + pad;
                let s = pat.steps[step];
                let c = if swing_view {
                    if pad >= 26 { 0 } else if pad as u8 <= self.swing - 50 { 0xFFA000 } else { 0x100800 }
                } else if self.duplicate && self.copy_src == Some(step as u8) {
                    0x00FFFF // the step being copied
                } else if track.playing_step == Some(step as u8) && self.playing {
                    0xFFFFFF
                } else if self.step_recording() && self.step_cursor as usize == step {
                    0xFFFFFF // step recording: where the next note goes
                } else if step as u8 >= pat.length {
                    0
                } else if s.on {
                    scale(colour, 64 + s.velocity as u32 * 3 / 2)
                } else if step % 4 == 0 {
                    0x181818
                } else {
                    0x080808
                };
                led::set(step_index(pad), c);
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

        // Track row. Mute armed: unmuted tracks in their colour, muted ones dark red. Solo armed: soloed tracks in
        // their colour, others dark. A queued change flashes white. Otherwise: selected track bright, others dim, and
        // tracks silenced by mute or solo darker still.
        let any_solo = self.tracks.iter().any(|t| t.solo);
        for t in 0..TRACKS {
            let tc = TRACK_COLOURS[t];
            let tr = &self.tracks[t];
            let pending = match self.arm {
                Arm::Solo { .. } => tr.pending_solo,
                _ => tr.pending_mute,
            };
            let track_led = if pending && self.blink {
                0xFFFFFF
            } else if let Arm::Mute { .. } = self.arm {
                if tr.muted { 0x300000 } else { tc }
            } else if let Arm::Solo { .. } = self.arm {
                if tr.solo { tc } else { 0x080808 }
            } else if t == sel {
                tc
            } else if tr.muted || (any_solo && !tr.solo) {
                dim(tc, 6)
            } else {
                dim(tc, 3)
            };
            led::set(101 + t as u8, track_led);
        }
        // Bottom row: Mute (red) and Solo (blue) flash while armed (queued mode alternates with white); dim otherwise.
        let flash = |on: u32, queued: bool| if self.blink { on } else if queued { 0xFFFFFF } else { dim(on, 4) };
        for n in 1..=8u8 {
            let c = match n {
                BTN_REC_ARM => {
                    if self.record_armed { if self.playing { 0xFF0000 } else { 0xA00000 } } else { 0x200000 }
                }
                BTN_MUTE => match self.arm {
                    Arm::Mute { queued } => flash(0xFF0000, queued),
                    _ => 0x300000,
                },
                BTN_SOLO => match self.arm {
                    Arm::Solo { queued } => flash(0x0060FF, queued),
                    _ => 0x000830,
                },
                _ => 0,
            };
            led::set(n, c);
        }
        if self.shift {
            // Sends/Tap: white for a moment after a tap, amber otherwise.
            let tapped = now_us().saturating_sub(self.taps[TAP_HISTORY - 1]) < 120_000;
            led::set(BTN_TAP, if tapped { 0xFFFFFF } else { 0x603000 });
        }

        // Step edit bars: velocity on the right-hand column, gate on the bottom row (replaces the mute row).
        let held = if self.held_track.is_none() && !self.patterns_held && !self.projects_held { self.held_step } else { None };
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
                None if idx == BTN_MUTATE => {
                    if self.mutate_flash_until != 0 {
                        0xFFFFFF
                    } else if matches!(self.mutate_orig[sel], Some((p, _)) if p == self.tracks[sel].pattern) {
                        0xC000FF // mutated: a long press restores
                    } else {
                        0x200030
                    }
                }
                None if idx == BTN_PATTERN_SETTINGS => {
                    if self.pset_view { 0xFF8000 } else { 0x281400 }
                }
                None if idx == BTN_PROBABILITY => {
                    if self.prob_view { 0x00E0FF } else { 0x002830 }
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
        // Green = internal clock, orange = external clock.
        let (on, off) = if self.ext_active { (0xFF8000, 0x602000) } else { (0x40FF40, 0x106010) };
        let play = if self.shift && held_step.is_none() {
            // Shift held: Play shows the clock setting (green = internal clock, the default; orange = follow external).
            if self.ext_ignore { 0x00C000 } else { 0xFF6000 }
        } else if self.playing {
            if beat { on } else { off }
        } else if self.ext_active {
            0x301000
        } else {
            0x101010
        };
        led::set(BTN_PLAY, play);
        led::set(BTN_SEQUENCER, 0xFFFFFF);
        led::set(BTN_NOTE_MODE, 0x202020);
        // ◀ / ▶: the page shown is white; the other page is green while the playhead is on it, grey if the pattern
        // reaches it, dim if not.
        let (sel_len, sel_play) = {
            let tr = &self.tracks[self.selected];
            (tr.pat().length as usize, tr.playing_step.filter(|_| self.playing).map(|p| p as usize / PAGE_STEPS))
        };
        for (pg, btn) in [(0u8, BTN_LEFT), (1u8, BTN_RIGHT)] {
            let c = if self.page == pg {
                0xFFFFFF
            } else if sel_play == Some(pg as usize) {
                0x00C000
            } else if pg == 0 || sel_len > PAGE_STEPS {
                0x303030
            } else {
                0x080808
            };
            led::set(btn, c);
        }
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
        self.patterns_down = false;
        self.prob_view = false;
        self.pset_view = false;
        self.mutate_down_us = 0;
        self.pattern_hold = None;
        self.shift = false;
        self.clear = false;
        self.duplicate = false;
        self.fixed_length = false;
        self.arm = Arm::Off;
        self.copy_src = None;
    }

    fn on_surface(&mut self, event: SurfaceEvent) {
        let i = event.index;
        if self.projects_held && is_grid(i) {
            if let (true, Some(n)) = (event.pressed, grid_number(i)) {
                if (n as usize) < project::SLOTS {
                    self.projects_used = true;
                    self.switch_project(n as usize);
                }
            }
        } else if self.patterns_held && is_grid(i) {
            if let Some(p) = grid_number(i).filter(|&p| (p as usize) < PATTERNS) {
                if event.pressed && self.patterns_down {
                    self.patterns_used = true;
                }
                self.on_pattern_pad(p, event.pressed);
            }
        } else if self.pset_view && is_grid(i) {
            if event.pressed {
                let tr = &mut self.tracks[self.selected];
                if let Some(pad) = step_from_index(i) {
                    let step = self.page as usize * PAGE_STEPS + pad;
                    tr.pat_mut().length = step as u8 + 1;
                    if tr.position > step as u8 {
                        tr.position = 0;
                    }
                } else if let Some(pad) = note_pad_from_index(i).filter(|&p| p < RATE_HALF_PULSES.len()) {
                    tr.pat_mut().rate = pad as u8;
                }
                self.dirty = true;
            }
        } else if self.prob_view && is_grid(i) {
            if event.pressed {
                let (row, col) = ((i / 10) as usize, (i % 10) as usize - 1);
                if row >= 7 {
                    // A level: sets the selected step (if it is on).
                    let lv = (8 - row) * 8 + col;
                    let pat = self.tracks[self.selected].pat_mut();
                    if let Some(st) = self.prob_sel.filter(|&st| st < pat.length) {
                        if pat.steps[st as usize].on {
                            pat.steps[st as usize].prob = PROB_LEVELS16[lv];
                        }
                    }
                } else if (3..=6).contains(&row) {
                    // A step: select it (only steps that are on and inside the pattern).
                    let step = self.page as usize * PAGE_STEPS + (6 - row) * 8 + col;
                    let pat = self.tracks[self.selected].pat();
                    if step < pat.length as usize && pat.steps[step].on {
                        self.prob_sel = Some(step as u8);
                    }
                }
                self.dirty = true;
            }
        } else if let Some(step) = step_from_index(i) {
            self.on_step_pad(step, event.pressed);
        } else if let Some(pad) = note_pad_from_index(i) {
            self.on_note_pad(pad, event.pressed, event.value);
        } else if let (true, false) = ((101..=108).contains(&i), self.arm == Arm::Off) {
            if event.pressed {
                self.mute_solo_tap((i - 101) as usize);
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
        } else if i == BTN_REC_ARM {
            if event.pressed {
                self.toggle_record();
            }
        } else if i == BTN_TAP && self.shift {
            if event.pressed {
                self.tap();
            }
        } else if i == BTN_MUTE || i == BTN_SOLO {
            if event.pressed {
                let queued = self.shift;
                self.arm = match (i == BTN_MUTE, self.arm) {
                    (true, Arm::Mute { .. }) | (false, Arm::Solo { .. }) => Arm::Off,
                    (true, _) => Arm::Mute { queued },
                    (false, _) => Arm::Solo { queued },
                };
                self.dirty = true;
            }
        } else if (1..=8).contains(&i) {
            // The other bottom-row buttons have no function here (Sends = tap tempo with Shift, handled above).
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
                    self.copy_src = None;
                    self.dirty = true;
                }
                BTN_PATTERNS => {
                    // Tap: latch the pattern view open / closed. Hold + pattern pads: momentary, closes on release.
                    self.patterns_down = event.pressed;
                    let close = if event.pressed {
                        self.patterns_used = false;
                        let was_open = self.patterns_held;
                        self.patterns_held = true;
                        was_open
                    } else {
                        self.patterns_used
                    };
                    if close {
                        self.patterns_held = false;
                        self.pattern_hold = None;
                    }
                    self.prob_view = false;
                    self.pset_view = false;
                    self.dirty = true;
                }
                BTN_MUTATE => {
                    // Tap (released before MUTATE_LONG_US) = shuffle; long press = restore (done in on_tick).
                    if event.pressed {
                        self.mutate_down_us = now_us().max(1);
                        self.mutate_long_done = false;
                    } else if self.mutate_down_us != 0 {
                        if !self.mutate_long_done {
                            self.mutate();
                        }
                        self.mutate_down_us = 0;
                    }
                    self.dirty = true;
                }
                BTN_PROBABILITY if event.pressed => {
                    // Tap: open / close the probability page (on the step page shown; no step selected yet).
                    self.prob_view = !self.prob_view;
                    self.prob_sel = None;
                    if self.prob_view {
                        self.pset_view = false;
                        self.patterns_held = false;
                        self.pattern_hold = None;
                    }
                    self.dirty = true;
                }
                BTN_PATTERN_SETTINGS if event.pressed => {
                    // Tap: open / close the Pattern Settings page.
                    self.pset_view = !self.pset_view;
                    if self.pset_view {
                        self.prob_view = false;
                        self.patterns_held = false;
                        self.pattern_hold = None;
                    }
                    self.dirty = true;
                }
                BTN_RECORD if event.pressed => self.toggle_record(),
                BTN_PLAY if event.pressed && self.shift => self.toggle_ext_ignore(),
                BTN_PLAY if event.pressed => {
                    self.play_pressed();
                }
                BTN_LEFT if event.pressed => {
                    self.page = 0;
                    self.dirty = true;
                }
                BTN_RIGHT if event.pressed => {
                    self.page = 1;
                    self.dirty = true;
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
        if self.mutate_down_us != 0 && !self.mutate_long_done && now_us().saturating_sub(self.mutate_down_us) >= MUTATE_LONG_US {
            self.mutate_long_done = true;
            self.mutate_restore();
        }
        if self.mutate_flash_until != 0 && now_us() >= self.mutate_flash_until {
            self.mutate_flash_until = 0;
            self.dirty = true;
        }
        let blink = (now_us() / 250_000) % 2 == 0;
        let pending = self.tracks.iter().any(|t| t.pending_mute || t.pending_solo);
        if blink != self.blink && (self.arm != Arm::Off || pending) {
            self.blink = blink;
            self.dirty = true;
        }
        if self.dirty {
            self.draw();
        }
    }

    fn take_requested_app_switch(&mut self) -> Option<AppId> {
        self.switch.take()
    }
}
