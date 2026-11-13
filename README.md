# Family Keeper

*Formerly called Family File.*

A family history program for current macOS, Windows and Linux: people, marriages, events,
sources, notes and photos, with family and pedigree views, search, duplicate merging,
printable reports, and GEDCOM import and export.

Family Keeper is a simple, offline family history program in the spirit of the classic desktop
genealogy programs of the 1990s and 2000s. Many of those programs have been discontinued and no
longer run well on current computers. Family Keeper aims to keep their simple way of working (one
family at a time, a pedigree view, plain records) on today's systems.

It is an independent project, written from scratch.

![Family Keeper](docs/screenshot.png)

## Download

Get the latest version from the **Releases** page of this repository:

- **macOS** (Apple Silicon and Intel): `Family-Keeper-macOS.zip`. Unzip it and drag *Family Keeper* to
  Applications. The app isn't notarized by Apple, so the first time, Control-click it, choose
  **Open**, then **Open** again.
- **Windows**: `Family-Keeper-Windows.zip`. Unzip and run `family-keeper.exe`.
- **Linux** (x86_64): `Family-Keeper-Linux-x86_64.tar.gz`. Unpack and run `family-keeper`.

## Building

Install Rust from https://rustup.rs, then in this folder:

- `cargo run --release` builds and starts the program. The program itself is `target/release/family-keeper`.
- On a Mac, `scripts/make-macos-app.sh` builds `dist/Family Keeper.app`, which runs on both Apple
  Silicon and Intel Macs. `scripts/make-macos-app.sh --zip` also makes the zip used for releases.
  `cargo bundle --release` (from `cargo install cargo-bundle`) works too and makes an app for your
  own Mac only.
- On Linux you also need the usual desktop libraries (X11 or Wayland, and a graphics driver with
  Vulkan or OpenGL). On Debian/Ubuntu: `sudo apt install libxkbcommon-dev libgtk-3-dev`.

Run the tests with `cargo test`.

## Releasing

`.github/workflows/build.yml` tests and builds on macOS, Windows and Linux on every push. To publish a
release, set the version in `Cargo.toml`, commit, then tag and push:

```
git tag v0.1.0
git push origin v0.1.0
```

GitHub builds all three systems and attaches the downloads to a new release.

## Your data

- Family files end in `.fkf` (Family Keeper file). Files saved as `.familyfile`, from before the
  program was renamed, still open. Each save keeps the previous version beside it as `.backup`.
- GEDCOM import reads ANSEL, UTF-8, UTF-16 and Windows Latin. Details the program has no field
  for are kept and written back out on export.
- Each file has a home person: the person the file opens on and the first one in exported
  GEDCOM files (marked in the header as `_HOME`). Imported files keep theirs, or else the first
  person in the file. Change it with Edit, then Make this person the home person.
- Edit, then Renumber people gives everyone new IDs 1, 2, 3... (family, present or alphabetical
  order); each person keeps their old ID, and Undo puts everything back.
- Files from other genealogy programs can't be opened directly. Export them as GEDCOM from that
  program first, then import the GEDCOM here.

## Languages

Menus and windows come in **English, Deutsch, Español (España and Latinoamérica), Français,
Norsk, Português (Brasil and Portugal), Русский, Svenska, 日本語, 한국어 and 中文** (Traditional).
Choose **View → Language**. Printed reports are in English.

The translations are in the `lang` folder as standard `.po` files, sharing their key terms with
PAF Classic (see `lang/GLOSSARY.md`). They haven't yet been reviewed by native speakers;
corrections are welcome.

## Options

- `--screenshot <file.png>` starts normally, draws 120 frames, saves a picture and exits.
- `--lang <code>` starts in a language: de, es, es-419, fr, nb, pt-BR, pt-PT, ru, sv, ja, ko or zh-Hant.
- `--show <screen>` starts on one screen: family, pedigree, people, marriages, sources, duplicates,
  reports, edit-person, edit-marriage, add-relative, edit-source, delete or empty.

## Planned

- Automatic updates: when a new version is released, ask whether to update, then download it, verify it and restart

## License

Family Keeper is free for any **noncommercial** use under the
[PolyForm Noncommercial License 1.0.0](LICENSE). Individuals, families, churches, libraries,
schools, genealogy societies and other nonprofits are welcome to use, copy, share and change it.
**Selling it, or using it to make money, is not allowed.**
