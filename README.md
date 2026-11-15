<p align="center">
  <img src="assets/bcfunl.png" alt="Bandcamp Funnel" width="320"/>
</p>

# bcfunl - The Bandcamp Funnel

Download your Bandcamp purchases, organize a tagged music library, and keep track
of what is complete.

## Highlights

- **Repeatable collection downloads.** SQLite history tracks successes and failures so later runs can focus on unfinished purchases.
- **A verified music library.** Clean up tags and filenames, preserve artwork, and verify decoded audio before publishing releases.
- **Parallel downloads and processing.** Run 2 to 8 releases at a time.
- **Clean up music you already own.** Walk an existing music folder, improve tags offline, and keep filenames, artwork, and file modification dates.
- **Better metadata.** Apply artist mappings, extract catalog numbers, filter invalid genre tags, and prefer confirmed Discogs release genres/styles.
- **Scheduled maintenance.** Acquire, process, and clean successful downloads in one serialized workflow.

## Overview

Think of bcfunl as two steps: download your purchases into a staging folder, then
sort and tag them into your music folder. Your audio stays in its original format.
bcfunl remembers completed work, so you can run it again without starting over.

For better tags, build a local Discogs library. Processing checks it first for
matching releases and can fill in catalog numbers, labels, genres, and credits.
You can work entirely offline or let the Discogs API help with missing matches.

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

For compilations, keep the default `Various Artists` grouping or use
`--varioustag '{label}'` to group by label, such as Soul Jazz Records.
`--varioustag '{albumtitle}'` uses the cleaned album title instead. Individual
track credits and the compilation flag are preserved; a missing label falls
back to `Various Artists`.

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
Artist/date filters and `--count` also work with staged processing and cleanup.
Collection walks use media modification dates instead of purchase dates, as
explained below.
Acquisition and processing attempt new releases first, up to `--count`, then
retry eligible failures without consuming those slots. Five genuine failures
park a release for review; deferred preorders do not exhaust that failure budget.
With `--summary`, per-release results are followed by a collection-wide status
grid. Existing processing totals remain available. Add
`--timings` to processing commands to see where extraction, verification,
tagging, and publication time is spent.

### Limit processing resource use

Want to leave some breathing room for other jobs? Processing uses a 20% resource
budget by default. Set `--nice` to a value from 10 to 50 to change it:

```sh
./bcfunl --process --nice 20 --parallel 8 --user johnsmith \
  --output ./downloads --serverdir /music --summary
```

bcfunl slows down when resources are tight and stops safely if it cannot continue.
Your source files stay put. `--parallel 8` allows up to eight releases at once;
fewer may run when the machine is busy. These controls require Linux and allow
brief spikes. For firm limits, also set CPU and memory limits in your container.

### Stop a sequence cleanly

Every invocation checks `<output>/.bcfunl-stop` at startup and while running.
Create the trigger to stop active work and make later calls skip their steps:

```sh
touch ./downloads/.bcfunl-stop
```

The trigger is never consumed. Remove it when you are ready to resume:

```sh
rm ./downloads/.bcfunl-stop
```

Use `--stopfile /shared/bcfunl.stop` on each call to share another path. Completed
work and history are preserved, and published-payload cleanup is skipped on a
stop. Incomplete items restart on the next run; temporary working files are discarded.
An intentional file stop exits successfully; an unreadable trigger path fails.

### Get better tags with a local Discogs library

First, download the current month's Discogs data:

```sh
./bcfunl --resources --output ./downloads
```

This builds a local reference library at `./downloads/resources/library.sqlite3`.
It includes artists, album groups (called masters), and individual releases.
The full download is large, so allow plenty of disk space and time for the first
setup. Run the same command each month to refresh it.

Already have a library and just need to refresh its genre/style list for testing?
Skip the downloads and full import:

```sh
./bcfunl --resources --genres-only --output ./downloads
```

An older library needs one scan to build its genre cache. Later refreshes reuse
that small cache. Normal monthly resource runs reuse unchanged files and import
only changed datasets, keeping each file's date, size, and checksum on record.

Next, process a small batch using only that library:

```sh
./bcfunl --process --offline --user johnsmith \
  --output ./downloads --serverdir /path/to/music \
  --parallel 2 --count 10 --summary --timings
```

`--offline` means no Discogs API requests. bcfunl looks for matching artist,
album title, and country, then uses confirmed release details to improve tags.
A Discogs release marked Worldwide can match any country. Genres and styles go
through the same cleanup rules before being written to your music files.
If a track still has no genre, offline processing can use an artist default
when the library has one clear artist-name match.

**Keeping the library somewhere else?** Add its full path:

```sh
./bcfunl --process --offline --user johnsmith \
  --output ./downloads --serverdir /path/to/music \
  --library /path/to/resources/library.sqlite3 --count 10 --summary
```

