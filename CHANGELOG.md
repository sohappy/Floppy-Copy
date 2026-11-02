# Changelog

All notable changes to Floppy-Copy (formerly HappyGreaseFloppy, and FluxMyFluffyFloppy before that).

## v0.19.0

Changes after user testing (report of 29/10/2026).

* **Drive type: 40 or 80 tracks**, chosen next to the drive selector (and for every drive in
  *Settings → Drives*). On a 40-track drive (5.25" DD 360K, 3"), commands that would send the
  head beyond cylinder 39 are refused with an explanation (e.g. an Amiga or 720K format, double
  step, a seek to cylinder 79); raw flux reads, flux writes and erases are limited to cylinders
  0-39. Before, *Erase disk* or an Amiga format on a 40-track drive drove the head into its end
  stop. The cylinder counts come from gw's own disk definitions. The Format page shows why a
  format does not fit the drive and greys out its button; the double-step option only appears for
  80-track drives.
* **Amstrad CPC**: new *Amstrad CPC (flux SCP)* preset (side 0, cylinders 0-39). The *Amstrad
  CPC / +3 (scan)* preset now reads cylinders 0-39 of side 0 only: gw's `ibm.scan` format
  defines 80 cylinders, which a CPC drive does not have. Presets now carry their own tracks.
* **GCR disks** (Apple II, Commodore 1541/1571, Macintosh): when gw lacks its compiled data
  routines ("Optimised data routines not found", "requires optimised C extension"), Floppy-Copy
  says so at start-up, warns on the Read, Write and Format pages, refuses the command with an
  explanation, and explains how to reinstall the host tools.
* **Head alignment**: when the installed gw has no `align` command (added in host tools 1.21),
  the page says so and links to the download page. gw's command list is read at start-up.
* **Help**: a *Help* page (button in the top bar, entry in the sidebar) with getting started,
  what each page is for, a glossary (image, flux, cylinder, TSPEC, double step…), the known
  limits of the host tools and the wiki links; a short explanation at the top of every page
  (the *?* next to the title hides it).
* **gw commands in the console**: a field at the bottom of the console runs any gw command
  (`reset`, `info`, `rpm --nr 5`…); the selected drive and port are added unless typed;
  commands that change a disk or the device ask for confirmation.
* Disk view: the white ring (track under the head) is labelled, and the legend explains it.
* Seek head: the motor is on by default (many drives only step while it turns).
* Reset device: what `gw reset` does is spelled out (it keeps the drive delays unless asked).
* Drive delays: current values next to the firmware's factory values, with advice on when to
  change them; the step delay default is the factory 10000 µs.
* Interface pins: one-click reads of the index, track 0 and write-protect signals; which pins
  each model can set.
* Drive delays and Interface pins are expert pages, hidden from the sidebar unless *Settings →
  Drives → Show the expert pages* is ticked.
* The exact command is now shown under the buttons of Seek, Clean, Head alignment, Drive delays,
  Interface pins and Reset as well, with the drive-type limits applied.

## v0.18.0

* **Renamed Floppy-Copy — Retro Disk Copier**, © 2026 RCC(A). Window title, retro screen
  (title, subtitle and copyright plate), About line, Windows and macOS metadata, archives
  (`Floppy-Copy.exe`, `Floppy-Copy.app`, `floppy-copy` on Linux).
* Settings, saved templates and the window position of HappyGreaseFloppy (and of
  FluxMyFluffyFloppy) are carried over on first start. On Linux, `install.sh` replaces the former
  menu entry.
* `FLOPPY_COPY_GW` sets the gw executable (`HGF_GW` still works); log files are named
  `floppy-copy-DATE.log`.

## v0.17.2

* **Retro copier screen** (*Retro* in the top bar, *CLASSIC* to go back): a full-window
  interface in the spirit of the classic Amiga disk copiers, with metal plates, a pixel font
  (Silkscreen, SIL OFL), source and target drive lamps, one 0-79 track grid per side and a
  status line. Modes DOSCOPY, NIBBLE, DISK>IMAGE, IMAGE>DISK, FORMAT, ERASE, CHECK and SPEED,
  with START/END cylinders and SIDE; DIRECTORY lists AmigaDOS images, REPEAT runs the last
  operation again. It drives the same jobs and confirmations as the classic interface.
* Copy disk: track range support (used by the retro screen).

* Write: an image larger than the usual size of its format (for example an 82-track Atari ST
  `.st` image) is no longer refused: a warning in the confirmation says so, and gw writes the
  tracks the format defines. A smaller (truncated) image is still refused.
