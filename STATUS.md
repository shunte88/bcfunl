# Project Status

Updated: 2026-10-30 (America/New_York)
Local release: **v0.1.27**, bounded audio read retries and detailed validation timings

## Current outcome

The downloader and release metadata changes are live accepted. Stue confirmed
five successful transfers after the Black Dog redirect fix and inspected six
completed rows in `/data/bcfstg3/collection.sqlite3`. Genres and canonical release
URLs are present; previous errors cleared. GB is recorded where known. Black
Dog's city-only Sheffield location stays NULL, accepted for later gap-filling.
The full Chrome-compatible release-page user agent resolved the observed loop.

media2026 is running the next large download batch. Latest user counts:
**4,977 complete, 3,716 pending**, total 8,693. The difference from the earlier
8,707 collection count remains unexplained. Stue plans a final `--count 9000`
run without `--force` after the batch. No active remote process was changed.

## Offline processing delivered

Version 0.1.5 adds `--process`, required `--serverdir`, and `--template`.
It reads the existing staging database and completed payloads, tags and names
tracks, verifies audio/tags, skips verified existing server copies and publishes
whole releases atomically. There are no network or provider lookups. mcfunl's
credit/tagging behavior is adapted locally; the partner project is unchanged.

```sh
cargo run --release -- --process --user stuarthunter \
  --output /data/bcfstg3 --serverdir /path/to/music-library --count 1 --summary \
  --template "{artist}/{album} [{country_iso}] [{catalog_num}]/{disc}-{track} - {title}"
```

Replace the example server path with the actual library mount. The template is
also the default. Missing country/catalog brackets disappear, track/disc numbers
are padded, and the audio extension is appended. Disc subdirectories are allowed
after a stable album directory. Unsafe paths, unknown placeholders and filename
collisions fail. `--fromdt`/`--todt` still filter inclusive purchase dates.
Processing is sequential; `--parallel` cannot be combined with `--process`.

Performer, track artist, remix and sort credits retain their separate values.
Primary album artist controls the folder. Compilations preserve each track's
credits and can use Various Artists with label evidence. Existing custom tags,
composer/conductor, identifiers and artwork are retained. Embedded album/country/
catalog tags precede SQLite fallbacks. Known genre tags are applied, and unknown
fields remain absent. Provider gap-filling is deferred.

## History and server skips

SQLite migrates `catalog_num`, `process_date`, `process_error`, `processed_path`
automatically without resetting downloads. Processing does not change download
attempt counters or completion/error fields. A successful new download clears
old processing completion for re-evaluation.

Existing server candidates must match track identity/count, duration, sample
rate/channels and decoded PCM SHA-256. Different filenames and country/catalog
suffixes can still match. Different audio at an existing destination requires
review. Processing never overwrites server releases, including with `--force`.
Sources remain intact. Completed processing with its manifest/present tracks can
skip quickly; older external outputs are verified again.

## Verification

- **67 tests passed**, plus the parent-invoked subprocess fixture. Signal tests
  include processing child reaping and staging cleanup for SIGINT/SIGTERM.
- Formatting, strict Clippy, offline release build and CLI checks passed.
  Executable reports `bcfunl 0.1.6`.
- Real local smoke: a disposable copy of the complete **14-track The Xx album**
  successfully tagged, verified and published. Repeat skipped. Renamed test
  folders/files also matched as an external server copy and skipped. The final
  release build passed publication to a fresh disposable library root.
- FLAC, WAV, MP3, AIFF, ALAC, AAC and Vorbis container/tag round trips passed
  without decoded audio changes. Credit, compilation, artwork, templates,
  migrations, conflicting destinations and failure history are covered.
- Original staging data/history and the production library were not modified.
  No media2026 deployment or production processing was performed.

## Runtime requirements and remaining limits

Processing requires Linux, ffprobe, ffmpeg, metaflac for FLAC, and 7z for ZIP.
The destination filesystem must support atomic no-replace directory renames.
Temporary extracted/prepared copies require space and audio verification takes
CPU time. CUE releases and unsupported RF64/ID3 variants need mcfunl/manual review;
missing required titles/numbers and conflicting metadata remain failures for
review. SIGKILL/power loss cannot run cleanup. Processing completion quick skips
check the manifest and file presence, rather than rehashing previously completed
outputs. The production NAS/full-library run and large-batch timing are untested.

## Persistent preferences

Address the user as Stue. Use US English and no em dashes. Preserve AGENTS.md and
environment-managed directories. Use `cp --update=none` instead of `cp -n` for
no-overwrite copies on this system. Keep cookies and signed download links out
of output, documentation and fixtures. Global preferences remain installed at
`/home/stuart/.codex/AGENTS.md`.

## David Holmes correction and agent trial

Version 0.1.6 cleans repeated artist/featuring prefixes from album names,
keeps the four primary artist fields stable, and puts guests/remixers in
TRACKARTIST and PERFORMER. Tag readback no longer reinfers remix credits,
fixing the GLOK/Glok verification failure. Complete disposable copies of remix
Vol. 1 (12 tracks) and Vol. 2 (7 tracks) published successfully with verified
unchanged audio. Existing production folders were not modified.

Copied mcfunl's tagskill.md locally and launched a tag sanitizing agent trial
against disposable Vol. 2 files and retrieved public Discogs metadata. Results
belong under reviews/. This is a session agent, not an installed MCP server.
The separate mcfunl resolver dry run could not complete provider lookups due to
DNS failures and skipped the release.

## Shared genre integration

bcfunl now depends on the same sibling VCW metadata crate as mcfunl, with
network features disabled. Processing combines existing track genres with release
genres through Genres::builtin().normalize_all(), retaining unknown values and
case-insensitive first-seen uniqueness. A focused test covers mapping expansion,
semicolon inputs, duplicate casing, unknown values and repeated processing with
no release genres. Full suite: 67 passed, one parent-invoked fixture ignored;
strict Clippy and formatting passed. Source builds now require ../vcw including
metadata/types crates and workspace configuration; compiled binaries remain
standalone. Provider lookup and applying agent handoffs remain pending.

## Session handoff: pause requested, 2026-10-25

Stue asked to save memory/status and resume later. Stop task work here.
Local version remains 0.1.6. Final shared-genre release build succeeded;
67 tests passed (one parent-invoked fixture ignored), strict Clippy and fmt passed.
No production audio, remote deployment, or mcfunl source changes were made.

Agent trial artifacts:
- tagskill.md: copied from mcfunl, updated to require shared VCW genre routine.
- reviews/david-holmes-vol2-tag-review.json: seven per-file proposed patches.
- reviews/david-holmes-vol2-tag-review.md: readable evidence and limitations.
Seven ordered tracks matched Discogs 30152429 within 0.84 seconds. Four remixer
corrections: Hardway Brothers + Monkton; add X-Press 2; add Cosmodelica; separate
Panda Bear + Sonic Boom. All seven proposals now also combine genre/style values
through the established map: Electronic, Alternative, Synth-pop, Acid House,
House, Progressive House. No agent patches have been applied to audio.
Discogs describes WAV while source files are FLAC; recording-content match does
not authorize edition-specific country/catalog/date changes. Catalog none stays
missing. Raven Violet on every track is Stue's explicit policy, including the
instrumental mixes, not a provider-confirmed per-track credit.

Next session: connect reusable mcfunl/VCW provider resolution to the skill-agent
handoff and a validated patch application before bcfunl's existing tag/audio
verification and atomic publication. Reuse existing Discogs/MusicBrainz/Bleep
matching, credential handling and shared normalization rather than duplicate
provider logic. Stue confirms mcfunl has working Discogs API credentials/cookies;
our isolated resolver trial failed DNS, not proof of broken mcfunl. Never expose
credentials. mcfunl has a partner maintainer; coordinate cross-project changes.
No installed music MCP server was available; the successful trial used an
explicitly requested session agent following tagskill.md.

Current runnable flow: download to staging, then --process offline with --output
pointing to the same staging DB and --serverdir pointing to a separate library.
--process does NOT yet perform provider lookup or import agent JSON. Start with
--count 3 and a fresh /data/bcfinal-test destination; omit count for full batch.
The default template is {artist}/{album} [{country_iso}] [{catalog_num}]/{disc}-{track} - {title}.
--fromdt/--todt are inclusive UTC purchase dates. Existing published folders are
preserved, even with --force; old incorrectly tagged folders need explicit repair.
Source builds now need sibling ../vcw workspace (metadata/types crates); compiled
binary is standalone. Do not deploy until instructed.

Disposable 19-track smoke copies and the isolated resolver cache remain under
/tmp/bcfunl-holmes-smoke-wy8rmpwy; two earlier source inspection samples under
/tmp/bcfunl-holmes-review-3931bdq8. Public provider response is at
/tmp/bcfunl-discogs-30152429.json. These are temporary and may disappear; persistent
reviews contain the evidence needed to resume. No cleanup/deletion performed at
pause. Live staging archives and production history remain authoritative.

