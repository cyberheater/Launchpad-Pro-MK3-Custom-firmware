// SPDX-License-Identifier: GPL-3.0-only
//
// DIN MIDI outputs of the Launchpad Pro MK3, matching the stock 4.40 firmware's hardware setup:
//   DIN Out 1 = UART4 TX on PA0 (AF8), DIN Out 2 = UART7 TX on PE8 (AF8), DIN In = UART4 RX on PC11 (AF8).
//   31250 baud 8N1 from APB1 = 54 MHz, non-inverted.
// Transmission is interrupt driven (TXE) from a 2 KB ring buffer per output, so sending never blocks. A DIN out carries
// only ~1000 three-byte messages a second, so the output side:
//   - queues whole messages only: when the ring is full, a message is dropped entirely (never cut off part-way, which
//     would garble the stream or leave a note stuck on);
//   - uses running status: a channel message with the same status as the previous one is sent without its status byte,
//     and a note-off with velocity 0 goes out as note-on velocity 0, so note-ons and note-offs share one status
//     (2 bytes instead of 3, ~1/3 more notes per second). Status is sent in full again whenever the line was idle.
// Reception is interrupt driven (RXNE) into a ring buffer; the main loop parses it (running status, real-time, SysEx)
// with poll().

use core::cell::Cell;
use core::sync::atomic::{AtomicU16, Ordering};

use stm32_metapac as pac;

use crate::app::MidiEvent;
use crate::sys::midi::MidiPort;

const TX_RING: usize = 2048;
const RX_RING: usize = 512;
const BRR_31250: u16 = (54_000_000 / 31_250) as u16; // 1728

struct Ring<const N: usize> {
    buf: core::cell::UnsafeCell<[u8; N]>,
    head: AtomicU16, // written by the producer (main loop)
    tail: AtomicU16, // written by the consumer (UART interrupt)
}

unsafe impl<const N: usize> Sync for Ring<N> {}

impl<const N: usize> Ring<N> {
    const fn new() -> Self {
        Self { buf: core::cell::UnsafeCell::new([0; N]), head: AtomicU16::new(0), tail: AtomicU16::new(0) }
    }

    /// Bytes that can still be pushed.
    fn free(&self) -> usize {
        let head = self.head.load(Ordering::Relaxed) as usize;
        let tail = self.tail.load(Ordering::Acquire) as usize;
        (tail + N - head - 1) % N
    }

    fn is_empty(&self) -> bool {
        self.head.load(Ordering::Relaxed) == self.tail.load(Ordering::Acquire)
    }

    fn push(&self, b: u8) -> bool {
        let head = self.head.load(Ordering::Relaxed);
        let next = (head + 1) % N as u16;
        if next == self.tail.load(Ordering::Acquire) {
            return false; // full: drop the byte
        }
        unsafe { (*self.buf.get())[head as usize] = b };
        self.head.store(next, Ordering::Release);
        true
    }

    fn pop(&self) -> Option<u8> {
        let tail = self.tail.load(Ordering::Relaxed);
        if tail == self.head.load(Ordering::Acquire) {
            return None;
        }
        let b = unsafe { (*self.buf.get())[tail as usize] };
        self.tail.store((tail + 1) % N as u16, Ordering::Release);
        Some(b)
    }
}

static OUT1: Ring<TX_RING> = Ring::new();
static OUT2: Ring<TX_RING> = Ring::new();
static IN: Ring<RX_RING> = Ring::new();

/// Running status per output (0 = none). Only touched by send(), on the main loop.
struct Status(Cell<u8>);
unsafe impl Sync for Status {}
static STATUS1: Status = Status(Cell::new(0));
static STATUS2: Status = Status(Cell::new(0));

fn uart(out: u8) -> pac::usart::Usart {
    if out == 1 { pac::UART4 } else { pac::UART7 }
}

