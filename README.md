<p align="center">
  <img src="assets/bcfunl.png" alt="Bandcamp Funnel" width="320"/>
</p>

# bcfunl - The Bandcamp Funnel

Rust CLI to download an authenticated Bandcamp collection with SQLite history.

## Build and run

Requires Rust/Cargo and the system SQLite library and development files
(`libsqlite3-dev` on Debian/Ubuntu). The executable links SQLite directly;
no Python, browser automation, or external `sqlite3` command is used at runtime.
Parallel mode additionally requires a working `aria2c` on PATH.
Builds reuse the sibling VCW checkout at `../vcw` (`vcw-metadata` and
`vcw-types`). Keep that checkout available when building
on another machine; the compiled executable does not need it at runtime.
Export your logged-in **bandcamp.com** cookies in Netscape format.

### External utilities

All runtime utilities must be available on PATH for the relevant mode:

| Utility | Purpose | Required for |
| --- | --- | --- |
| `aria2c` | Parallel transfers | Download mode with `--parallel` |
| `7z` | Validate and extract release archives | Processing ZIP archives |
| `ffprobe` | Inspect audio, tags, and artwork | `--process` |
| `ffmpeg` | Verify decoded audio, extract artwork, and remux supported containers | `--process` |
| `metaflac` | Read and write FLAC tags and artwork | Processing FLAC |
| `gzip` | Stream and validate compressed XML extracts | `--resources` with gzip input |
| `sha256sum` | Verify resource checksums | `--resources` |

Install the complete runtime utility set on Ubuntu/Debian:

```sh
sudo apt update
sudo apt install aria2 7zip ffmpeg flac gzip coreutils libsqlite3-0
```

The `flac` package provides `metaflac`, `ffmpeg` provides `ffprobe`, and
`coreutils` provides `sha256sum`. Older distributions may provide `7z` through
`p7zip-full` instead of `7zip`.

Source builds additionally need Rust/Cargo, a system linker (provided by
`build-essential`), `libsqlite3-dev`, and the sibling VCW checkout described
above. The optional `sqlite3` package provides the command for manual database
queries; bcfunl uses the SQLite library directly.

### Build and launch

```sh
cargo build --release
./target/release/bcfunl --user johnsmith --count 100 \
  --output ./downloads --format FLAC
```

Options:

| Option | Behavior |
| --- | --- |
| `--user` | Bandcamp username; defaults to `johnsmith`. |
| `--artist` | Case-insensitive artist substring; offline SQL supports LIKE wildcards. Download mode uses Search my Collection and checks returned artist names; processing filters stored artists. |
| `--cookies` | Cookie export; defaults to `bandcamp.cookies.txt`. |
| `--fromdt` | Inclusive UTC **purchase** date, `YYYY-MM-DD`. |
| `--todt` | Inclusive UTC **purchase** date, `YYYY-MM-DD`. |
| `--count` | Maximum eligible releases to attempt this run; omitted means all. Completed entries, collection preorders, and missing links do not consume the limit. |
| `--output` | Download directory; defaults to `downloads`. Also holds the history database. |
| `--format` | Case-insensitive lossless preference: `flac` (default), `wav`, `alac`, or `aiff-lossless`. Also accepts `aac-hi`, `vorbis`, `mp3-320`, `mp3-v0`, and legacy `mp3`; these keep the standard fallback order and never force lossy over lossless. |
| `--parallel` | Run 2-8 releases concurrently. Download mode uses aria2c with one connection per file; `--process` uses local workers. Omit for sequential work. |
| `--summary` | Download mode prints the run status table; processing prints attempted/published/skipped/failed counts. |
| `--force` | Retry completed work; download mode replaces matching files after successful transfer. Processing never overwrites server releases. |
| `--process` | Tag, verify and publish completed downloads offline. |
| `--clean` | Remove successfully processed staged payloads, standalone or after `--process`. Preserves history and published files. |
| `--serverdir` | Required library destination for processing. |
| `--library` | Optional read-only Discogs research database for missing-genre fallback during processing. |
| `--template` | Processing filename rule; defaults to the example below. |

For example, `--fromdt 2026-11-01 --todt 2026-12-01` selects purchases made in November 
2026. Unknown purchase dates are excluded when a date filter is active.

## Format and files

Default preference, from lossless to lossy:

`flac -> wav -> aiff-lossless -> alac -> aac-hi -> vorbis -> mp3-320 -> mp3-v0`