## 2026-10-26: artist selection and sanitized titles (0.1.7)

Resumed at Stue's request. Added --artist as a case-insensitive literal artist
substring in download and processing modes. Download mode calls authenticated
/api/fancollection/1/search_items (Search my Collection), then rejects title-only
matches using band_name. It does not walk the full collection after a search.
Existing cookies, origin checks, pacing, date filters, completion skips and count
limits remain in effect. This branch needs live Bandcamp acceptance testing;
no authenticated collection requests or downloads were launched here.

Processing uses processing_rows to build a parameterized WHERE clause containing
only supplied artist/fromdt/todt predicates. All eight artist/date combinations
are tested, including missing dates, inclusive boundaries, literal SQL wildcard
characters and injection-like input. Purchase dates, not release/download dates,
define the window. Missing purchase dates are included only without date filters.
Results order by purchase_date DESC,item_key. Count limits attempted releases
(including failures) after destination completion skips; skipped releases do not
consume count. Processing does not increment download attempts.

New src/sanitizing.rs centralizes display/title rules. data/forceupper.txt is
embedded at build time (XX, YMCA, USSR, USA, UK, GB, RSD, BP, DJ). Contractions
stay It's/I'm/Don't, surname apostrophes remain O'Rourke/D'Angelo. Artist-prefixed
hyphenated release names use the final spaced-hyphen segment per fixtitle.txt.
Trailing single-token letter+digit catalog hints in () or [] are extracted
uppercase, with common format/volume/BPM and year exclusions; multiword editions
stay. Catalog extraction is a heuristic, not provider edition verification.

SQLite adds sanitized_title and backfills existing rows on open. Original
release_title remains source evidence; corrected artist, sanitized_title and
catalog_num are saved before downstream use. Existing real catalogs take
precedence; catalog none is treated as missing. Download filenames, summaries
and processing use sanitized titles. Download-page identity still compares the
preserved original title to avoid conflating different catalog variants.
Processing also cleans embedded album titles and extracts their catalogs.
Archives and already-published folders are not renamed or retagged by migration;
completion/error/attempt/download history is preserved.

README and copied tagskill.md document the rules and artist-scoped commands.
74 tests passed, one parent-invoked fixture ignored; strict Clippy passed.
No production DB/audio or mcfunl source changes and no remote deployment.
Provider enrichment/agent patch import remain pending as before.

Final 0.1.7 offline release build, formatting and CLI help/version checks passed.

Stue requested SQL LIKE for artist selection. Processing now uses
UPPER(artist) LIKE '%' || ? || '%' with trimmed uppercase bound artist input.
SQL LIKE wildcards % and _ are intentional and covered by tests; bound values
remain injection-safe. Collection search can search artists or titles, but this
CLI continues to assume --artist contains an artist and checks band_name.

## 2026-10-26: private Martin Stephenson purchases (0.1.8)

Investigated staging history read-only: Juvenilia 2 (album417928465) and Back In
The Day (album710019384) fail at public metadata lookup. Authenticated Search my
Collection confirms both items have is_private=true and exactly the stored
daintees.bandcamp.com URLs. Artist search worked live; these are private release
pages, not incorrect slugs. Public catalog lacks these titles.

Added private flag to captured in-memory Release and skip only the public album
metadata lookup for explicitly private items. Keep authenticated download-page
preorder/future-date checks, format selection and downloads unchanged. Keep the
release URL in SQLite; missing public genres/country remain gaps. Public release
errors still propagate. No blanket 404 fallback. Added private/public/preorder
capture regression. Original staging DB/audio were not modified, no downloads or
remote deployment launched. Temporary authenticated search response was removed
without printing cookies or signed download URLs. Separate download-page probes
failed DNS in this environment; end-to-end private transfer requires Stue's run.

0.1.8 verification: 75 tests passed, one parent fixture ignored; strict Clippy,
formatting, offline release build and CLI version check passed.

## 2026-10-26: single-file numbering (0.1.9)

Stue reported missing track numbers on single-audio releases and requested
default disc/track 1. prepare_tags now sets missing track number to 1 only
when exactly one audio file has no usable tag/filename number. Disc already
defaults to 1. Existing tag/filename numbering is preserved; explicit malformed
numbers and multi-file missing numbers still fail for review. Regression covers
single defaults, multiple-track rejection, existing numbers and invalid tags.
76 tests passed, one parent fixture ignored; no live files changed. Failed
processing rows remain eligible for a rerun without --force.

Stue also requested total tracks for single-file releases: existing processing
already writes TRACKTOTAL/TOTALTRACKS=1 and DISCTOTAL/TOTALDISCS=1 for the default
disc. Added explicit assertions. 0.1.9 release build and strict Clippy passed.

## 2026-10-26: artistmap.yaml and title credits (0.1.10)

Stue seeded data/artistmap.yaml: bvdub . Brock Van Wey -> bvdub/Brock Van Wey;
BP Fallon & David Holmes -> BP Fallon/David Holmes; Henry McCullough Andrew
Weatherall -> Henry McCullough/Andrew Weatherall. Added runtime YAML loading
using installed serde_yaml dependency; file is relative to cwd, embedded seeds
are fallback when absent. Values preserve exact case and bypass initcap.
Reject malformed maps, empty lists/names, case-insensitive duplicate keys,
duplicate outputs and conflicting canonical spellings. Maps apply once at
startup; restart to load edits. No sibling project changes.

Capture/backfill artist display uses canonical values joined with spaced slash.
Explicit mapped collaborations produce ARTIST/ARTISTSORT arrays; first mapped
value is primary ALBUMARTIST/ALBUMARTISTSORT/folder. TRACKARTIST and PERFORMER
retain all expanded credits. Explicit mappings override ordinary primary-only
artist policy only for those collaborations. Generic ft/feat/featuring splitting
still works; arbitrary periods/commas/ampersands are not globally split.

Stue clarified Weatherall phrase in a formatted track title must be removed and
added as separate TRACKARTIST/PERFORMER values on affected tracks only. Implemented
word-boundary matches against explicit declared keys, longest first, preserving
remaining title/mix wording and canonical case. No new remixer role inferred by
this map. Generated canonical singleton aliases are not title removal rules.
Empty resulting title still requires review. Album titles retain wording and
restore canonical names after ordinary formatting.

79 tests passed (one parent fixture ignored), including YAML validation, case
preservation, expansions, unchanged unmapped punctuation, native tag readback,
track-local title extraction and unaffected adjacent track. No live library or
staging history changes. Map edits do not rewrite existing sanitized DB rows
globally or modify existing published folders. Provider/agent import remains
pending. README and tagskill document mapping/caller overrides.

0.1.10 final verification: strict Clippy, formatting, offline release build and
version checks passed. Disposable runtime-map smoke confirmed custom eXaCt
spelling is saved during backfill and invalid YAML aborts before processing.
Temporary smoke data removed; no production data changed.

## 2026-10-26: And/ampersand key equivalence (0.1.11)

Stue removed separate And entry; current data/artistmap.yaml has a single
Martin Stephenson & The Daintees mapping whose first output is Martin Stephenson
And The Daintees, followed by Martin Stephenson and The Daintees. Artist lookup
normalizes whole-word And/standalone & only for comparison. Output spelling
is unchanged. Validation now permits self-named final outputs in flat mappings
but still rejects normalized duplicate keys and conflicting indirect mappings
or cycles. No recursive expansion is implemented.

Separated explicit-separator decoding from map expansion for native tag readback:
ALBUMARTIST remains a single combined-name value after write/read, while ARTIST,
TRACKARTIST and PERFORMER keep the three declared credits. Title removal still
uses declared literal phrases at word boundaries. 81 tests passed, one parent
fixture ignored, including both conjunction forms, self-name preservation,
Anderson not being treated as a conjunction, cycle/duplicate rejection and
native tag round trips. No live audio/history or map-file edits made.

0.1.11 final checks: strict Clippy, formatting and offline release build passed.
Current runtime artist map parses successfully; validation probe stopped at its
intentionally nonexistent staging directory before any processing or writes.

## 2026-10-26: standard tag-sanitize skill package

Stue requested moving the tag agent to standard .agents/skills layout. Read and
used system skill-creator guidance. Prepared skills/tag-sanitize/SKILL.md with
standard name/description frontmatter, agents/openai.yaml UI metadata, and
references/tag-rules.md containing preserved detailed rules. Entry point calls
out bcfunl-specific map/artist/title/genre/numbering overrides and distinguishes
JSON proposals from runtime imports/MCP services. tagskill.md is now a compatible
pointer; README documents installation and $tag-sanitize invocation.