fn ring(out: u8) -> &'static Ring<TX_RING> {
    if out == 1 { &OUT1 } else { &OUT2 }
}

fn running(out: u8) -> &'static Cell<u8> {
    if out == 1 { &STATUS1.0 } else { &STATUS2.0 }
}

/// Data bytes after a channel status byte.
fn data_len(status: u8) -> usize {
    match status & 0xF0 {
        0xC0 | 0xD0 => 1,
        _ => 2,
    }
}

pub fn init() {
    // Clocks: GPIOA, GPIOE; UART4, UART7 (kernel clock = PCLK1 by reset default)
    pac::RCC.ahb1enr().modify(|w| {
        w.set_gpioaen(true);
        w.set_gpiocen(true);
        w.set_gpioeen(true);
    });
    pac::RCC.apb1enr().modify(|w| {
        w.set_uart4en(true);
        w.set_uart7en(true);
    });

    // PA0 -> AF8 (UART4_TX), PE8 -> AF8 (UART7_TX)
    pac::GPIOA.moder().modify(|w| w.set_moder(0, pac::gpio::vals::Moder::ALTERNATE));
    pac::GPIOA.afr(0).modify(|w| w.set_afr(0, 8));
    pac::GPIOE.moder().modify(|w| w.set_moder(8, pac::gpio::vals::Moder::ALTERNATE));
    pac::GPIOE.afr(1).modify(|w| w.set_afr(0, 8));
    // PC11 -> AF8 (UART4_RX), pulled up so an unplugged input idles high
    pac::GPIOC.moder().modify(|w| w.set_moder(11, pac::gpio::vals::Moder::ALTERNATE));
    pac::GPIOC.afr(1).modify(|w| w.set_afr(3, 8));
    pac::GPIOC.pupdr().modify(|w| w.set_pupdr(11, pac::gpio::vals::Pupdr::PULL_UP));

    for out in [1u8, 2] {
        let u = uart(out);
        u.cr1().write(|_| {});
        u.brr().write_value(pac::usart::regs::Brr(BRR_31250 as u32));
        u.cr1().write(|w| {
            w.set_ue(true);
            w.set_te(true);
            if out == 1 {
                w.set_re(true);
                w.set_rxneie(true);
            }
        });
    }

    unsafe {
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::UART4);
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::UART7);
    }
}

/// Queues one MIDI message (or any raw bytes, e.g. SysEx) on DIN Out 1 (`out` = 1) or Out 2 (`out` = 2).
/// All of it is queued, or none of it when there is not enough room.
pub fn send(out: u8, data: &[u8]) {
    let (r, rs) = (ring(out), running(out));
    if data.is_empty() {
        return;
    }
    if r.is_empty() {
        rs.set(0); // line idle: send the next status in full
    }
    let status = data[0];
    let mut buf = [0u8; 3];
    let bytes: &[u8] = if (0x80..0xF0).contains(&status) && data.len() == 1 + data_len(status) {
        // One channel message: note-off velocity 0 -> note-on velocity 0, then drop a repeated status byte.
        let status = if status & 0xF0 == 0x80 && data[2] == 0 { 0x90 | (status & 0x0F) } else { status };
        let n = data.len();
        if status == rs.get() {
            buf[..n - 1].copy_from_slice(&data[1..]);
            &buf[..n - 1]
        } else {
            buf[0] = status;
            buf[1..n].copy_from_slice(&data[1..]);
            &buf[..n]
        }
    } else {
        data
    };
    if bytes.len() > r.free() {
        return; // full: drop the whole message
    }
    for &b in bytes {
        r.push(b);
    }
    if (0x80..0xF0).contains(&status) && data.len() == 1 + data_len(status) {
        rs.set(bytes.first().filter(|&&b| b >= 0x80).copied().unwrap_or(rs.get()));
    } else if status < 0xF8 {
        rs.set(0); // anything else but real-time ends running status
    }
    uart(out).cr1().modify(|w| w.set_txeie(true));
}