To let the Discogs API help when local matching comes up empty, remove `--offline`
and set `DISCOGS_TOKEN` in your environment or private container `.env` file.
Local matches need no token. Existing libraries built from artists and masters
still provide genre defaults; refresh them to add detailed release matching.
These examples keep your staged downloads. Add `--clean` after you are happy
with the results. For existing extract files and more options, see the
[Discogs resource guide](docs/REFERENCE.md#monthly-discogs-research-resources).

### Tidy an existing music collection

Have music from CDs, vinyl, or other download stores? Use `--collection` to clean
its tags without moving or renaming anything. Build your Discogs library first
so it includes the approved genre list:

```sh
./bcfunl --resources --output ./downloads

./bcfunl --collection --serverdir /path/to/music \
  --output ./downloads --library ./downloads/resources/library.sqlite3 \
  --parallel 3 --count 10 --summary
```

If the library is already built, use `--resources --genres-only` for the genre
update instead of a full resource run.

Start with a small batch you have backed up. Tags are written directly, without
copying or decoding whole audio files. File modification dates stay the same,
existing artwork stays, and no new artwork is added. A requested stop lets the
current metadata update finish first. Direct writes do not keep a rollback copy.
Extra named tags are kept. Supported files are FLAC, MP3,
WAV, AIFF, M4A, and Ogg Vorbis; other formats and symbolic links are left alone.

Country and catalog details in album tags or templated folders, such as
`The Seeds Of Love [UK] [558 105-2]`, fill missing tags and help matching. UK is
stored as GB; EU is kept as a region. Existing tags take priority.

A known release ID or catalog match can supply detailed release tags. Otherwise,
an artist/title match to a Discogs master can supply album artist names and
genres. It does not invent a pressing's catalog, country, date, or release ID.
Without either match, bcfunl tidies what it can safely identify. Unknown genres
are removed: every retained genre must
appear in the cleaned Discogs genre list. This mode makes no online requests.

By default, the walk includes releases with media files modified after the date
two calendar months ago. Dates use UTC and the cutoff day is excluded. Set your
own cutoff with `--fromdt`:

```sh
./bcfunl --collection --serverdir /path/to/music --output ./downloads \
  --fromdt 2026-10-01 --parallel 3 --summary
```

One newer media file qualifies the whole release. Older releases are skipped
before tag reads and matching, and do not use count slots. Even `--force` obeys
the cutoff. For a full historical walk, use an early date such as
`--fromdt 1900-01-01`. Files modified since their last successful walk are picked
up again automatically. The walk preserves their modification dates.

Choose `--parallel` from 3 to 16; the default is 3. Completed folders are recorded
in `./downloads/collection.sqlite3` and skipped next time. Add `--force` to
revisit them after refreshing the library or adding files to an existing folder.
For several machines, give each a separate, nonoverlapping music folder and
access to the local library. See the [collection guide](docs/REFERENCE.md#clean-an-existing-music-collection)
for matching and safety details.

### Check and back up collection history

```sh
./bcfunl --database --user johnsmith --output ./downloads
```

Run this first in the daily schedule. It checks history, saves a verified
SQLite snapshot under `downloads/backups`, and retains seven verified backups.
If checks fail, it restores the latest healthy backup, applies current schema
updates, and preserves damaged files for review. If none is healthy, it stops.
Initialize staging with an acquisition before enabling this daily job.

Preorders and future-dated purchases are tested against their authenticated
download page. Available audio can be acquired before publication; otherwise,
the entry is deferred without a failed-run error. Recorded deferred purchases
are reconsidered within the requested artist, count, and date filters.

### Reclaim staging space

```sh
./bcfunl --cleanup --user johnsmith --output ./downloads
```

`--clean` remains an alias for the same action. Standalone cleanup does not tag
or process releases. Or add `--clean` to a processing command. Cleanup permanently removes eligible
staged payloads only after successful processing and a matching publication
manifest with its listed files still present. Failed or unverifiable releases
are retained. History and published music remain intact. See the
[cleanup rules](docs/REFERENCE.md#clean-processed-downloads) before enabling it.

A ZIP CRC failure parks the failed archive as **no-burn** and triggers one full
fresh download and processing retry. A second CRC failure parks the replacement
for review. Cleanup never deletes parked evidence. Offline mode parks without
downloading; see the [recovery rules](docs/REFERENCE.md#crc-recovery-and-parked-payloads).

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

For a Linux container with all runtime utilities included, see
[Docker Compose on the NAS](docs/CONTAINERS.md), including published images
from successful push builds.

Native processing targets **Linux and macOS**. Native Windows supports downloads
and resources; use the Linux container for processing and cleanup. All native
binaries need a system SQLite library and the utilities for the selected workflow. A compatible prebuilt executable needs neither Rust nor
the sibling VCW source checkout at runtime.

| Utility | Used for |
| --- | --- |
| `aria2c` | Parallel downloads |
| `flock` | Scheduled maintenance wrapper |

On Ubuntu/Debian, install the complete runtime set:

```sh
sudo apt update
sudo apt install aria2 libsqlite3-0
# Optional scheduled maintenance wrapper:
sudo apt install util-linux
```

Audio checking, tagging, artwork handling, and ZIP extraction are built in.
You do not need FFmpeg, ffprobe, metaflac, 7-Zip, or Python for normal use.
The optional scheduling script uses `flock`. Install the `sqlite3` command only
if you want to inspect the history database yourself.

For TrueNAS, use the [deployment guide](docs/TRUENAS.md). If the appliance lacks
the runtime, install it in a Linux container or VM with persistent dataset mounts.

## Data and operating behavior

- Keep the same `--output` directory to retain `collection.sqlite3` history. Treat cookies and databases containing signed links as private.
- Completed work skips on repeat runs. Download `--force` can replace downloaded files; processing never overwrites an existing music-library release.
- Preorders are checked for available downloads. Purchases that are not ready wait for a later run.
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
| [Containers and push builds](docs/CONTAINERS.md) | NAS-local Compose setup, profiling, and native build artifacts |
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