The project's .agents directory is environment-managed/read-only in this session;
it was not modified. Actual discovery installation remains the final manual
step in a writable user session: mkdir -p .agents/skills then
cp -R skills/tag-sanitize .agents/skills/ from repository root. Source package
stays in skills/ for maintenance. Restart/discover skills after installing.
Skill quick_validate and local reference checks passed. Documentation/package
changes only; no application build, tag writes or live data changes needed.

## 2026-10-26: version display and release v0.1.12

Stue requested `bcfunl v0...` for the version output and a release bump, followed
by memory/status updates after green checks. Cargo.toml/Cargo.lock now identify
0.1.12. Clap CLI display prefixes CARGO_PKG_VERSION with v; --version and -V
both print exactly `bcfunl v0.1.12`. Internal package version stays numeric.

Verification green: cargo fmt --check; cargo test --offline (81 passed, one
parent-invoked fixture ignored); strict Clippy for all targets; offline optimized
release build. Both version flags checked against exact stdout and empty stderr.
Updated local target/release/bcfunl only; no deployment, live downloads, database
or media changes. Standard skill discovery installation remains pending because
.agents is read-only here; source package is skills/tag-sanitize/.

## 2026-10-26: build timestamp and release v0.1.13

Added build.rs using the existing chrono crate as a build dependency, embedding
BCFUNL_BUILD_DATE in UTC. Both --version/-V now print
`bcfunl v0.1.13 (built 2026-10-26T15:07:41Z)` for the current release binary.
Timestamp identifies the build, not execution time; Cargo reruns its generator
when build.rs, src/, data/, Cargo.toml or Cargo.lock changes. No new runtime
requirements. Package/lockfile bumped to 0.1.13.

Checks green before handoff: 81 tests passed with one parent fixture ignored,
formatting, strict all-target Clippy and optimized offline release build. Both
version flags matched and UTC timestamp parsed/checked against current time.
README, memory and status updated. Local build only; no deployment or live
media/database changes.

## 2026-10-26: Discogs research resources (v0.1.14, import in progress)

Stue requested --resources to acquire monthly Discogs artists and build SQLite,
then expanded scope to masters and artist genre/style defaults. Future shared
research use with VCW is intended, but no VCW changes or 10.5GB releases download
authorized/performed. Current October artist extract in /data2 has 10,243,337
records and no genre/style fields; retain identity data, not inferred biographies.

Added --resources, --resource-file (artists override), --master-file (masters
override). Output is <output>/resources/library.sqlite3. Resource mode bypasses
Bandcamp cookies, artist-map loading and collection history. Filters/process/
parallel/serverdir/template conflict with resources. Uses official current-year
HTML listing and ?download=data/... links, selects newest current UTC month,
verifies CHECKSUM SHA-256, caches archives, and atomically replaces a staged
SQLite library only after both imports and indexes/checks succeed. Unchanged
source hashes skip rebuilding unless --force. Runtime gzip/sha256sum required;
installed quick-xml crate added; no Python runtime. XML is record-streamed.

Schema: artists (exact ID/name/realname/profile/quality plus JSON variations,
URLs/aliases/members/groups); masters (ID/title/main_release/year/quality/raw
genres/styles); master_artists ordered credits/ANV/join/nullable artist ID;
artist_genres normalized combined values with distinct-master frequency;
artists.genres ranked fallback collection; resource_import filenames/hashes/
timestamps/counts/field inventory. Unknown genre remains NULL. Raw styles are
on masters; artists.styles stays NULL as no direct artist style field exists.
Shared VCW normalizer is reused. No propagation through aliases or membership,
no fallback is automatically applied to --process yet.

Official public HTTP links were verified via curl. Downloaded masters archive
to downloads/resources/discogs_20261001_masters.xml.gz (629,708,615 bytes), SHA-256
cd93c9ff801edad023a80225273cda81d6aa6610a7a7fbf3ca2cf3c01df4e2bb
matches published manifest. Direct old S3 links return 403; use official endpoint.
Rust HTTP acquisition in this sandbox hit DNS restrictions; explicit local
inputs exercise the importer without network. User's original XML/gzip untouched.

Full-size tests found/fixed root whitespace accumulation (>8MB across millions
of records), and masters with name-only credits (e.g. master35849/Hecate).
Preserve unlinked names with NULL artist_id; do not invent genre associations.
Failed staged libraries were removed, collection DB/audio unaffected. Artist
pass completed all 10,243,337 before the source-ID case surfaced. Final full run
imports masters first, then artists, to validate master source cases first.

Current run command: target/release/bcfunl --resources --resource-file
/data2/discogs_20261001_artists.xml --master-file
/data2/bcfunl/downloads/resources/discogs_20261001_masters.xml.gz --output
/data2/bcfunl/downloads. Log /tmp/bcfunl-resources-full.log. Unified exec session
69487 is active. Do not start another concurrent importer. Full library has not
yet been published. 88 tests passed + one parent fixture ignored; formatting,
strict Clippy and optimized release build passed. README documents usage/schema.

## v0.1.14 full resource import completed, 2026-10-26

Final library published at /data2/bcfunl/downloads/resources/library.sqlite3,
about 2.6 GiB. Verified read-only after publication:
- artists: 10,243,337
- masters: 2,598,586
- master_artists credits: 3,193,864
- artist_genres rows: 3,600,119
- artists with ranked fallback genre collections: 791,640
- master credits without artist IDs: 129, retained with NULL IDs and excluded
  from inferred artist genre links.

Exact names and genre samples verified for Bvdub (ID824551), Brock Van Wey
(ID956978), David Holmes (ID951), Martin Stephenson (ID283407), The Black Dog
(ID473), and Framewerk (ID3481501). Genre lists are ranked evidence, not an
automatic tag-all list; e.g. David Holmes has 69 distinct source-derived
classifications with Electronic/Downtempo/Leftfield among the leaders.
Artist field inventory across ALL 10,243,337 rows confirms no direct genre/style
fields; one source artist has no name and is retained by ID. Original master
genres/styles are preserved; per-artist defaults use shared VCW normalization.

All importer processes have finished. Earlier in-progress session notes are
superseded. Final log /tmp/bcfunl-resources-full.log; repeat log
/tmp/bcfunl-resources-repeat.log confirms unchanged hashes skipped rebuilding.

Verification complete: 88 tests passed, one parent fixture ignored; fmt, strict
all-target Clippy, optimized release build and CLI mode conflict checks passed.
Library passed PRAGMA quick_check before atomic publication. Real XML/gzip
imports, counts, public-master download SHA-256 and repeat skip are verified.
Automatic acquisition through the Rust HTTP client could not be exercised in
this sandbox due DNS restrictions; public curl fetch/official index/manifest
were verified and source overrides completed the full real import.

No collection history, source archives, published audio or VCW source changed.
--resources builds research data only; --process does not yet consume this
library. Preserve release-specific genres when wiring artist-default fallback
later. Edition catalogs/countries/labels/dates still need release data. The
10.5GB releases archive was not downloaded. Standard tag skill installation
remains pending in read-only .agents; source package remains skills/tag-sanitize/.

## 2026-10-26: tag-sanitizer genre fallback (v0.1.15)

Stue requested using the research library to assign genres where release
matching left no genres, and confirmed the tag sanitizer is the right layer.
No VCW project files changed; only the existing vcw-metadata normalizer is used.

Added src/processing/genres.rs, called after ordinary tag normalization and
existing-server checks, before native tagging/audio verification/publication.
--library optionally specifies the research SQLite; default is
<output>/resources/library.sqlite3 when present. Missing implicit library keeps
old behavior; explicit missing/invalid library fails before processing. Added
SQLite read-only connection flags via shared bindings; library queries cannot
modify or create the research database.

Only tracks with empty normalized GENRE receive fallback; embedded and release
genres remain authoritative. Lookup requires exactly one case-insensitive exact
primary artist name. No partial/biography/alias/group guesses. Up to five
strongest master-derived genre/style values ranked by distinct-master count
are normalized through VCW; unresolved/ambiguous artists stay unassigned.

BCFUNL_GENRE_SOURCE stores artist ID plus master snapshot filename/hash.
Published manifests and new nullable releases.genre_fallback JSON retain
assignments/evidence/affected tracks. Confirmed releases.genre stays unchanged
so defaults are not silently promoted into release facts. Successful new
downloads clear fallback history. Existing published files retain skip/no-
overwrite behavior. Shared skill entrypoint/reference and README updated.

Checks: 91 tests passed, one parent fixture ignored; fmt, strict all-target
Clippy, offline release build and standard skill validator passed. Integration
test verifies fallback tags, manifest/history audit and decoded audio equality;
known per-track genres survive, ambiguous names remain empty, library writes
are rejected and missing paths aren't created.

