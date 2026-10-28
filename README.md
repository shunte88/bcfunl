<p align="center">
  <img src="assets/bcfunl.png" alt="Bandcamp Funnel" width="320"/>
</p>

# bcfunl - The Bandcamp Funnel

Download your Bandcamp purchases, organize a tagged music library, and keep track
of what is complete.

## Highlights

- **Repeatable collection downloads.** SQLite history tracks successes and failures so later runs can focus on unfinished purchases.
- **A verified music library.** Clean up tags and filenames, preserve artwork, and verify decoded audio before publishing releases.
- **Parallel downloads and processing.** Run 2–8 releases at a time.
- **Better metadata.** Apply artist mappings, extract catalog numbers, and fill missing genres from a local Discogs research library.
- **Scheduled maintenance.** Acquire, process, and clean successful downloads in one serialized workflow.

## Overview

bcfunl is a Rust command-line tool for maintaining a local copy of an authenticated
Bandcamp collection. It downloads owned releases into staging, then processes
completed downloads into a separate music library without audio transcoding.
Download and processing history are tracked separately, making retries and regular
maintenance practical.

Use it interactively for a few releases or schedule it after the initial
collection has been downloaded. Processing is offline; monthly Discogs artist
and master extracts provide optional genre fallback without live provider lookups.

## Quick start

With the [runtime installed](#installation) and a compatible executable at
`./bcfunl`, export your logged-in **bandcamp.com** cookies in Netscape format to
`bandcamp.cookies.txt`. Keep that file private:

```sh
chmod 600 bandcamp.cookies.txt
```

Download a small batch:

```sh
./bcfunl --user johnsmith --output ./downloads --count 5 --parallel 2 --summary
```

Process downloaded releases into a separate library:

```sh
./bcfunl --process --user johnsmith --output ./downloads \
  --serverdir /path/to/music --parallel 2 --summary
```

Default filename template:

```text
{artist}/{album} [{country_iso}] [{catalog_num}]/{disc}-{track} - {title}
```

Missing country/catalog fields disappear, and disc/track numbers are padded to
two digits. Change the layout with `--template`. FLAC is the default download
preference; available resolution and format determine the selected source.
See [format selection](docs/REFERENCE.md#format-and-files) for fallback rules.

## Common workflows

### Select an artist or purchase window

```sh
./bcfunl --user johnsmith --artist "David Holmes" --output ./downloads --summary
./bcfunl --user johnsmith --fromdt 2026-10-01 --todt 2026-10-31 \
  --output ./downloads --summary
```

Either date bound can be supplied alone. Dates are inclusive **purchase dates**.
Artist/date filters and `--count` also work with processing and cleanup.

### Fill missing genres

```sh
./bcfunl --resources --output ./downloads
```

This acquires the current month's Discogs artist/master extracts and builds
`./downloads/resources/library.sqlite3`. Processing automatically uses that library
when present. Known genres take precedence; fallback requires a unique exact
artist-name match and retains provenance. Use `--library` for another database.

### Reclaim staging space

```sh
./bcfunl --clean --user johnsmith --output ./downloads
```

Or add `--clean` to a processing command. Cleanup permanently removes eligible
staged payloads only after successful processing and a matching publication
manifest with its listed files still present. Failed or unverifiable releases
are retained. History and published music remain intact. See the
[cleanup rules](docs/REFERENCE.md#clean-processed-downloads) before enabling it.

### Schedule maintenance

Place `bcfunl`, `data/`, private cookies, and `scripts/maintenance.sh` together
in the application directory, keeping the script in its `scripts/` subdirectory:

```sh
sh scripts/maintenance.sh johnsmith /path/to/staging /path/to/music
```

The wrapper runs acquisition, then processing with cleanup. Processing still runs
after acquisition failures; either failure produces a nonzero exit status.
A workflow lock prevents overlapping scheduled runs, and logs go under `logs/`.
See [TrueNAS deployment and schedules](docs/TRUENAS.md) for setup and resource
refresh commands.

## Installation

Processing requires **Linux**, a system SQLite library, and the utilities for
the selected workflow. A compatible prebuilt executable needs neither Rust nor
the sibling VCW source checkout at runtime.

| Utility | Used for |
| --- | --- |
| `aria2c` | Parallel downloads |
| `7z` | Processing ZIP archives |
| `ffprobe`, `ffmpeg` | Inspecting media, verifying audio, and updating supported containers |
| `metaflac` | FLAC tags and artwork |
| `gzip`, `sha256sum` | Discogs resource imports and checksum verification |
| `flock` | Scheduled maintenance wrapper |

On Ubuntu/Debian, install the complete runtime set:

```sh
sudo apt update
sudo apt install aria2 7zip ffmpeg flac gzip coreutils util-linux libsqlite3-0
```

`flac` provides `metaflac`; `ffmpeg` provides `ffprobe`. Older distributions may
provide `7z` through `p7zip-full`. The `sqlite3` command is optional for manual
queries; bcfunl links the SQLite library directly.

For TrueNAS, use the [deployment guide](docs/TRUENAS.md). If the appliance lacks
the runtime, install it in a Linux container or VM with persistent dataset mounts.

## Data and operating behavior

- Keep the same `--output` directory to retain `collection.sqlite3` history. Treat cookies and databases containing signed links as private.
- Completed work skips on repeat runs. Download `--force` can replace downloaded files; processing never overwrites an existing music-library release.
- Preorders remain excluded, with a narrow exception for confirmed single-track releases dated strictly before today (UTC). Same-day and uncertain releases remain excluded.
- Processing verifies decoded audio and tags before publication. Completed-publication skips and cleanup check the manifest and file presence rather than rehashing audio.
- Download, process, and clean jobs sharing staging cannot overlap. The maintenance wrapper also locks its complete workflow.
- Run from the application directory so `data/artistmap.yaml` loads. Mapping changes affect later invocations, not already-published files.

## Documentation

| Guide | Covers |
| --- | --- |
| [Command and behavior reference](docs/REFERENCE.md) | Options, formats, transfers, SQLite history, retries, termination, and limitations |
| [Processing and templates](docs/REFERENCE.md#process-downloaded-releases) | Tagging, verification, naming, and existing-library skips |
| [Artist transformations](docs/REFERENCE.md#artist-transformations) | Canonical names, collaborations, and track-credit rules |
| [Discogs resources](docs/REFERENCE.md#monthly-discogs-research-resources) | Imports, database schema, and genre evidence |
| [TrueNAS deployment](docs/TRUENAS.md) | Paths, runtime checks, and sequential scheduled tasks |
| [Tag sanitizing skill](skills/tag-sanitize/SKILL.md) | Agent instructions and detailed tagging policy |

Run `./bcfunl --help` for the complete CLI, or `./bcfunl --version` for the
release and UTC build timestamp. The tag skill is separate from the executable;
installing it does not add a runtime MCP service or provider lookup pipeline.

## Feedback

When reporting a problem, include the version, sanitized command, relevant error,
and whether it occurred during download, processing, resources, or cleanup.
Remove cookies, signed URLs, and private data.

## License

See [LICENSE](LICENSE) for the GNU General Public License, version 3.