FLAC, WAV, AIFF, and ALAC are lossless. AIFF precedes ALAC because
[Bandcamp limits ALAC to 48 kHz](https://get.bandcamp.help/en/articles/15263308-can-i-upload-higher-resolution-audio),
while FLAC/WAV/AIFF preserve the source sample rate. Advertised higher lossless
bit depth and then sample rate outrank container preference. The requested
lossless format breaks equal/unknown-resolution ties; the default order breaks
remaining ties. AAC, Vorbis, and MP3 are lossy. The lossy sequence is a fallback
preference, not a universal quality ranking across different encoders/bitrates.
Files are saved without conversion, resampling, or upsampling.

| Encoding | Single-track extension |
| --- | --- |
| `flac` | `.flac` |
| `wav` | `.wav` |
| `aiff-lossless` | `.aiff` |
| `alac` | `.m4a` |
| `aac-hi` | `.m4a` |
| `vorbis` | `.ogg` |
| `mp3-320` | `.mp3` |
| `mp3-v0` | `.mp3` |

Files go under `<output>/<user>/<artist>/<title> - <item-key> [format].<extension>`.
Artist/title formatting uses the shared local sanitizer. Apostrophe contractions
stay `It's`, `I'm`, and `Don't`; surnames such as `O'Rourke` stay capitalized.
`data/forceupper.txt` supplies canonical whole-word spellings such as `XX`, `DJ`,
and `BP` for artist and title values. This list is embedded at build time.
Original captured `release_title` text is retained; `sanitized_title` supplies
clean names downstream. Known artist-prefixed hyphenated titles use the final
` - ` segment, following `fixtitle.txt`. Trailing single-token bracketed catalog
codes containing letters and digits move to uppercase `catalog_num`; dates,
common disc/volume/BPM markers and multi-word edition text remain in the title.
Existing nonempty catalog metadata wins. Migration back-fills old rows once and
corrects artist display without resetting download/processing history. Existing
archives and published folders are not renamed by migration.

To download up to eight releases at once:

```sh
./target/release/bcfunl --user johnsmith --count 100 --parallel 8
```

Parallel mode resolves release metadata on the main thread, starts each aria2c
transfer after the same randomized pause, and allows up to the requested number
of overlapping transfers. Each process uses one connection and one file, with
splitting and automatic retries disabled; the limit is across releases, not
connections per release. aria2c handles its own redirects without the Rust
request delay. SQLite writes remain on the main thread. The ordinary sequential
path does not require or invoke aria2c.

aria2c downloads each payload into a visible `bcfunl-*` staging directory
under `~/Downloads`, matching the nuascn workflow. The HOME-based Downloads
path is passed as-is: bind mounts and symlinks are not resolved to their backing
paths, preserving the path accessible to Snap. aria2c's stdin entry sets
`out=payload` per download. It also receives a mode-0600 temporary Netscape
cookie file containing only Bandcamp-domain cookies. The file is removed when
the transfer routine returns, including when a valid payload is retained after
a publication error. Cookie values and signed URLs are not printed.

Rust then copies the payload to a destination `.part` file under `--output`,
validates and syncs it, and publishes it atomically. This works across
filesystems. SQLite completion is recorded only after publication succeeds.
The staged copy is removed after success. A failed transfer, invalid/empty
payload, or graceful termination removes its owned staging directory. If a
valid completed payload cannot be published, its staging path is printed and
the payload is retained for recovery until the next parallel startup. No
automatic import of retained payloads is implemented.

Before parallel requests, cleanup removes stale `bcfunl-<pid>-<hex>` directories
under `~/Downloads`, including old payloads and `.aria2` partial/control files.
It preserves live or inaccessible owners and, on Linux, directories referenced
by active transfer `--dir` arguments. Symlinks, unrelated Downloads contents,
SQLite files, and final downloads are untouched. New staging directories start
empty; retained files from finished earlier runs are removed at startup.

During handoff, the staged and destination copies coexist. `~/Downloads` may
be a bind mount, but staging always uses that visible path. Existing contents
of Downloads are left alone. Signed URLs go to aria2c over stdin and its output
is suppressed.
A missing or broken aria2c installation stops parallel mode before collection
requests; it does not silently change to sequential mode.
A format absent from the download metadata falls back to the next available
format; HTTP/transfer failures are recorded for retry on the next run.

A `.part` file is written first, checked against Content-Length when supplied,
synced, and published before SQLite records completion. Payload headers reject
HTML error pages. ZIP contents and audio streams are not decoded or fully
validated. Ordinary failures clean up partial files; after an interrupted
process, remove its stale `.part` before retrying. Existing files without a
completion record require `--force`; completed rows are skipped even if their
files were later moved or removed.

## Process downloaded releases

Processing is offline and uses completed rows in `<output>/collection.sqlite3`
plus embedded audio tags. Start with a small batch:

```sh
cargo run --release -- --process --user johnsmith \
  --output /data/bcfstag3 --serverdir /data2/music --count 1 --summary \
  --template "{artist}/{album} [{country_iso}] [{catalog_num}]/{disc}-{track} - {title}"
```

`--serverdir` is required and must be a separate directory tree from `--output`.
The example template is the default. Omit `--count` for all eligible completed
downloads; `--fromdt` and `--todt` retain their inclusive purchase-date meaning.
Processing defaults to one release at a time. Add `--parallel 2` through
`--parallel 8` for concurrent release processing; aria2c is not needed in this
mode. Start with two workers because extraction and decoded-audio verification
compete for disk bandwidth, CPU, and temporary space. SQLite history writes stay
on the main thread, and each worker opens its own read-only genre library
connection. The server lock and atomic no-overwrite publication remain in force.
`--count` limits scheduled attempts after completed-release skips. On termination,
workers are joined and successful publications are recorded before exit.
Cookies and network services are not used.

Templates support `artist` (primary album artist), `albumartist`, `album`,
`country_iso`, `catalog_num`, `disc`, `track`, `title`, `trackartist`, `performer`,
`artistsort`, `albumartistsort`, and `remixer`, each inside braces. Disc and track
numbers have at least two digits. Audio extensions are appended automatically.
Bracketed sections with missing values disappear; unknown country/catalog values
remain absent. Substituted values are sanitized, while template path traversal,
unknown fields and duplicate filenames are rejected. Include `{album}` in a
release directory before any track-specific directories, for example
`{artist}/{album}/Disc {disc}/{track} - {title}`.

The tag and performer/remixer rules are adapted from mcfunl. ARTIST,
ALBUMARTIST, ARTISTSORT and ALBUMARTISTSORT use the primary album artist.
TRACKARTIST holds separate primary, featured and remix credits; PERFORMER
includes those credits and existing performers. Explicit leading album guests
are included per Stue's tagging policy. Repeated known artist prefixes are
removed from album names. Other embedded fields are retained. Commas inside
names and band names such as `Earth, Wind & Fire` remain intact. Compilations
retain individual track credits under Various Artists.
Embedded album/country/catalog values take precedence over SQLite fallbacks;
SQLite genres supplement existing track genres. Processing normalizes their
combined values through the shared `vcw_metadata::Genres` routine used by
mcfunl, retaining unknown values and deduplicating case insensitively. Discogs
genres and styles use this same routine when provider enrichment is connected. The ALBUM tag includes known country and
catalog suffixes. Cover images are retained, and an archive cover or embedded
picture supplies `folder.jpg` when available.

Requirements: Linux, `7z` for ZIP archives, `ffprobe`, `ffmpeg`, and `metaflac` for
FLAC tags. FLAC, WAV, MP3, AIFF, ALAC/AAC in M4A, and Vorbis/Ogg retain their audio
format. Native FLAC/WAV/MP3 tagging is reused from mcfunl; other containers are
remuxed with audio stream copy. Every prepared track must preserve its decoded
PCM SHA-256 (including sample rate and channel layout) and round-trip the expected tags before publication. No audio
transcoding or provider lookup is performed. CUE releases, unsupported RF64/ID3
variants, missing titles/track numbers, and conflicting metadata require review;
they remain unprocessed with their sources intact.

Existing exact destinations and prior mcfunl-style artist/album folders are
checked by disc/track number, title, count, duration and decoded PCM hashes.
Verified matches skip even when filenames or country/catalog suffixes differ.
Different audio at an existing destination is reported for review. Neither
normal processing nor `--force` overwrites an existing server release.
`--force` only retries completed processing; in download mode it retains its
existing redownload/replacement meaning.

SQLite adds `catalog_num`, `process_date`, `process_error`, and `processed_path`
automatically. Processing history is separate from download dates, errors and
attempt counts. Previously processed outputs with a valid manifest and present
tracks skip quickly; older external outputs are verified again. A successful
new download clears old processing completion so changed audio is reconsidered.
A processor lock covers the destination library. Whole releases publish through
Linux atomic no-replace directory renames, which the destination filesystem must
support. Sources and archives are never renamed or removed. SIGINT/SIGTERM reap
children and clean private staging; SIGKILL may leave hidden staging directories.
Verification and temporary extraction/output copies require time and free space.

## Release genres and country

The collection item’s `item_url` is saved as `release_url` in SQLite, separately
from its authenticated `download_url`. Custom-domain item URLs use the collection
`url_hints.subdomain` to replace the host with `<subdomain>.bandcamp.com` over
HTTPS, preserving the release path and query. If that page redirects back to
its custom domain, the release metadata request permits only the exact custom
host marked verified by the collection and matching its original item URL.
Bandcamp cookies are never sent to the custom domain. Release-page requests use
a full browser-compatible user agent with the bcfunl version appended; a minimal
user agent can trigger a Bandcamp/custom-domain redirect loop. Recapture refreshes this URL even for
completed rows encountered during the walk; missing URLs preserve earlier values.

Before each selected download, the public release page supplies `genre` and
`country_iso` in SQLite. Genre tags keep their spelling and page order, are
deduplicated, and use a semicolon separator, for example `electronic;ambient`.
Tags matching the sidebar location, recognized country names, or explicit
location markers are excluded. Other artist-supplied tags are retained.

`country_iso` is the uppercase ISO 3166-1 alpha-2 code from the artist sidebar,
for example `Belfast, UK` becomes `GB`. English country names and common aliases
are supported using a bundled table. Full US state names in city/state locations
are recognized; ambiguous Georgia and city-only locations remain NULL.
Missing genres also remain NULL. These are the page owner's location and tags,
which may describe a label rather than the individual performer.

Existing databases gain both nullable columns automatically. Completed releases
still skip and are not backfilled. Metadata is saved before the audio transfer,
so it remains available if that transfer fails. Releases without a public page
URL cannot be enriched. The summary columns remain unchanged.

## SQLite history and final output

Direct format URLs are tried first. `/statdownload/` is no longer required:
a mandatory status lookup caused the 0.1.2 regression reported by Stue.
If a transfer returns Bandcamp download-page HTML, the shared recovery path
parses its page data, verifies artist/title identity, selects a fresh supported
format link, and tries again. Each processing attempt allows at most three
actual transfers. The attempts column counts the processing invocation once,
including its internal recovery tries.

During HTML recovery, status lookup may supply a fresh download/retry URL.
Ordinary status failures are logged with their cause and fall back to the
page's direct link; explicit verification challenges stop. Pending status
responses are polled at the existing pace for at most twelve requests. JSONP
is parsed without executing scripts. Returned pages are capped at 2 MiB and
refreshed URLs must use approved Bandcamp/CDN origins. Both transfer modes
support recovery. Rust HTTP requests and aria2c cookie loading honor cookie
domains, paths, and expiry; Bandcamp cookies are not sent to bcbits.com.

The main collection is walked newest-first, using the collection page's
`#pagedata` and the paginated collection endpoint. The separate hidden-items
collection is not included. Eligible links are saved **before** downloading.
`<output>/collection.sqlite3` contains a `releases` table keyed by username and
item key, with artist, release title, publication date, purchase date, download
date, download-page URL, selected format, file path, attempts, last-attempt
timestamp, and last error.
The attempts counter increments before a release starts processing, including
metadata failures, retries, forced downloads, and interrupted attempts. Skipped
completed releases do not increment it. Existing databases are migrated
automatically; historical attempts start at zero because they were not recorded.

Publication dates come from collection/download metadata, or the release page
when needed. Unavailable dates remain SQL NULL. Explicit preorders and future
publication dates are rejected. A preorder discovered on a download page counts
as an attempted release but is not marked complete. Some releases can still
have markup-only labels the current schema does not expose.

A non-NULL `download_date` skips that entry unless `--force` is supplied.
Failures preserve any previous successful completion date and file. The utility
continues after individual failures and exits nonzero if any failed. Collection
capture failures stop processing; links captured so far remain in SQLite.

With `--summary`, completion prints an aligned, Unicode-aware table containing
artist, title, download date, attempts, and last error. Attempts are right-aligned;
headers and separators make the columns readable. Full titles/errors are retained,
and control characters are escaped. Only the current
user's rows with `last_attempt_date >= process start` are included, so failed
attempts and forced retries with an older completion date are also reported.
Skipped completed releases and older/unattempted rows are excluded. The filter
uses UTC timestamps, not purchase dates. Another simultaneous process for the
same user/history can appear if it attempts releases after this process starts.
Legacy rows receive a NULL last-attempt timestamp until processed again.

The last column displays `success` when a download date is present and the stored
error is NULL; otherwise it displays the stored error (or NULL for an interrupted
attempt without an error). Errors now include the underlying cause, such as an
HTTP status or JSON parsing failure, instead of only the outer context label. Signed links are omitted. With no `--summary`, stdout
has no status table. Progress/errors go to stderr. The summary is also printed
on request failures or graceful termination after database initialization; it
may contain just the header when nothing was attempted.

```sh
./target/release/bcfunl --count 100 --output ./downloads --summary > collection-summary.txt
```

Keep cookies and the database private: they contain session material or signed
links. Do not commit them. Keeping the same `--output` preserves history across
runs; changing it creates a separate history database.

## Termination

Ctrl+C (SIGINT) and `kill <pid>` (SIGTERM) stop scheduling new work. In-flight
HTTP waits and randomized delays are canceled. Active aria2c children are
killed and reaped before their staging directories are removed; partial files
owned by the current run are removed. Already published files finish recording
their successful completion, and previous history is preserved. Canceled
releases remain eligible on the next run. The table is dumped when `--summary` is enabled, and SQLite is
closed before exit (130 for SIGINT, 143 for SIGTERM).

Cleanup can wait for filesystem writes or a SQLite operation already in
progress. `kill -9` (SIGKILL), power loss, and crashes cannot run this cleanup
and may leave partial/staging files or children. Never delete SQLite journal
or WAL files as part of download cleanup. This behavior requires restarting
with the rebuilt executable; an already running older process is unchanged.

## Development

```sh
cargo test --offline
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
```

Tests cover CLI/date boundaries, format/resolution preference, cookie scoping,
URL validation, collection filtering, SQLite persistence, failed transfers,
atomic replacement, randomized request-delay bounds, and a 100-release fixture run with skip and force reruns.
Parallel tests cover option bounds, actual worker limits at 2 and 8, per-file
failure handling, history skips, force reruns, and safe aria2c command options.
Termination tests send actual SIGINT/SIGTERM to subprocesses during pacing,
partial-file writes, asynchronous waits, and active child/parallel transfers.
They check child reaping, file cleanup, preserved completion dates, table dumps,
and SQLite integrity. The ignored fixture test is invoked by its parent test.
The fixtures use synthetic audio payloads; they do not prove live downloads.

Protocol reference: [collection and download-page implementation](https://github.com/easlice/bandcamp-downloader/blob/master/bandcamp-downloader.py).
Bandcamp lists its [supported download formats](https://get.bandcamp.help/en/articles/15263234-in-which-formats-can-i-download-my-purchases).

## Artist-scoped runs

```sh
./target/release/bcfunl --user johnsmith --artist "David Holmes" \
  --output /data/bcfstag3 --format FLAC --parallel 8 --summary
./target/release/bcfunl --process --user johnsmith --artist "David Holmes" \
  --output /data/bcfstag3 --serverdir /data/bcfinal-test --summary
```

Date windows and count still apply after artist filtering. Offline artist matching uses `UPPER(artist) LIKE '%' || ? || '%'`
with an uppercased bound value; `%` and `_` retain SQL LIKE wildcard meaning. Collection search may return
title-only matches, which are excluded by the artist check. The authenticated
search response is used directly; no full collection walk follows an empty
search. An authenticated Martin Stephenson search was verified on 2026-10-26.

Offline processing applies user, artist substring and inclusive purchase-date
filters in SQLite before loading eligible releases. It orders by purchase date
descending, then item key. Completed releases verified at this server destination
are skipped before the count limit. `--count` limits actual attempts, including
failures; skipped releases do not consume it. With a date window, rows missing
purchase_date are excluded. Download attempt counts are unchanged by processing.

Private collection purchases stay eligible for downloading. When Bandcamp marks
a collection item `is_private`, its public release-page metadata lookup is
skipped because that page can return 404 even to a purchaser. Publication and
preorder checks still use authenticated download-page/collection data. Genre
and country may remain missing for later processing; the original release URL
is retained in SQLite. Ordinary public-page errors still fail visibly.

Single-audio-file releases default missing track/disc numbers to 1. Existing
tag/filename numbering is preserved; multi-file releases still require track
numbers and explicit invalid numbers remain errors.

## Artist transformations

`data/artistmap.yaml` maps a whole source artist credit to canonical text values:

```yaml
bvdub . Brock Van Wey:
  - bvdub
  - Brock Van Wey
Henry McCullough Andrew Weatherall:
  - Henry McCullough
  - Andrew Weatherall
```

The file loads at startup relative to the working directory. Edits take effect
on the next invocation without rebuilding. If absent, the executable uses its
bundled seed map. Malformed YAML, empty names and conflicting canonical spellings
fail before processing. Single-value lists fix spelling/case alone. Keys match
case insensitively with normalized whitespace; values retain exact spelling and
bypass initcap. Every listed output name also retains its canonical case when
appearing alone. YAML parsing uses the installed serde_yaml crate.

Captured collaborations display as `bvdub / Brock Van Wey`. For mapped
collaborations, ARTIST/ARTISTSORT hold separate mapped values; ALBUMARTIST and
ALBUMARTISTSORT and the release folder use the first mapped name. TRACKARTIST
and PERFORMER include all mapped names, preserving other verified credits.
Unmapped releases keep the existing primary-artist policy. Arbitrary periods,
commas and ampersands remain part of unmapped names.

In track titles, an explicitly declared map-key phrase matching at word
boundaries is removed and its mapped names are added to TRACKARTIST/PERFORMER
for that track only. Surrounding title/mix wording remains; no album-wide
propagation or new REMIXER role is inferred by this transformation. If removal
leaves no title, processing still requires review. Album titles retain their
wording while listed names regain canonical case. Published releases are not
modified by editing the map; test with a fresh destination. Existing database
rows with sanitized titles are not globally rewritten on every map change.

Whole-word `And` and standalone `&` are equivalent for artist-map key lookup,
case insensitively. Output spelling remains literal. One `Martin Stephenson &
The Daintees` entry also matches `Martin Stephenson And The Daintees`. Its
output may include the combined name itself followed by individual names; that
is a flat expansion, not recursive lookup. Duplicate normalized And/& keys
remain errors, as do conflicting indirect expansion keys/cycles. Native tag
readback decodes explicit separators without applying transformations again.

## Tag sanitizing agent skill

The standard skill package is [skills/tag-sanitize](skills/tag-sanitize/SKILL.md),
with SKILL.md, agents/openai.yaml and supporting tag rules. tagskill.md remains a
compatibility pointer. To install it for project discovery from the repository
root in a session where .agents is writable:

```sh
mkdir -p .agents/skills
cp -R skills/tag-sanitize .agents/skills/
```

Then start a session that discovers the installed skill and invoke `$tag-sanitize`.
The source package remains under skills/ for maintenance and deployment. Skill
installation does not add provider lookup, agent JSON import, or an MCP server
to the bcfunl runtime. The current agent session cannot install into its read-only
.agents directory; the package is prepared and validated for that final copy.

`--version` and `-V` include the release and UTC compilation timestamp, for
example `bcfunl v0.1.13 (built 2026-10-26T15:07:41Z)`. The timestamp is embedded
by build.rs and stays fixed for that build; running the executable does not
change it. Cargo reruns the build script when source, bundled data, build script
or Cargo manifests/lockfile change.

## Monthly Discogs research resources

```sh
# Discover the latest artists and masters snapshots in the current UTC month,
# download using the official Discogs links, verify SHA-256, and rebuild SQLite.
./target/release/bcfunl --resources --output ./downloads

# Reuse existing XML or gzip snapshots, without network requests.
./target/release/bcfunl --resources --output ./downloads \
  --resource-file /data2/discogs_20261001_artists.xml \
  --master-file ./downloads/resources/discogs_20261001_masters.xml.gz
```

The database is `<output>/resources/library.sqlite3`, separate from collection
history. `--resource-file` overrides the artists input; `--master-file` overrides
the masters input. Either override may be omitted to acquire that dataset online.
Neither cookies nor artist-map loading is needed. Resources cannot be combined
with `--process`, artist/date/count filters, parallel downloads or server/template
options. `--force` rebuilds even when both source hashes are unchanged.

The official listing is `https://data.discogs.com/?prefix=data%2F2026%2F` for 2026;
the year and current UTC month are determined at runtime. File links use the
site's `?download=data/...` endpoint. No guessed S3 buckets or API credentials
are needed. The current-month selection rejects future/other-month snapshots.
If none is published, use explicit local files rather than silently importing
an older month. Successful downloads are cached under the resource directory.
The publisher's CHECKSUM manifest supplies SHA-256 digests.

Imports stream individual XML records, including gzip input, without extracting
the complete XML. Runtime tools `gzip` and `sha256sum` must be on PATH. A new
database is built privately, indexed and checked before atomic replacement of
the library. An importer lock prevents simultaneous rebuilds. On failure or
SIGINT/SIGTERM, staged data is discarded and an existing library stays intact.
Downloads/extracts supplied by the user and collection history stay untouched.
The build needs the installed quick-xml dependency; no Python runtime is added.
Allow space for cached archives plus old and newly built databases during refresh.

| Table/field | Retained information |
| --- | --- |
| `artists` | Discogs ID, exact name, real name, profile, quality; JSON URLs, name variations, aliases, members and groups, including relationship IDs/attributes |
| `masters` | Master ID, title, main release ID, year, quality, original genre/style JSON arrays |
| `master_artists` | Ordered master credits, artist IDs, source names, name variations and join text |
| `artist_genres` | Normalized combined genre/style value and supporting distinct-master count for each artist ID |
| `artists.genres` | Default combined genre collection, ranked by supporting master count then name |
| `resource_import` | Each input filename, SHA-256, import time, record count and observed XML field counts |

Artist dumps contain no genre/style fields. Their profile text is retained as
text, never used to guess genres. Masters provide the classifications. The
shared `vcw_metadata::Genres` routine combines genres/styles, expands established
mappings, preserves unknown values and removes case-insensitive duplicates. A
master counts once for each primary credited artist even if that artist appears
more than once. There is no propagation through aliases, groups or members.
`artists.styles` remains NULL because the artist dump has no direct style field;
original styles are available on masters and normalized styles contribute to
`artists.genres`. Artists without supporting master evidence retain NULL genres.

```sh
sqlite3 ./downloads/resources/library.sqlite3 \
  "SELECT name,genres FROM artists WHERE name='David Holmes' COLLATE NOCASE;"
```

Genre defaults are research/fallback evidence, not confirmation for a particular
recording. No existing music tags are changed by `--resources`. During `--process`, the
tag sanitizer may use this library only for missing genres, as described below.
Release-specific and embedded genres take precedence. Masters describe release families;
edition-specific catalog numbers, labels, countries and exact dates require
release records. The 10.5 GB October releases archive is not downloaded by this
command. The SQLite catalog is a shared-reader foundation for bcfunl, VCW and
other research tools; no VCW code is changed by creating it.

The October 2026 full import was verified locally: 10,243,337 artists, 2,598,586
masters and 791,640 artists with genre defaults in a roughly 2.6 GiB SQLite
library. An unchanged rerun skipped rebuilding. 129 source credits lacked an
artist ID; they remain searchable by their retained name but contribute no
artist-level genre links. Public downloads were checksum-verified with curl;
the Rust HTTP acquisition path hit sandbox DNS restrictions during this session.

## Missing-genre fallback in tag sanitizing

```sh
./target/release/bcfunl --process --user johnsmith \
  --output /data/bcfstag3 --serverdir /data/bcfinal-test \
  --library /data2/bcfunl/downloads/resources/library.sqlite3 --count 3 --summary
```

Without `--library`, processing uses `<output>/resources/library.sqlite3` when
present. An absent implicit library preserves the previous offline behavior.
An explicitly requested missing or invalid library fails before processing;
the library is always opened SQLite read-only and never created or modified.

The tag sanitizer first combines/normalizes embedded GENRE and confirmed release
genres. Only tracks still lacking genres receive a fallback. It requires one
exact, case-insensitive match for the primary album artist in the research
library; absent/ambiguous names stay unresolved. It does not guess from partial
names, biographies, aliases, group membership or another artist's recording.
The five strongest artist genres/styles are selected by distinct-master count
and name, then passed through the shared VCW normalization routine.

Assignments are marked in `BCFUNL_GENRE_SOURCE` with the Discogs artist ID and
master snapshot filename/hash. The published release manifest and staging
`releases.genre_fallback` JSON retain the applied genres, evidence counts and
affected track numbers. Confirmed `releases.genre` stays unchanged; artist
defaults are not promoted to confirmed release classifications. A new successful
download clears old fallback history. Tag readback and decoded-audio verification
still precede publication. Genre-only borrowed evidence never brings dates,
labels, catalogs, countries or artwork with it.

Already published releases retain the existing skip/never-overwrite behavior.
Use a fresh destination to test new fallback tagging; this command does not
retag previously published server files in place. The source archives and VCW
project remain unchanged.


## Clean processed downloads

Run cleanup alone without cookies or network access:

```sh
./target/release/bcfunl --clean --user stuarthunter --output /data/bcfstg
```

Or process and then clean successfully published releases in the same call:

```sh
./target/release/bcfunl --process --clean --user stuarthunter \
  --output /data/bcfstg --serverdir /data/bcfinal --parallel 2 --summary
```

Cleanup requires a nonempty `process_date`, no processing/download error, and a
matching `.bcfunl-release.json` with all listed published files still present.
It removes only the recorded regular audio/ZIP payload below
`<output>/<user>/`, rejects symlink paths and shared payload references, and
leaves directories, SQLite history, resources, and published music intact.
Already-removed sources are skipped. Existing publications without a matching
bcfunl manifest are retained for review. Cleanup checks publication presence;
it does not repeat decoded-audio verification from processing.

Artist and purchase-date filters also apply to cleanup. `--count` caps removed
payloads independently of the processing attempt limit. Standalone cleanup does
not accept `--force` or `--parallel`; `--force` with processing never bypasses
cleanup checks. When some releases fail processing, cleanup still handles the
successful releases and the command retains its failure exit status. A signal
stops work without starting cleanup. Removal is permanent; download and process
completion history remains unchanged, so normal collection runs do not download
the cleaned files again. Use a deliberate forced download if needed later.

Download, process, and clean modes share a staging-directory lock; overlapping
jobs using that staging directory fail immediately. All scheduled jobs must use
the updated binary to participate in this lock.

## TrueNAS scheduling

For Stue's supplied deployment paths and prepared task commands, see the
[TrueNAS deployment guide](docs/TRUENAS.md).

bcfunl processing currently requires Linux (`renameat2` for atomic publication).
For a Linux-based TrueNAS installation, run it in a Linux container or VM with
staging and music datasets mounted persistently. For FreeBSD-based TrueNAS CORE,
use a Linux VM. Install the runtime utilities inside that environment, rather
than modifying the TrueNAS appliance OS. Build there if a copied executable's
architecture or system-library versions differ.

TrueNAS documents [containers and dataset access](https://www.truenas.com/docs/scale/containers/managingcontainers/)
and [scheduled cron jobs](https://www.truenas.com/docs/scale/systemsettings/advanced/managecronjobs/).
Choose a task account with read/write access to staging and the music destination,
keep cookies readable only by that account, and use absolute paths. The working
directory must contain `data/artistmap.yaml` and other runtime mapping files.
Set `HOME` to a persistent writable directory when using parallel downloads;
aria2c stages under its `Downloads` directory.

These example cron entries run **inside a Linux VM/container with cron installed**.
Replace `/opt/bcfunl`, `/staging`, `/music`, and the user with the actual paths.
Create `/opt/bcfunl/logs` first. They are examples, not installed tasks:

```cron
# Daily acquisition; completed downloads remain skipped.
0 1 * * * cd /opt/bcfunl && ./bcfunl --user stuarthunter --output /staging --cookies /opt/bcfunl/bandcamp.cookies.txt --parallel 2 --summary >> /opt/bcfunl/logs/download.log 2>&1
# Hourly processing followed by cleanup of successfully published payloads.
15 * * * * cd /opt/bcfunl && ./bcfunl --process --clean --user stuarthunter --output /staging --serverdir /music --parallel 2 --summary >> /opt/bcfunl/logs/process.log 2>&1
# Refresh monthly research snapshots after the start of the month.
0 4 5 * * cd /opt/bcfunl && ./bcfunl --resources --output /staging >> /opt/bcfunl/logs/resources.log 2>&1
```

The default genre library is `/staging/resources/library.sqlite3` in these
examples. If acquisition runs past the processing time, the staging lock rejects
that processing invocation; the next hourly run retries. No waiting queue is
created. Runtime `PATH` must include the installed utilities. Cookie expiry can
require a new browser export. Cron logs need periodic rotation.