Real October library smoke on disposable generated FLAC matched David Holmes
ID951 and assigned Electronic, Downtempo, Leftfield, Rock, Score. Native tag
readback and decoded PCM verification passed, provenance/history checked, all
temporary smoke media removed. No live collection, source media, library data
or VCW files changed. Current binary reports bcfunl v0.1.15 with UTC build date.

Example next test: ./target/release/bcfunl --process --user stuarthunter
--output /data/bcfstg --serverdir /data/bcfinal-test
--library /data2/bcfunl/downloads/resources/library.sqlite3 --count 3 --summary.
Use a fresh destination; completed published releases are not retagged in place.
Provider/agent JSON import remains separate pending work.

## Break handoff, 2026-10-26

Stue requested memory/status saved and a short break. Stop work here.
Current local executable: v0.1.15, including UTC build timestamp.

Completed: monthly --resources acquisition/import for artists and masters,
full October SQLite catalog, and read-only genre fallback in the tag sanitizer.
Library: /data2/bcfunl/downloads/resources/library.sqlite3 (~2.6 GiB), with
10,243,337 artists, 2,598,586 masters and 791,640 artist genre defaults.
--process uses <output>/resources/library.sqlite3 when present or an explicit
--library path. Only empty track genres receive up to five ranked defaults
from a unique exact artist match. Provenance is retained in tags, manifests
and releases.genre_fallback; confirmed releases.genre remains unchanged.

Verification green: 91 tests plus the parent-invoked fixture, formatting, strict
Clippy, optimized release build, skill validation and a real-library disposable
FLAC smoke test with tag/PCM verification. No production audio/history or VCW
source changed. No importer is running. Original extracts remain untouched.

Next user acceptance test: add --library
/data2/bcfunl/downloads/resources/library.sqlite3 to a --process command using
a fresh destination. Existing published folders are skipped, not retagged.
Review any unresolved exact artist matches and the selected genre defaults.
Automatic Rust HTTP acquisition still needs a non-sandbox live run because
this environment blocked DNS; official curl download/checksum and complete
local imports succeeded. Full provider/agent JSON import and VCW consumption
of the catalog remain future work. Standard skill source is
skills/tag-sanitize/; .agents/skills installation is still pending in a writable
user session. Do not deploy, start bulk processing or modify VCW while paused.

## 2026-10-27: parallel offline processing (v0.1.16)

Stue requested processing parallelism. Reused --parallel 2..8 with --process;
omitting it preserves one-release-at-a-time behavior. Scoped release workers
use the existing bounded FIFO scheduling pattern from downloads. Main thread
alone records SQLite history; each job opens its own read-only genre library
connection. Server lock, isolated temporary trees, audio/tag verification and
atomic no-replace publication remain intact. Count bounds scheduled attempts
and completed releases still skip. Shutdown stops scheduling, joins workers,
and records successful results before returning the termination error.

README now documents concurrent processing, resource costs, and consolidated
external utility installation (flac provides metaflac). No aria2c dependency
for processing. Version bumped to v0.1.16. Validation: 92 tests passed, one
parent fixture ignored; fmt, strict all-target Clippy and offline release build
passed. Integration covers concurrent publications, count, retry skips,
failure isolation and unchanged download history; fallback integration runs
through the worker path. No production processing or remote deployment run.
Start acceptance testing with --process --parallel 2; disk, CPU and temporary
space may limit throughput. FIFO result collection can wait for an earlier
slow job before refilling a slot, matching the existing download scheduler.

## 2026-10-27: processed payload cleanup and TrueNAS preparation (v0.1.17)

Stue requested standalone --clean and --process --clean for scheduled operation
on TrueNAS. Added src/processing/clean.rs. Cleanup selects the same user/artist/
purchase-date universe, requires process_date, no process_error/last_error and
a matching published manifest with all files still present. Only recorded regular
audio/ZIP payloads beneath <output>/<user>/ are removed. Symlink paths, shared
recorded paths, unavailable publications and mismatched manifests retain sources
and report a failure. Missing sources skip idempotently. No recursive directory
removal. History, file_path, download/process dates, resources and server files
remain intact. Manifest presence validation does not rehash decoded audio.
Existing externally verified server folders without bcfunl manifests are retained.

Combined mode cleans successful rows even if other processing attempts fail,
then retains failure status; cancellation does not start cleanup. --count caps
cleanup removals independently of processing attempts. Standalone --force and
--parallel rejected. --resources conflicts with --clean. Shared staging lock
coordinates updated download/process/clean binaries, and status checks use
SQLite write transactions. Older binaries do not honor this new staging lock.
Lock explicitly unlocks on drop to avoid transient inherited fork descriptors.

95 tests passed, one parent fixture ignored; fmt, strict Clippy and optimized
release build passed. Tests use disposable paths only and cover process+clean,
failed-source preservation, missing/mismatched publication files, symlinks,
user scope, shared payloads, history preservation, repeat cleanup and locking.
README has clean examples, conservative deletion policy and TrueNAS Linux VM/
container scheduling examples. Official TrueNAS cron/container docs consulted.

No NAS deployment, installed scheduled tasks, appliance package changes or live
cleanup performed. Asked Stue for TrueNAS edition/version and staging/music paths;
answer pending. Need those before preparing actual deployment-specific tasks.
Linux required by current renameat2 implementation; FreeBSD CORE needs Linux VM.
Build in deployment environment if copied binary's glibc/architecture differs.
Current executable v0.1.17. PAYLOAD_REVIEW.md and README reference were removed
at Stue's request before this change.

## 2026-10-27: TrueNAS deployment paths supplied

Stue confirmed TrueNAS SCALE 25.10, staging /media/backup, published music
/mnt/tankn4/F8NAS1/music/, and application/support data under ~/bcfunl. NAS
update currently running. Prepared docs/TRUENAS.md with exact paths, runtime/
permission checks and daily acquisition, hourly process+clean and monthly
resources task commands. README links guide. Do not treat these as installed
tasks. No remote NAS access/deployment/cleanup or appliance changes performed.
Task account home path and native-vs-Linux-container/VM runtime still require
verification after update. Host media utilities/glibc compatibility unverified;
use Linux container/VM if host runtime is incomplete, rather than enabling apt
on appliance. Source version stays v0.1.17 (documentation-only update).

## 2026-10-27: TrueNAS staging path correction

Stue changed the staging directory to /media/backup/bcfstage (previously
/media/backup). Updated docs/TRUENAS.md, including all commands and default
research-library path. Application remains ~/bcfunl and music remains
/mnt/tankn4/F8NAS1/music/. No files moved or NAS tasks changed.

## 2026-10-27: obsolete tag skill pointer removed

Removed tagskill.md at Stue's request after confirming it was only a compatibility
pointer. Maintained skill remains skills/tag-sanitize/SKILL.md with supporting
rules and agent metadata. README now links only the maintained package. Earlier
memory/status and review references describe historical trials and are preserved.
No application code or skill rules changed.

## 2026-10-27: sequential scheduled maintenance workflow

Stue approved replacing separate acquisition and processing schedules with one
sequential maintenance task. Added scripts/maintenance.sh OWNER STAGING MUSIC,
deployed under ~/bcfunl/scripts/, with acquisition followed by process --clean.
Processing still runs after acquisition failures; both exit statuses print and
either failure returns nonzero (acquisition status takes priority if both fail).
A flock lock covers the whole workflow; monthly resources use the same lock.
README and docs/TRUENAS.md now show one daily 01:00 maintenance task plus monthly
resources. README examples use johnsmith; personalized TrueNAS guide uses Stue's
collection and supplied paths. Initial full runs may be long; shadowed collection
maintenance skips completed work. No scheduled tasks installed or changed on NAS.
Validation: sh -n plus scripts/test-maintenance.py with disposable stubs verified
ordering, all four success/failure combinations and overlapping-run rejection.
No Rust changes or rebuild. Wrapper adds flock/util-linux dependency; Python is
test-only. Version remains v0.1.17. Memory/status historical separate schedules
are superseded by this entry.

## 2026-10-27: documentation preference and current handoff

Lasting preference: use johnsmith as collection owner in ALL documentation,
sample commands and deployment guides, including the TrueNAS guide. Do not use
Stue's real collection username in examples. Recorded in project AGENTS.md.
README.md and docs/TRUENAS.md checked: both use johnsmith; no real collection
username remains in either. Historical session records remain unchanged.

Current release binary: v0.1.17, built and validated with standalone --clean and
--process --clean. Last Rust checks: 95 tests passed, one parent fixture ignored,
fmt, strict all-target Clippy and offline optimized build passed. Later changes
are documentation and the scheduled-maintenance wrapper, not Rust code.

