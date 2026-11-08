<div align="center">
  <a href="https://fw.anthonyhfm.dev">
    <img src="header.svg" alt="CoreFW Header" width="100%">
  </a>
</div>

> [!IMPORTANT]
> **This is a fork: CoreFW with an 8-track standalone MIDI step sequencer for the Launchpad Pro MK3**
> (branch `lppmk3-sequencer`). Changes from upstream: the sequencer in `src/app/sequencer/`, Pro MK3 DIN MIDI
> (`src/sys/driver/launchpad-pro-mk3/din.rs`), external-flash project storage and small hooks in the app host and
> drivers; see the git history for every change.
> Original CoreFW by Anthony Hofmeister: [anthonyhfm/launchpad-core-firmware](https://github.com/anthonyhfm/launchpad-core-firmware).
> Please report problems with the sequencer here, not upstream.
>
> - User guide: [SEQUENCER_MANUAL.pdf](https://github.com/cyberheater/lpp-emu/blob/main/SEQUENCER_MANUAL.pdf)
> - Firmware download: [releases](https://github.com/cyberheater/launchpad-core-firmware/releases)
> - Emulator and tests used to develop it: [cyberheater/lpp-emu](https://github.com/cyberheater/lpp-emu)

> [!WARNING]
> **Early beta.** The sequencer firmware has been tested on very few devices. Flashing custom firmware onto your own
> Launchpad is entirely at your own risk: it may misbehave or lose your projects and settings, and an interrupted or
> failed update can leave the Launchpad needing recovery from the bootloader. The authors accept no responsibility
> for damage to your device or data.

### Flashing the sequencer (Launchpad Pro MK3)

1. Download the latest `.syx` from the [releases page](../../releases), or build it: `cargo lppmk3`
   (makes `build/lppmk3/fw.syx`).
2. Unplug the Launchpad, hold **Setup**, and plug the USB cable back in while holding it. A MIDI port with
   "Bootloader" in its name appears.
3. Send the file: `python3 tools/flash.py <file>.syx` (needs `pip install mido python-rtmidi`), or use any SysEx
   tool (SysEx Librarian on macOS, MIDI-OX on Windows). Do not unplug while it is sending.
4. The Launchpad restarts; press **Sequencer** to open it. To go back to the original firmware, enter bootloader
   mode the same way and update with Novation Components.

---

CoreFW is a full reimplementation of the firmware for the Novation Launchpad device series. Originally, it was a reverse engineering and binary injection project (`launchpad-injection-cfw`). It now supports all RGB and non-RGB Launchpads with accessible bootloaders.

> [!NOTE]
> This project does not redistribute official Novation firmware. CoreFW is custom firmware built through reverse engineering and independent reimplementation of the original firmware. This project is not affiliated with Novation.

## Installation

Visit [fw.anthonyhfm.dev](https://fw.anthonyhfm.dev) for a step-by-step installation guide that walks you through flashing CoreFW onto your Launchpad.

Alternatively, download the `.syx` file from the [releases page](../../releases) and flash it manually via the bootloader's MIDI port.

## Features

- Performance optimizations for lightshow use
  - ARM Assembly intrinsics to process inputs and lights in parallel
  - Asynchronous MIDI processing
  - Optimized flash storage read/write routines
- [Roadrunner](https://github.com/anthonyhfm/lppmk3-roadrunner) support on the Launchpad Pro Mk3 (high-speed LED system)
- DAW integration: Session control on all targets, model-specific page navigation and native faders
- Multiple built-in color palettes
  - Novation palette
  - Mat1jaczyyy palette
  - MXOS palette
  - Launchpad S (legacy) palette
- 3 flashable custom palette banks
- Color palette editor
- FastLED Apollo Support
- Performance Mode
  - Bottom row mirroring on the Launchpad Pro Mk3
- Programmer Mode
- Custom boot animation

## Device Support

Novation Launchpad support:

| Device                          | Working | RGB | Custom Palettes | Highspeed LED |
| ------------------------------- | :-----: | :-: | :-------------: | :-----------: |
| Launchpad Pro Mk3               |   ✅    | ✅  |       ✅        |      ✅       |
| Launchpad X / FLPad             |   ✅    | ✅  |       ✅        |      ✅       |
| Launchpad Mini Mk3 / FLPad Mini |   ✅    | ✅  |       ✅        |      ✅       |
| Launchpad Pro                   |   ✅    | ✅  |       ✅        |      ✅       |
| Launchpad Mk2                   |   ✅    | ✅  |       ✅        |      ✅       |
| Launchpad Mini Mk1              |   ✅    | ❌  |       ❌        |      ❌       |
| Launchpad S                     |   ✅    | ❌  |       ❌        |      ❌       |

## Roadmap

- Complete native Live instrument modes
- Note Mode
- Chord Mode
- Custom Modes via Novation Components on all Launchpads
- Sequencer Mode
- Core Configurator app

... and more to come!

## Build

Build a firmware package using one of the cargo aliases:

```sh
cargo <device>
```

Available `<device>` targets:

| Alias     | Device                 |
| --------- | ---------------------- |
| `lppmk3`  | Launchpad Pro Mk3      |
| `lpx`     | Launchpad X            |
| `mini`    | Launchpad Mini Mk3     |
| `lpp`     | Launchpad Pro          |
| `mk2`     | Launchpad Mk2          |
| `minimk1` | Launchpad Mini Mk1     |
| `lps`     | Launchpad S            |

To build all targets at once:

```sh
cargo all
```

### Flashing the firmware manually

First of all, you need to put your Launchpad into bootloader mode.

- Unplug the Launchpad from USB, if it is connected.
- Hold the "Capture MIDI" button on the top right corner of the Launchpad.
- While holding the button, plug the Launchpad back into USB.

You are now in bootloader mode.

You can flash the firmware using our `flash.py` tooling. Here is an example for the Launchpad Pro Mk2

```shell
python3 tools/flash.py build/core-launchpad-pro.syx
```

You should see a device named something like "Launchpad MIDI Bootloader" appear in your MIDI devices list. Select it and the script will flash the firmware to your Launchpad.

After the flashing is complete, the Launchpad will automatically reboot into normal operating mode with your new firmware.

## Credits

[Kaskobi](https://www.youtube.com/@Kaskobi) - Boot Animations

[mat1jaczyyy](https://github.com/mat1jaczyyy) - The original lpp-performance-cfw for the Launchpad Pro Mk2

[zeph](https://github.com/ZephyrCodesStuff) - Low-level performance optimizations, improving code soundness and maintainability, refactoring

[Aezuro](https://github.com/Aezurolp) - Added rotation and setup button hold feature.

## License

CoreFW is licensed under the [GNU General Public License v3.0](LICENSE) (GPL-3.0-only).

Copyright (C) 2025-2026 Anthony Hofmeister and contributors.

This means that if you modify CoreFW and distribute or publish your modified
version (e.g. as a fork, a public build, or a firmware release), you must:

- release the full corresponding source code of your modified version under
  the GPL-3.0-only license as well,
- retain the original copyright notices and give credit to Anthony
  Hofmeister as the original author,
- clearly mark what you changed.