* Hint for IPF images when the SPS/CAPS library gw needs is not installed, with where to put it
  on Windows, macOS and Linux.

## v0.17.1

* **Format disk for Commodore, BBC Micro and Apple II**:
  * Commodore 1541 (D64), 1571 (D71) and 1581 (D81), with disk name and ID: byte for byte the
    image that VICE's `c1541 -format "name,id"` creates (664, 1328 and 3160 blocks free);
  * Acorn DFS (BBC Micro, Master, Electron with DFS), 40 or 80 tracks, single- or double-sided
    (two DFS drives), with a title: catalogue identical to dfsimage's;
  * Apple II DOS 3.3 (volume number) and ProDOS (volume name) data disks: the same volume
    structures as AppleCommander, without the boot code (boot DOS or ProDOS from another disk).
* Double step option for 40-track disks (1541, 1571, Apple II, DFS 40, PC 360K) written in an
  80-track 5.25" drive.
* Tests compare the images with VICE c1541, dfsimage and AppleCommander when they are installed
  (`HGF_APPLECOMMANDER=/path/AppleCommander-ac.jar` for the latter), and write a file into each
  blank image with those tools.

## v0.17.0

* **Format disk for PC, Atari ST and MSX**: blank FAT12 disks (PC 360K, 720K, 1.2M, 1.44M,
  2.88M; Atari ST 360K single-sided and 720K double-sided; MSX 1DD and 2DD) with a volume label,
  written with the matching gw format and verified, or saved as an image. The boot sector has
  the standard BIOS parameter block (identical to `mformat`'s), a "Non-system disk" boot code
  on PCs and a non-executable boot sector on the ST. Writing images was never limited to the
  Amiga: the Write page takes every format gw supports.
* Features of FluxMyFluffyFloppy (the Windows GUI by FrankieTheFluff) that were missing:
  * **templates**: named Read and Write settings with a description, saved, replaced, deleted,
    marked "modified" when changed after loading (custom track ranges included);
  * **disk numbering** for reads, with a preview of the next file name;
  * **image file options** (`disk.hfe::bitrate=250:version=3`): HFE bitrate, version,
    interface mode, encoding, double step, uniform; SCP disk type, revolutions, legacy
    single-sided; KryoFlux sample clock; D88 disk index; with a warning for HFE without bitrate;
  * **command line preview** (Read, Write, Convert, Erase) with a copy button;
  * **log file**: every command and its output appended to one file per day;
  * **serial port list** for `--device` (Linux by-id names, macOS usbmodem, Windows registry);
  * `--bt` (backtrace) option; **open folder** buttons; Convert "next to the source";
  * Help links to the Greaseweazle wiki (getting started, troubleshooting, drive select, track
    specification, image types, precompensation, downloads).
* Taskbar/Dock notification when an operation ends while the window is in the background (a
  request made to FluxMyFluffyFloppy, open there).
* "Restore defaults" keeps saved templates and the log folder.
* Fixed: two theme tests could fail when run in parallel (shared text scale).

### Compared with FluxMyFluffyFloppy 5.2.9 (FrankieTheFluff's Windows GUI)

Every gw command and option FluxMyFluffyFloppy drives is available here, and its interface
features above have been added. Not carried over: the Windows code page option (gw output is
read as UTF-8, `PYTHONIOENCODING` is set) and the separate "Diskdefs folder" (gw takes one
`--diskdefs` file, set in Settings). Only here: Linux and macOS, track-by-track visualizer,
hints, re-read of failed tracks, drive health check, disk copy, Format disk, image inspector,
head alignment chart, Amiga compatibility check.

## v0.16.0

* **Amiga compatibility check** on the Write page, for disks that verify fine in the PC drive
  but do not work on the Amiga: flags options left on from earlier sessions that gw's verify
  cannot catch (swap heads, double step, custom tracks, reverse, hard sectors, fake index,
  forced density) with a one-click reset; reads the ADF and warns when it needs Kickstart 2.0+
  (FFS, International, DirCache) and so shows "Not a DOS disk" on an A500 with Kickstart 1.3;
  explains non-DOS (game) and non-bootable disks; troubleshooting tips; a one-click Amiga
  precompensation preset (`--precomp 40=140`, as trackdisk.device does on DD disks).
* **JetBrains Mono** for the whole interface and the console (clear 0/O and 1/l/I, readable on
  dark backgrounds), embedded in the executable; its SIL OFL licence ships in every archive.
* **Text size** setting (80 % to 160 %) in *Settings → Appearance*, applied immediately and
  remembered; widths that hold text (sidebar, forms) follow it.
* **Theme**: Dark, Light or **System** (follows GNOME, KDE, Windows or macOS). Fixed: the chosen
  theme (and text size) was replaced by the desktop's theme at start-up; both egui styles are now
  customised and the preference is explicit (regression tests simulate the desktop theme).
  Settings are now also saved every 5 seconds, so they survive a crash or a killed process
  (previously only a normal close saved them).
* Production review fixes: cancelling a confirmation no longer interrupts an unrelated wizard;
  temporary images are removed when the app closes during a copy or a health check; default read
  destination in the home folder; correct hint when the gw path is not executable; feedback when
  an action is started while gw is still being detected; better contrast of selected items in
  the light theme; bounded sector numbers from gw output; the pin confirmation names the pin and level; temporary files left by a crash are
  removed at the next start; the disk visualizer fits the window at every text size (discs stack
  when they are too wide side by side).
* Release hygiene: CI builds and tests on Windows and macOS too, `.gitattributes` keeps LF line
  endings (scripts stay runnable when checked out on Windows), `THIRD-PARTY-LICENSES.txt` in every
  archive, yanked `yoke-derive` updated.
* Application icon redrawn as a disk seen from above (cylinders as rings, 11 sectors, active
  track in white, a retried track in amber), with simpler designs for small sizes;
  `tools/make-icons.py` regenerates every icon file.
* macOS: `packaging/package.sh macos` also packages a plain `cargo build --release` made on a Mac.

## v0.15.0

* Renamed **HappyGreaseFloppy** (executable `happygreasefloppy`, crates `hgf-core` / `hgf-gui`,
  environment variable `HGF_GW`). Settings of FluxMyFluffyFloppy are copied over on first start.
* **Hints** on gw errors and suspicious results (`hints.rs`, tested with real messages).
* **Re-read failed tracks** (`DiskMap::start_reread`).
* **Custom tracks badge**; the free *Tracks* field is no longer persisted.
* **Format disk** page (`amigados.rs`: blank AmigaDOS images, verified with xdftool and a gw
  round trip).
* **Copy disk** page (one or two drives, sector or flux).
* New **application icon** (a disk's cylinders and sectors) for Linux (all hicolor sizes + SVG), Windows (multi-size .ico) and
  macOS (.icns), with a simplified design for small sizes.

## v0.14.0

* **Drive health** page: guided diagnosis with verdict and report (logic unit-tested with
  captures from a real Greaseweazle V4.1).
* **Automatic detection of gw** at start-up and from Settings.
* **Ready-to-use executables** for Windows, macOS and Linux built by GitHub Actions; application
  icon (window, title bar, Windows executable, macOS bundle, Linux menu entry); Linux user
  installer, also usable from a source build.
* Application id `happygreasefloppy`, so that GNOME and KDE on Wayland show the icon.

## v0.13.0 (since v0.12.1, as FluxMyFluffyFloppy)

* **Removed** the Ratatui TUI; the GUI is the only front-end.
* **All gw commands** are now available, with all their options.
* **Fixes:**
  * device detection no longer reports "connected" on an error message, and is reset when
    the device disappears;
  * `Command Failed:` (which gw reports with exit status 0) is now treated as a failure;
  * operations can be stopped, and gw is interrupted cleanly when the window closes;
  * the format's own geometry is used: `--tracks` is only passed when you ask for it
    (the old 1541 preset truncated disks to 35 tracks and never double-stepped);
  * the *Recovery* image is saved to your file instead of a hidden temporary one;
    *Quick scan* cleans up after itself; the old *Diagnostic* mode was identical to a normal read;
  * an existing file is never overwritten without confirmation (`--no-clobber` otherwise);
  * the image extension follows the chosen preset;
  * `gw seek` can no longer hang waiting for a console answer;
  * a track state can no longer be downgraded by an unrelated output line; partial tracks of
    `gw convert` are marked bad;
  * regexes are compiled once; the console is capped and virtualised;
  * AmigaDOS explorer followed the parent pointer instead of the hash chain, and an empty root
    block was reported as valid;
  * other failures that gw reports with exit status 0 are detected too: `ERROR: …` (device in
    update mode, unsupported firmware, garbled USB data), `** UPDATE FAILED`, and
    `** SKIPPING UPDATE` (shown as a warning with its reason);
  * closing the window during an operation asks for confirmation (an interrupted write leaves
    the disk incomplete); the drive selector is locked while gw runs;
  * a stopped or failed read/convert warns that the image left on disk may be incomplete;
  * risky firmware-update flags (force, bootloader) are never remembered between sessions;
  * Windows: no console window flashes when gw runs.