Current deployment guide: docs/TRUENAS.md for TrueNAS SCALE 25.10. Application
and support data ~/bcfunl; staging /media/backup/bcfstage; music destination
/mnt/tankn4/F8NAS1/music; default research library under staging/resources/.
Prepared workflow script scripts/maintenance.sh takes OWNER STAGING MUSIC.
It runs acquisition then processing with cleanup, even after acquisition failures;
both statuses are reported and either failure returns nonzero. Workflow/resource
jobs share flock; use one daily task, not independent acquisition/processing
schedules. Proposed schedule: maintenance 01:00 daily, resources 04:00 day 5.
Wrapper checked with sh -n and scripts/test-maintenance.py (ordering, failure
combinations, overlap rejection); Python is test-only, flock is a wrapper runtime
dependency. Completed work skips during maintenance of the shadowed collection.

Removed obsolete PAYLOAD_REVIEW.md and tagskill.md at Stue's request. Maintained
tag skill remains skills/tag-sanitize/SKILL.md. Historical reviews retain their
original references. Standard skill installation still pending in read-only
.agents; no runtime agent/MCP or provider JSON import has been added.

No NAS tasks installed, no deployment or live cleanup performed. NAS update was
running when paths were supplied. Next step after update: verify task-account
home, utilities (including flock), binary/glibc compatibility, persistent storage
and dataset permissions; then test a small process run before enabling cleanup.
If appliance runtime is incomplete, use a Linux container/VM rather than modifying
the appliance OS. Prepared commands are not an assertion of NAS readiness.

## 2026-10-27: leading catalog hints and preserved subtitles (v0.1.18)

Stue supplied [AF005Z]/[AF015]/[AF005Y]/[AF007]/[AF020] leading-title catalogs
and Hnrbox003: Upwards At 33 1/3 Degrees - Höga Nord Rekords Singles Collection
Vol.3, then authorized sanitizer changes. Shared src/sanitizing.rs now extracts
leading [catalog] hints using existing trailing-token checks. Bare catalog colon
prefixes require 2–12 ASCII letters, 3–8 digits, up to three suffix letters.
Numeric/date/format/edition prefixes and empty resulting titles are preserved.
Conflicting leading/trailing codes retain leading text and use trailing hint.
Known artist-prefix cleanup removes through the first separator AFTER the known
artist, preserving Title - Subtitle and artists whose names contain spaced
hyphens. Previous last-segment rule is superseded. Confirmed catalog metadata
still wins. Original captured release_title remains untouched.

All callers reuse the shared sanitizer: capture, NULL-title database backfill,
and processing embedded album metadata. Already-populated sanitized_title rows
are not bulk rewritten; collection recapture refreshes them. Existing archives
and published folders are not renamed or retagged in place. README and maintained
tag skill reference document updated rules and bare-prefix heuristic limits.

97 tests passed, one parent fixture ignored. Regression cases cover all six
supplied examples, numeric/ordinary colon titles, conflicting codes, empty titles,
subtitle retention, spaced hyphens inside artist names, embedded album/catalog
processing and precedence of existing confirmed catalogs. Formatting, strict
Clippy and offline optimized build checked. No production data changed or NAS
deployment performed. Local version v0.1.18.

## Latest handoff: v0.1.18 saved

Stue requested memory and status saved after accepting the catalog/title fixes.
Current local release: bcfunl v0.1.18 (built 2026-10-27T21:24:18Z).
Leading bracketed catalog hints and strict bare catalog-colon prefixes now extract
to uppercase catalog_num; artist-prefix cleanup preserves title/subtitle hyphens.
All six supplied examples have regression coverage. Confirmed catalogs retain
precedence. Previously populated sanitized_title rows refresh on collection
recapture, not through an automatic bulk rewrite. Published files remain intact.

Final checks completed: 97 tests passed, one parent fixture ignored, cargo fmt
--check, strict all-target offline Clippy, and optimized offline release build.
README and skills/tag-sanitize/references/tag-rules.md reflect current policy.
No production metadata/media changes, live cleanup or NAS deployment performed.
Next step is Stue's acceptance test with the new binary; TrueNAS runtime and
schedule installation remain unverified/pending. Use johnsmith in all example
commands and documentation. Deployment paths and sequential maintenance workflow
remain as documented in docs/TRUENAS.md. No further work started.

## 2026-10-28: aria2 settings aligned with nuascn (v0.1.19)

Stue first requested split=10/concurrent=5/connections=5, then explicitly steered
to nuascn argument settings. Final implementation supersedes the interim values:
--split=20, --max-concurrent-downloads=16, --max-connection-per-server=16,
--continue=true, --auto-file-renaming=true, --allow-overwrite=true, quiet and
summary interval 0. Removed max-tries=1 to use aria2 default retries like nuascn.
Retained bcfunl's private scoped cookie file/stdin signed URL, TLS validation,
no-conf, timeouts, no preallocation and disabled torrent/metalink following.
Each Rust worker still feeds one release URL to one aria2 process. With parallel8
up to roughly128 connections to one download host; actual splitting depends on
file size/range support. Resume/overwrite/rename apply only in fresh private
staging, not published files; final save still obeys --force. Cross-run resume
and nuascn's separate outer three-attempt loop were not added. Nuascn source
unchanged. README documents final settings; command tests assert tuning and
file-handling flags plus URL/cookie argument protections. Local version v0.1.19.
No real Bandcamp speed benchmark, production download or NAS deployment run.

## 2026-10-28: aria2 overwrite/renaming correction

Stue explicitly requested no overwrite and no renaming. Final aria2 arguments
use --allow-overwrite=false and --auto-file-renaming=false; command regression
assertions and README updated. Resume remains true; split20/concurrent16/
connections16 and default retry limit remain. This supersedes those two true
flags in the earlier nuascn-alignment entry. Release remains v0.1.19, rebuilt
with corrected flags. No production download or live speed test performed.

## 2026-10-28: release v0.1.20 handoff

Stue requested a release bump and memory/status save after correcting aria2 flags.
Current binary: bcfunl v0.1.20 (built 2026-10-28T14:30:46Z).
Cargo.toml and Cargo.lock updated; optimized offline release build passed and
--version verified. No new behavior changes in this bump.

Final aria2 settings: split20, max-concurrent-downloads16,
max-connection-per-server16, continue=true, allow-overwrite=false,
auto-file-renaming=false. Default aria2 retries restored. Each Rust worker still
handles one release URL; --parallel8 can permit roughly128 connections to one
host subject to size/range support. Signed URLs stay on stdin; scoped private
cookies, TLS checks and final publication --force policy remain intact. Resume
is limited to private staging; cross-run resume is not implemented.

Prior full validation: 97 tests passed, one parent fixture ignored, formatting,
strict all-target Clippy and optimized build passed. After disabling overwrite
and renaming, focused command regression and rebuild passed. This version-only
bump was rebuilt, without rerunning the full test suite. No live download speed
benchmark, production cleanup or NAS deployment performed. Next step: Stue's
acceptance test/deployment. All documentation examples use johnsmith. TrueNAS
paths and sequential workflow remain in docs/TRUENAS.md. No further work started.

## 2026-10-28: user-facing README rewrite

User requested rewriting README using banesullivan/README guidance, then clarified
no development wording or personal references and preserve project heading/art.
README now leads with the original artwork/heading, highlights, overview, quick
start and common workflows, followed by runtime installation, operating behavior,
documentation links, feedback and license. No development/test section or named
collection owner; all examples use johnsmith. Detailed existing behavior moved
to docs/REFERENCE.md with fixed relative links and user-facing wording. Removed
personal references from that guide as well. Fixed an inclusive November date
example to end November30 and removed the incorrect example-user default claim.
Artwork remains assets/bcfunl.png, original # bcfunl - The Bandcamp Funnel heading.

Validation: 16 relative documentation links/anchors checked, balanced code fences,
artwork existence, generic owner and README wording assertions passed. No runtime
changes, rebuild or tests needed for documentation-only changes. Release remains
v0.1.20. No deployment, live processing or cleanup performed. Source guidance:
https://github.com/banesullivan/README and its TEMPLATE.md; applied structure
without copying its prose. All earlier runtime/TrueNAS handoff facts unchanged.

## 2026-10-28: release v0.1.21 saved

User requested release bump after the README rewrite and style confirmation.
Current binary: bcfunl v0.1.21 (built 2026-10-28T14:40:49Z).
Cargo.toml/Cargo.lock updated; optimized offline release build passed and version
output verified. This bump adds no runtime behavior beyond v0.1.20.

README is now a concise user-facing entry point based on banesullivan/README
structure. Original project heading and assets/bcfunl.png preserved. Detailed
command/behavior documentation moved to docs/REFERENCE.md. No development/test
section in README, no personal references in user guides, johnsmith used for all
example collection owners. Relative links/anchors and code fences validated.
US English and no em dashes explicitly recorded in project AGENTS.md and checked
in README, docs/REFERENCE.md and docs/TRUENAS.md. Memory/status retain historical
session details; current handoff supersedes older release numbers.