fn service(out: u8) {
    let u = uart(out);
    let isr = u.isr().read();
    if out == 1 {
        if isr.ore() {
            u.icr().write(|w| w.set_ore(true));
        }
        if isr.rxne() {
            let _ = IN.push(u.rdr().read().dr() as u8);
        }
    }
    if u.cr1().read().txeie() && isr.txe() {
        match ring(out).pop() {
            Some(b) => u.tdr().write(|w| w.set_dr(b as u16)),
            None => u.cr1().modify(|w| w.set_txeie(false)),
        }
    }
}

#[unsafe(export_name = "UART4")]
pub extern "C" fn uart4_handler() {
    service(1);
}

#[unsafe(export_name = "UART7")]
pub extern "C" fn uart7_handler() {
    service(2);
}

/// What DIN In delivered: a channel / system / real-time message, or a complete SysEx (F0 ... F7).
pub enum DinInput<'a> {
    Event(MidiEvent),
    Sysex(&'a [u8]),
}

const SYSEX_MAX: usize = 256;

struct Parser {
    status: u8,
    data: [u8; 2],
    count: usize,
    sysex: [u8; SYSEX_MAX],
    sysex_len: Option<usize>,
}

struct ParserSlot(core::cell::UnsafeCell<Parser>);
unsafe impl Sync for ParserSlot {}
static PARSER: ParserSlot = ParserSlot(core::cell::UnsafeCell::new(Parser {
    status: 0,
    data: [0; 2],
    count: 0,
    sysex: [0; SYSEX_MAX],
    sysex_len: None,
}));

pub fn has_input() -> bool {
    IN.head.load(Ordering::Acquire) != IN.tail.load(Ordering::Acquire)
}

fn event(status: u8, data1: u8, data2: u8) -> MidiEvent {
    MidiEvent { port: MidiPort::Din, status, data1, data2 }
}

/// Parses everything received on DIN In so far. Main loop only.
pub fn poll(mut f: impl FnMut(DinInput)) {
    let p = unsafe { &mut *PARSER.0.get() };
    while let Some(b) = IN.pop() {
        if b >= 0xF8 {
            f(DinInput::Event(event(b, 0, 0))); // real-time: may interleave anything
            continue;
        }
        if b == 0xF0 {
            p.sysex[0] = 0xF0;
            p.sysex_len = Some(1);
            p.status = 0;
            continue;
        }
        if b == 0xF7 {
            if let Some(n) = p.sysex_len.take() {
                if n < SYSEX_MAX {
                    p.sysex[n] = 0xF7;
                    f(DinInput::Sysex(&p.sysex[..n + 1]));
                }
            }
            continue;
        }
        if let Some(n) = p.sysex_len {
            if b < 0x80 {
                if n < SYSEX_MAX - 1 {
                    p.sysex[n] = b;
                    p.sysex_len = Some(n + 1);
                }
                continue;
            }
            p.sysex_len = None; // a status byte aborts the SysEx
        }
        if b >= 0x80 {
            p.count = 0;
            match b {
                0xF6 => {
                    p.status = 0;
                    f(DinInput::Event(event(b, 0, 0)));
                }
                0xF4 | 0xF5 => p.status = 0,
                _ => p.status = b,
            }
            continue;
        }
        if p.status == 0 {
            continue; // stray data byte
        }
        p.data[p.count] = b;
        p.count += 1;
        let need = match p.status {
            0xC0..=0xDF | 0xF1 | 0xF3 => 1,
            _ => 2,
        };
        if p.count == need {
            let (d1, d2) = (p.data[0], if need == 2 { p.data[1] } else { 0 });
            f(DinInput::Event(event(p.status, d1, d2)));
            p.count = 0;
            if p.status >= 0xF0 {
                p.status = 0; // system common cancels running status
            }
        }
    }
}