Final aria2 settings remain split20/concurrent16/connections16, resume enabled,
overwrite disabled, automatic renaming disabled, default aria2 retries. Final
publication follows --force; processing never overwrites published releases.
Prior full tests: 97 passed, one parent fixture ignored. Most recent runtime
correction had focused command regression pass; documentation/version-only
changes did not rerun the full suite. No live speed benchmark or NAS deployment.

TrueNAS SCALE25.10 preparation unchanged: application ~/bcfunl, staging
/media/backup/bcfstage, music /mnt/tankn4/F8NAS1/music. Sequential workflow script
acquires then processes/cleans even after acquisition failures, with flock and
both statuses reported. Prepared schedules are not installed tasks. Runtime
compatibility, utility availability and dataset permissions remain unverified on
NAS. No production download, processing or cleanup started. Next action is user
acceptance/deployment testing with v0.1.21 when ready.

## 2026-10-28: stale preorder exception for released single tracks (v0.1.22)

User reported Scuba + Distance - Opposites incorrectly rejected as preorder,
although a single FLAC with an approximately three-year-old release date. User
explicitly restricted exception to single-track releases dated STRICTLY BEFORE
today, not <= today. Shared helpers now inspect type/track_count/num_tracks/
trackinfo cardinality and available date fields. A lone download_items entry
alone is not proof of one track. Explicit count >1 overrides single indicators.
Flagged collection items require explicit single/past evidence to be selected;
unknown flagged items remain excluded. Download preparation evaluates dates and
cardinality before rejecting flags, including public-page metadata. Any known
same-day/future date prevents the exception; missing dates/count remain denied.
Future ordinary publication checks remain. Download format must be available.

HTML link recovery uses the same strict rule and rechecks returned date/count
conflicts. Prepared state retains confirmed past-single eligibility so a refresh
without redundant metadata can use previous evidence. Original title/artist
remain available for recovery identity matching. A stale flag is logged when
ignored. Derived sanitized_title/download basename remove explicit bracketed
pre-release/prerelease/pre release markers after validated exception; original
captured release_title remains unchanged. Processing removes these same explicit
markers from ALBUM/TITLE only for exactly one staged audio file and a known date
strictly before today. Ordinary wording such as Pre-Release Versions remains.
No global stripping or multi-file/same-day/missing-date marker cleanup.

Changes: src/collection.rs, src/main.rs, src/download.rs, src/sanitizing.rs,
src/processing/mod.rs and focused tests. No dependencies/schema additions.
README, docs/REFERENCE.md and tag skill reference updated, US English/no em dash
and johnsmith examples retained. bcfunl v0.1.22 (built 2026-10-28T17:21:26Z).
101 tests passed, one parent fixture ignored; cargo fmt --check, strict offline
all-target Clippy and optimized offline release build passed. Tests cover strict
date boundary, conflicting dates, cardinality, capture, metadata parsing,
refreshed links and single-file-only marker cleanup.

No authenticated live inspection/download of Opposites performed. Actual release
metadata still needs user's acceptance test, and unknown single-track evidence
remains conservatively excluded. No production cleanup/deployment started.
Earlier blanket preorder rejection policy is superseded only by this exception.

## 2026-10-28: Ombra International title/catalog rule (v0.1.23)

User requested recognizing Ombra Intl numeric catalog titles, retaining the
catalog designation if it is the entire title, expanding Intl to International,
and allowing normal country/catalog template suffixes. Interpreted user typo
Ibternational as International. Official label Bandcamp listing confirmed spaced
and joined forms (Ombra INTL 006 / Ombra INTL021) and titles with subtitles.
Source: https://ombrainternational.bandcamp.com/ .

Shared sanitizer recognizes case-insensitive Ombra Intl / Ombra International
prefix with 1–6 digits, preserving leading zeros. Catalog hint normalized as
OMBRA INTL 006. Catalog-only titles become Ombra International 006, including
bracketed-only designation; separate subtitles retain their wording while the
catalog is extracted. Leading/trailing bracketed forms supported. Invalid,
non-numeric and overlong codes are not inferred. Existing real catalog metadata
wins. Country is taken only from existing metadata, never inferred by label.
Default directory example with supplied country DE:
Artist/Ombra International 006 [DE] [OMBRA INTL 006]. Reprocessing the expanded
title is idempotent; original captured release_title remains evidence. Existing
non-NULL sanitized titles refresh on capture, not bulk rewrite. Published media
is not renamed or retagged automatically.

Updated src/sanitizing.rs, focused sanitizer/processing tests, reference guide
and tag skill rules. bcfunl v0.1.23 (built 2026-10-28T21:24:21Z). Full validation: 103 tests passed,
one parent fixture ignored; formatting, strict all-target Clippy and optimized
offline release build passed. Processing test checks preserved album name,
country/catalog template suffixes, tag fields and confirmed-catalog precedence.
No production files/history changed, no NAS deployment or live processing run.

## 2026-10-29: genre capture/processing hygiene (v0.1.24)

User reported server genre vocabulary grew from ~720 curated values to3000+
after processing. Confirmed cause in current code: Bandcamp captured free tags
excluding locations, and shared VCW normalization intentionally passed unknown
values through. No live Discogs release confirmation pipeline exists. User
requested rejecting numbers, mixed words/numbers, band names, single characters,
and applying initcap.

Added src/genres.rs wrapping the existing VCW expansion table (OnceLock, no new
dependencies or VCW edits). Semicolon inputs are split; input/output checks
reject any Unicode numeric character and fewer than two alphabetic characters.
Reject artist-map names and case/punctuation-insensitive known artist credits.
Apply title casing via lowercasing then existing initcap, first-seen deduplication.
Examples: AMBIENT -> Ambient, post-punk -> Post-Punk, IDM -> Idm per initcap policy;
80s/2-step/etc rejected even if they could have expanded through genre map.

Capture collects known artist from collection SQLite plus public page artist
selectors, retains existing location exclusion, then filters/normalizes before
saving. Processing applies same checks to embedded + release genres, using
release/album artist, track artists, performers and remixers. When nothing valid
remains, remove GENRE rather than retaining the old invalid tag, allowing missing-
genre fallback. Research fallback uses same hygiene before assignment. Raw
Discogs imports remain unchanged as evidence. Unknown alphabetic words and
unrelated bands not known through credits or artist map can still remain;
this is not a curated allowlist or Discogs release confirmation.

README highlights, docs/REFERENCE.md and tag skill reference updated. Tests cover
numeric/Unicode numeric/mixed tokens, single letters/punctuation, artist/map
names, page/storefront artist exclusions, title casing, map expansion and removal
of fully rejected existing genres. Adjusted expected genre case in prior tests.
106 tests passed, one parent fixture ignored; formatting, strict offline all-target
Clippy and optimized offline release build passed. bcfunl v0.1.24 (built 2026-10-30T01:04:26Z).

Existing published server files and staging history were not bulk modified.
New hygiene applies to recapture and future processing; already-published folders
still skip. Remediation of the existing server's expanded genre vocabulary is
separate pending work. No production cleanup/deployment or live provider request.

## 2026-10-29: VCW genre review and break handoff

User asked whether existing genre sanitization from VCW was fully implemented.
Read-only inspection confirmed VCW Genres::normalize_all performs alias expansion,
semicolon splitting and first-seen case-insensitive deduplication; unknown values
pass through unchanged by design. Its built-in genre.dat has639 mapping rows
and is byte-identical to /data2/vripr/assets/genre.dat. VRipr sanitize_genres
also preserves unknowns. Neither implementation rejects numbers, mixed numeric
terms, artist/band names or single letters. VCW exposes custom table loading;
bcfunl currently uses the built-in table only. VCW Discogs provider merges actual
genres+styles before normalization; bcfunl previously fed arbitrary Bandcamp
tags through the normalizer during processing, while capture stored them raw
apart from location filtering. No live Discogs release confirmation is wired
into bcfunl. No VCW/VRipr files changed during inspection.

Current release remains v0.1.24. New local genre hygiene wraps, rather than
replaces, VCW mapping: rejects Unicode numeric characters and fewer than two
letters, excludes known release/track credit and artist-map names, title-cases
and deduplicates valid values. Applied at capture, processing, research fallback.
Unknown alphabetic words and unrelated unknown band names may remain. Restricting
to the user's approximately720 curated genres requires a supplied allowlist or
other explicit vocabulary policy; that curated list has not been inspected.
No allowlist or live Discogs confirmation implemented. Existing published server
files are unchanged and still skip processing; repairing their3000+ vocabulary
is separate work, not authorized or executed as a bulk modification here.

Last validation:106 tests passed, one parent fixture ignored, formatting, strict
all-target offline Clippy and optimized offline release build passed for v0.1.24.
This VCW inspection was read-only, with no additional code changes/tests. User
requested memory/status saved and a break. Stop here. No production download,
processing, cleanup, deployment, provider query or library retagging started.
Next session: await user's direction on curated vocabulary and/or acceptance
of v0.1.24. Preserve US English, no em dashes, johnsmith in all documentation
examples. Existing TrueNAS deployment and sequential workflow remain pending.

## 2026-10-30: shared NAS processing failures (v0.1.25)

User runs staging on media2026 and processing against a NAS share. Supplied logs
show ENOENT(error2), EINVAL(error22) at no-replace publication, audio mismatch,
and multiple verified copies. Asked for findmnt -T SERVERDIR -o TARGET,FSTYPE,OPTIONS
as asynchronous clarification; exact share type/options not yet supplied.
No remote mount/runtime inspection performed.

Found likely ENOENT race: existing-copy scan called file_type on unrelated root
entries including another worker's transient .bcfunl-process folder before name
filtering. Now names are checked first at artist/album levels; only relevant
paths probed. Added path contexts for directory traversal and publication sync.
Existing candidates with same nonzero (device,inode) identity deduplicate so
case aliases on some shares do not falsely count as multiple physical copies.
Genuinely separate verified directories and audio mismatches remain review
failures, unchanged. Skip logs now identify artist/release under parallel output.

Linux rename documentation confirms RENAME_NOREPLACE can return EINVAL when
filesystem lacks flag support. Publication catches only EINVAL/EOPNOTSUPP/ENOSYS,
exclusively creates a previously absent empty destination, then uses ordinary
rename to replace that owned reservation. Existing destinations (even empty or
symlink) remain refused. Failure cleanup uses remove_dir only, never recursive,
and only when reservation identity still matches. Share must support ordinary
empty-directory replacement; if not, publication still fails with original
payload retained. A brief empty reservation is visible before atomic completed
release rename, contains no manifest and is not completion. No unsafe unchecked
rename over a preexisting destination. NFS failure ambiguity leaves any nonempty
published output intact; next run can verify it.

Added regression tests for reserved publication, existing nonempty/empty/symlink
refusal, failed-reservation cleanup and source retention, unrelated scan entries,
and preserved genuine-duplicate review. 108 tests passed, one parent fixture
ignored; formatting, strict all-target offline Clippy and optimized offline
release build passed. bcfunl v0.1.25 (built 2026-10-30T12:38:28Z). docs/REFERENCE.md updated.
No production files/history changed or NAS test/deployment performed. Exact
share compatibility and ENOENT cause still need acceptance on media2026. Recommend
small retry without force/clean initially; do not interrupt user's active batch.
Source: https://man7.org/linux/man-pages/man2/rename.2.html .

## 2026-10-30: media2026 mount confirmed NFSv3

User supplied findmnt output for /mnt/FN042018/F8NAS1/music/ on media2026.
Mount target /mnt/FN042018, FSTYPE nfs, vers=3, rw, hard, proto=tcp,
rsize/wsize1048576, timeo600, retrans2, sec=sys. This is the CLIENT music path,
separate from the NAS-native /mnt/tankn4/F8NAS1/music path documented earlier.
NFSv3 is consistent with the observed RENAME_NOREPLACE EINVAL; v0.1.25's
reserved-empty-directory/plain-rename fallback is intended for this case.
Actual share publication test remains pending. Do not claim verified NAS support
from mount identification alone. ENOENT scan race and genuine duplicate/audio
mismatch reviews remain as described in prior entry. No code/deployment changes
or live processing performed while recording this information.

## 2026-10-30: processing performance, NAS-local Compose, push builds (v0.1.26)

User's limited MONO - OATH run failed with old EINVAL wording, with/without clean;
combined cleanup unexpectedly examined583 older successes. Exact script used
/data2/bcfunl/target/release/bcfunl, staging /data/bcfstag3, NFS client music
/mnt/FN042018/F8NAS1/music. User then confirmed executable v0.1.24, so the v0.1.25
NFS fallback had not been tested. No new evidence that fallback failed. Main
batch not interrupted or modified.

Found and fixed count check occurring after costly completion checks, allowing
NAS history scans after reaching --count. Now stops before additional checks.
ZIP extraction moved from server filesystem to local --output; prepared output
still resides under --serverdir for final publication. --timings (requires
--process) reports setup/query, history skip checks, per-release source/extract/
read_tags/prepare/verify_existing/genre/copy_tag_verify/artwork/manifest_sync/
publish/temp_cleanup/total, plus cleanup. Error stages retain timings. PCM decode
errors now include path, distinguishing source vs server corruption. Combined
process+clean now restricts cleanup to attempted release keys, not all filtered
history; standalone clean remains historical. Processing+cleanup failures retain
both errors. No verification weakening. Regression covers limited cleanup scope.

Read-only local NAS name listing:6235 root entries in0.024sec; not a per-file
metadata/decode benchmark. Disposable0.2sec FLAC profiling smoke tagged/published
with PCM/tag verification and all timings; about0.566sec total,0.408sec copy/tag/
verify in this tiny local case. Not an NFS speed benchmark. Real timing evidence
from user's workload still pending. Recommend --parallel2 initial comparison;
NAS-local execution removes NFS transport but not decode/verification costs.

User specified container runs ON NAS: final staging /mnt/backup; landing/music
/mnt/tankn4/music. These supersede earlier NAS paths in current guides/defaults.
Added Dockerfile (Rust slim-trixie builder, Debian trixie runtime), compose.yaml,
.env.example, strict .dockerignore and docs/CONTAINERS.md. Runtime installs all
utilities including sqlite3 for migration. Named build context ../vcw copies only
workspace manifest/metadata/types crates and license; no VCW source edits or
private cookies/databases copied into image. Runtime uses configured UID/GID,
read-only cookie/map mounts, persistent container-state HOME/Downloads/logs/
workflow lock; bind mounts use identical NAS host/container paths and refuse to
create missing datasets. Default command is help; jobs explicitly run via
compose run, with sequential maintenance wrapper supported.

User preferred SQL path updates rather than aliases. Added transactional,
idempotent scripts/migrate-nas-paths.sql: file_path /data/bcfstag3/ -> /mnt/backup/;
processed_path /mnt/FN042018/F8NAS1/music/ -> /mnt/tankn4/music/. Only anchored
prefixes change, history intact, prints changed counts. Stop jobs, copy payloads
and consistent database, make backup, then migrate. NOT run on real data.
Optional alias env settings removed in favor of simple identical bind paths.
scripts/test-migration.py verifies boundaries, NULLs, idempotence and history.

Added .github/workflows/build.yml on every push/manual dispatch, no version gate.
Checks out current repo + shunte88/vcw, optional read-only VCW_READ_TOKEN needed
if private, optional VCW_REF defaults main. Native Linux x64/macOS ARM64/Windows
x64 packages include maps/assets/skill/docs/licenses/migration/wrapper in tar.gz
to preserve execute bits. Linux job tests pipeline AND builds/runs container/tool
check. Native macOS processing uses reservation fallback; Windows natively supports
downloads/resources only, processing+clean explicitly point to Linux Docker.
Asked user's full-vs-initial Windows scope asynchronously, no reply; communicated
this initial limitation rather than claiming full native Windows processing.
Portability changes isolate staging lock in src/staging.rs, cfg Linux renameat2,
portable temporary dirs, conservative Windows stale aria staging cleanup (no
removal without live-PID check), HOME/USERPROFILE fallback. Windows source module
provides explicit unsupported processing/clean errors; no silent behavior change.
Hosted jobs use SQLite apt/brew/vcpkg setup and stable Rust with locked deps.

Local gates:109 tests passed, one parent fixture ignored; fmt, strict all-target
Clippy, optimized release build, shell syntax/wrapper tests, migration tests,
Compose config, workflow YAML/triggers/matrix, docs/link/style checks passed.
Minimal isolated Docker source context resolved with locked Cargo metadata.
Docker daemon socket access is denied here, so image build/run NOT exercised
locally; CI job is prepared to gate it on push. macOS/Windows hosted builds have
NOT run; native platform/runtime validation remains pending. Workflow NOT pushed
or installed remotely. No production NAS modification/download/processing/clean.
bcfunl v0.1.26 (built 2026-10-30T15:50:38Z). Release bumped at user's request after available local gates.
Public docs use johnsmith, US English, no em dashes, no personal references.
Next: user builds Compose on NAS or pushes workflow, supplies --timings output
for an actual workload; do not call remote/image gates green before execution.

## 2026-10-30: GHCR image publication alongside native builds

User requested providing a Docker image with the OS builds. Extended existing
push/manual workflow with publish-image job gated by the entire native build
matrix. Separate job alone receives packages:write; repository default token
permissions remain contents:read. Builds and smoke-tests publication image,
logs into ghcr.io using ephemeral GITHUB_TOKEN via password-stdin, pushes
sha-<full SHA> under lowercased github.repository. Only default-branch runs move
latest. Per-ref concurrency cancels superseded publication runs. OCI source and
revision labels retained. All pushes eligible, no version/release gate. Native
matrix failure prevents publication. Initial image Linux amd64, matching x64NAS.
Optional private VCW checkout token requirement unchanged.

Compose image now accepts BCFUNL_IMAGE override, default bcfunl:local. .env.example,
docs/CONTAINERS.md and README explain pulling published image instead of building
on NAS, private package read access/public visibility, and commit/digest pins.
VCW source not needed on NAS when pulling. Package visibility not automatically
changed; no Docker Hub credentials required. No actual registry push/deployment
performed in this environment. GitHub workflow execution still pending.

Validated workflow YAML, all-build dependency, scoped package permissions,
verification-before-push ordering, default-only latest guard, Compose local and
registry-image configurations. Mocked shell tests verify both default and feature
branches publish commit tag, only default publishes latest. No Rust/runtime
changes; release remains v0.1.26. Docker daemon inaccessible here, actual image
build/run/push and macOS/Windows hosted gates remain unverified until user push.
Memory/status saved. No production files/history or active jobs touched.

## Break handoff, 2026-10-30

User requested memory/status saved and a break. Stop work here.
Current local release v0.1.26 (built2026-10-30T15:50:38Z). User's confirmed
media2026 executable was v0.1.24, so their MONO EINVAL test did not exercise
v0.1.25's NFS fallback. No active user batch was interrupted or modified.

Latest local changes: processing --timings, local ZIP extraction, early count
stop, scoped combined cleanup, clearer decode path diagnostics, NAS-local
Docker Compose and every-push native/image build workflow. NAS final paths are
/mnt/backup staging and /mnt/tankn4/music landing. Migration SQL updates old
media2026 payload/published path prefixes transactionally; no live DB migrated.
Native Windows processing/cleanup remain unsupported, use Linux container;
native downloads/resources enabled. Linux/macOS processing paths prepared.

Local gates passed:109 tests + parent-invoked fixture, fmt, strict Clippy, release
build, wrapper/migration tests, Compose configuration, workflow YAML and branch
publication mock checks, minimal build context resolution, disposable FLAC
profile/publication smoke. Actual Docker daemon unavailable here. No image run,
registry push or GitHub matrix run verified. Container image publish job waits
for all OS builds and its own smoke check, publishes ghcr.io/<owner>/<repo>:
sha-<full SHA>, updates latest only for default branch. BCFUNL_IMAGE can select
published image, avoiding NAS-side build. Private VCW needs VCW_READ_TOKEN;
package visibility/read access may need account configuration after first push.

Next when user returns: await performance acceptance results or first workflow/
container run; investigate actual timing/CI errors as supplied. Do not claim all
platform/image gates green before hosted execution. No deployment, production
processing/cleanup, registry publication or library metadata rewrite performed.
Guides: docs/CONTAINERS.md, docs/TRUENAS.md, docs/REFERENCE.md. Use johnsmith in
all documentation/examples, US English, no em dashes, no personal references in
public guides. Nothing else started; paused for user's break.

## 2026-10-30: NAS Compose switched to prebuilt GHCR image

User returned and reported check-in is now building container; user removed macOS
and Windows from active workflow. Read current workflow and preserved user's
Linux-only changes. Updated compose.yaml to default ghcr.io/shunte88/bcfunl:latest
and removed local build/additional VCW context entirely. BCFUNL_IMAGE remains an
override for a pinned commit/digest. Updated .env.example to concrete published
image, removed unused BCFUNL_VCW_SOURCE. Preserved user's current example staging
/mnt/backup/bcfstag3 and music /mnt/tankn4/music, UID/GID, mounts and protection.
Updated docs/CONTAINERS.md from build to pull instructions, removed NAS-side VCW
requirement and stale macOS/Windows active-matrix claims. Existing .env may still
override image to bcfunl:local; user must update/remove that override if present.

Validated docker compose config as JSON: exact default image, no build section,
preserved mount/user settings and explicit image override. No Rust changes,
release remains v0.1.26. No actual pull/container launch performed here; Docker
daemon access remains unavailable. User reports remote build underway, not yet
verified by agent. Memory and status saved; no production data/jobs changed.

## Audio read retry and validation phase breakdown (v0.1.27)

User successfully deployed GHCR image through Dockge. Initial empty-dependency
error cleared by fresh stack; missing container-state bind directory created.
Default image help/exit0 is expected one-shot behavior. Acquisition first failed
/state/Downloads write permissions; after ownership correction all5 acquisitions
succeeded (MONO OATH, EMIKA Fountain Immersive, Martin Stephenson Cuckoos,
MONO Snowdrop, bvdub/Brock Van Wey Farewell). Cookies worked throughout.

NAS-local process on imagev0.1.26:3 published,2 failed source decode (MONO OATH
track10, Cuckoos track07) in temporary extracted files. Published destinations
use /mnt/tankn4/F8NAS1/music; staging /mnt/backup/bcfstag3. Publication measured
5–15ms; copy_tag_verify dominated52–188sec under5 concurrent releases.
Cuckoos original ZIP passes7z t; extracted track07 passes flac -t. Host and
container ffmpeg decode same copy successfully with identical PCM SHA256:
6935e16d4779d1a126264a81e93a6f99c0f90d02a9b328353015a4ecdbf4e30b.
A single-release processing retry published Cuckoos successfully in51.039sec,
47.541sec copy_tag_verify. Earlier transient cause remains unproven. MONO OATH
has not been confirmed successful by user yet. No evidence of archive corruption
or deterministic decoder incompatibility from these tests.

User requested bounded retry plus investigation of47sec validation. Added
retry_audio around read-only pcm_hash: one fresh subprocess retry after200ms,
cooperative cancellation, max2 attempts. Persistent read/decode failures still
error; successful unequal hashes still error without retry; writes/publication
not retried. Applied to both existing comparisons and prepared-audio validation.
No validation weakening or tolerance of malformed input.

copy_tag_verify now reports6 aggregate --timings components, also on failure:
copy, write_tags, source_pcm_hash, destination_pcm_hash, tag_readback, file_sync.
Hashing includes layout probes and retries. Current implementation decodes each
track twice into pcm_f64le for SHA256, so actual cost depends on audio duration/
resolution and hardware, not archive bytes. Need real NAS subphase timings to
locate47sec; no promise of full-album verification within seconds. Refactored
FLAC tag readback to export native comments directly without redundant ffprobe;
shared tag alias/multivalue normalization retained. Other formats unchanged.
Source/destination PCM verification remains mandatory.

Tests: transient failure succeeds on2nd, persistent failure stops after2 with
cause retained, success no retry; fast FLAC tags equal full-reader tags.111 tests
passed +one parent fixture ignored; fmt, strict Clippy and optimized offline
release build passed. Disposable synthetic FLAC publication/validation smoke
verified all6 timing fields (copy0s/write_tags0.103s/source_hash0.102s/dest_hash
0.101s/readback0.051s/fsync0s). Tiny local fixture, not NAS benchmark.
bcfunl v0.1.27 (built 2026-10-30T21:39:47Z). docs/REFERENCE.md updated. Memory/status saved.
No production files/history changed, no live NAS processing or image deployment
performed by agent. Updated image requires user's next successful push build.
Next: profile a NEW/unprocessed release (Cuckoos now quick-skips), inspect6 totals,
and test bounded retry as needed; don't conclude parallelism/hardware caused
the earlier glitch without evidence. Public docs retain johnsmith/US English/
no em dash. Dockge image support and current user-edited paths preserved.

## Latest handoff: ready for v0.1.27 acceptance test

User requested memory/status saved before testing. Current local binary is
bcfunl v0.1.27 (built2026-10-30T21:39:47Z). Bounded read-only audio validation
retry: max2 attempts,200ms pause, cancellation respected. Persistent errors or
unequal decoded hashes still block publication. --timings now separates copy,
write_tags, source_pcm_hash, destination_pcm_hash, tag_readback and file_sync.
FLAC tag readback avoids redundant ffprobe while retaining normalization.

111 tests passed, one parent fixture ignored; formatting, strict Clippy,
optimized release build and disposable FLAC/profile/publication smoke passed.
Actual NAS47sec phase still requires component timing evidence. Earlier
Cuckoos decode glitch did not recur on user retry; valid archive/FLAC and equal
host/container PCM hashes recorded above. MONO OATH success still unconfirmed.

Next action belongs to user: successful check-in image build, pull/redeploy
GHCR latest in Dockge, verify --version reportsv0.1.27, then run --timings on an
unprocessed release. Cuckoos is already completed and normally skips. Examine
component totals before changing hash strategy or verification policy. No
production data/history changes, live processing, cleanup or registry push
performed by agent. No further work started; waiting for test results.
