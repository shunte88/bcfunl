# Project Status

## Next-session reminder: acquire limit

On 2026-10-31 (America/New_York), remind the user that the acquire cron job is
still capped at `--count 100`. Review whether to raise or remove that limit.
Do not change it automatically. No proactive notification was scheduled because
this session has no general reminder/scheduling tool.


Updated: 2026-11-15 (America/New_York)
Local release: **v0.2.4**, lightweight collection writes with hard date window; Docker/live acceptance pending

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

## Break handoff: NAS acceptance and cron jobs defined

User confirmed v0.1.27 container acquisition of5 new releases succeeded. Process
then attempted5 (including previously failed MONO OATH), published5, skipped4,
failed0; cleanup removed exactly5 successful archives, retained0. Exit0.
MONO OATH now successfully published. No retry messages appear in supplied log;
success with retry enabled does not prove retries were exercised or explain the
prior transient decode glitches.

Real NAS timings now isolate the bottleneck: source/destination PCM hashes total
about66sec for Songs Of Sunderland,96sec American Dub Electronics,137sec From
The Roots Discomix,140sec Glory Be,211sec OATH. They dominate roughly90–94% of
copy_tag_verify. Copying about0.9–2.8sec, tag writes3.1–9.3sec, readback0.4–0.6sec,
fsync0.6–2.9sec. Publication3–12ms, cleanup11ms, processing wall242.961sec with
5 releases overlapping. Full decoded hashing, not publication/history checks,
is the measured slow phase. CPU vs storage/concurrency contribution not proven;
no hash-strategy change or validation weakening implemented.

User requested acquire/process be independently triggered. Explained two Dockge
stacks using the same GHCR image, same staging/database and absolute host paths
for cookies/data/state. Acquire command currently --count100. Process command
uses --process --clean --parallel2 --summary --timings. Both are one-shot jobs;
same staging lock prevents simultaneous acquire/process and a blocked job exits
for next scheduled retry. No daemon/service scheduler added inside image.

User says cron jobs now DEFINED on TrueNAS. Prior suggested commands cd to the
actual Dockge stack folder then /usr/bin/docker compose run --rm -T bcfunl.
Suggested acquire daily01:00, process hourlyminute20. Actual stack names, schedules,
Docker permissions and first unattended cron executions have not been inspected;
user's confirmation is of setup, not agent verification. Current observed NAS
paths: staging /mnt/backup/bcfstag3 and music /mnt/tankn4/F8NAS1/music; preserve
user-edited .env/configuration. No remote task changes made by agent.

User requested reminder TOMORROW2026-10-31 America/New_York: acquisition remains
limited to100 releases; review raising/removing --count100 after maintenance
acceptance. Saved prominently at top of MEMORY.md/STATUS.md for next session.
No general reminder tool available, so no proactive notification scheduled.
Do not claim reminder delivery or silently change the limit. Current release
v0.1.27; no code changes, build or new tests this turn. User requested a break:
stop here, no further processing, deployment, cleanup or cron changes started.

## 2026-10-31: rolling acquisition date window

User now runs one container with acquire/process arguments; acquisition uses a
rolling last-week window. Reviewed capture: inclusive UTC purchase-date matching
was correct, but pagination only stopped at eligible count or collection end.
Added before_purchase_window cutoff to collection_items older-page responses:
whole nonempty page must have known purchase dates strictly before fromdt.
Equality, mixed dates and unknown dates keep scanning. No cutoff for todt-only
or no bounds, nor artist search (separate request). Initial profile cache is not
used for cutoff because display can be curated. This relies on older-page API
chronology, as existing pagination does. No arbitrary cursor jump implemented.
Processing already uses bound optional >=/<= SQL predicates and DESC purchase
order; unchanged. Count still limits unfinished eligible attempts, not inspected
entries. User reminded of previous --count100; current deployed cap unverified.
Tests112 passed, one fixture ignored; fmt and strict offline Clippy passed.
No live authenticated scan or NAS container deployment performed. Release version
unchanged at0.1.27. See later entry for optimized build result if needed.

Optimized offline release build also passed for the rolling-window change.

## 2026-10-31: v0.1.28 release gates green

Bumped Cargo.toml and bcfunl entry in Cargo.lock from0.1.27 to0.1.28 for
rolling purchase-window pagination cutoff. Binary verified:
bcfunl v0.1.28 (built 2026-10-31T20:46:28Z).
Validation:112 tests passed,0 failed,1 parent-invoked fixture ignored;
cargo fmt --check; strict offline all-target Clippy; offline optimized build;
maintenance shell syntax and maintenance/migration Python checks all passed.
Date bounds remain inclusive UTC purchase dates. Rolling seven calendar dates
including today uses fromdt=today minus6 days, todt=today. User confirmed their
wrapper already uses this calculation. No wrapper, cron or NAS settings changed.
No live authenticated pagination test, container build/pull/deploy, commit or push
performed here. Existing GitHub push workflow must build the updated image before
NAS rollout. Older-page chronology assumption remains as documented above.
Memory and status saved after gates passed.

## 2026-11-01: staging lock recovery and scheduled-run handoff

User has one parameterized container invocation supporting acquire/process.
Deploying during acquisition was followed by a staging lock acquisition error.
Reviewed src/staging.rs: .bcfunl-staging.lock uses File::try_lock, an OS-held
lock, released when holders close/exit; existence of the file is not a stale
lock indication. Do not delete it to bypass contention, which could allow
concurrent jobs against different inodes.

User checked NAS host docker ps: no bcfunl containers running (only unrelated
services). fuser reported no holder for
/mnt/backup/bcfstag3/.bcfunl-staging.lock. Host flock -n on that file succeeded
and printed "Staging lock is free". Lock is available at time of test; original
holder/cause not established and no subsequent job startup verified. No file
deletions, process kills, container restarts, or environment changes by agent.

User will let jobs run on schedule and remain hands off until schedule settles.
Next session: review actual scheduled acquire/process logs and deployed version
if provided; do not assume successful unattended runs from the lock test alone.
Local release remains v0.1.28 with prior gates green. No code changes or new
build/tests this session. Previous --count100 limit was mentioned; current
configured cap still unverified. User requested memory/status saved and session
ended. No further work or remote actions started.

## Session close: cron Docker permissions resolved, acquisition running

User's TrueNAS scheduler jobs failed with EFAULT/exit1. Added persistent logs
revealed Docker socket permission denied before container startup. User confirmed
they had changed the cron execution user; switched both jobs to root. Container
still runs as configured non-root UID/GID. No Docker socket chmod required.
Earlier suspicion that compose run bypassed a command-based wrapper was not the
cause: deployed wrapper is an entrypoint and correctly receives acquire/process.

User supplied host ps output confirming root cron -> docker compose run --rm -T
bcfunl acquire -> docker-init -> /app/bcfunl acquisition running under container
user. Stack path /mnt/.ix-apps/app_mounts/dockge/stacks/bcfunl. This is a one-off
Compose run container, so Dockge main service need not show running. Single
service with parameterized entrypoint is working for launch; no split stacks
needed. Acquisition completion and process success not yet confirmed.

Deployed acquire wrapper currently uses --count100 --parallel8 --summary, with
--fromdt and --todt commented out intentionally during troubleshooting. User will
wait for completion, then restore dates. Thus current run has no date window.
Wrapper shown computes TODT using date +%F and FROMDT using
BCFUNL_ENDDT plus BCFUNL_WINDOW days ago; actual env values/timezone unverified.
When restoring, preserve inclusive UTC semantics and seven-date window of today
minus6 days through today. Earlier user confirmed intended math. Do not change
live environment during this active run.

Deployed process wrapper uses --process --clean --parallel8 --summary --timings,
configured collection/staging/music/template variables and extra argument pass
through. Logs /mnt/backup/bcfunl-acquire-cron.log and
/mnt/backup/bcfunl-process-cron.log. ps shows scheduler adds > /dev/null after
user's >>log 2>&1, so stdout may be discarded while stderr remains logged; avoid
claiming full combined capture without checking scheduler output settings.

Local release remains v0.1.28, previous validation gates green. No source edits,
new tests/build, deployment, remote commands, or NAS data changes by agent this
session. User requested memory/status saved and close until morning. Next:
review completion logs, restore date flags after run, verify scheduled process
result and deployed version if supplied. Leave jobs running without intervention.

## Next session priority: performance and Symphonia evaluation

User reports the job is still running and wants to identify the bottleneck next
session. Explicit preference: bring in Symphonia for tag encode/decode work,
following VCW, to simplify installation. Record as requested direction, not an
implemented or verified capability. Inspect actual VCW usage and distinguish
Symphonia audio decoding from tag reading/writing before choosing integration;
do not assume Symphonia provides tag encoding. Profile active acquisition versus
processing separately. Prior measured processing bottleneck was source/output
PCM decoding and hashing, not tag writes; retain mandatory audio/tag verification
and bounded retries. Determine whether native decoding can replace subprocesses
without changing comparison semantics. No dependency/code changes requested for
tonight. User says just make note and resume in the morning. Current job outcome
remains unverified; no intervention or live environment changes made.

## 2026-11-02: native media pipeline v0.1.29, gates green

User authorized Symphonia/native metadata implementation referencing VCW, with
multiple tracks and releases concurrently. Reviewed VCW export/tagging.rs native
Vorbis, ID3 and MP4 routines and its Symphonia ALAC tests. Important distinction:
VCW uses Lofty for metadata; available Symphonia0.6 code is ALAC test decoding,
not a ready all-format runtime pipeline. Also reviewed VRipr Symphonia0.5 decode
pattern. Implemented native adaptation in bcfunl without editing sibling repos.

New src/processing/decoding.rs streams Symphonia0.5.5 packets into reusable
RawSampleBuffer<f64>, hashes interleaved little-endian PCM SHA256 including rate
and channel count. Decoder-per-file, strict errors, cancellation per packet,
verify:true/final checksum, nonempty output, exact expected frame counts for
lossless formats; lossy presentation counts cannot equal padded decoder frames.
Existing bounded read-only retry retained. Symphonia0.5 AIFF SSND incorrectly
counts8 header bytes as audio: read-only virtual source patches chunk length,
never source file. Independent FFmpeg PCM oracle tests cover AIFF and other
lossless formats, high-resolution PCM, truncation and channel identity.

Replaced processing/tagging.rs subprocess/manual writers with Lofty0.25 native
Vorbis/ID3/MP4 backends following VCW's approach. Preserve repeated custom fields,
multiple artists/performers/sort names, artwork and unrelated metadata; consolidate
MP4 multi-value atoms. Read legacy RIFF INFO/AIFF text as well as primary tags;
synchronize alternate RIFF/AIFF/ID3v1/APE known fields on write. Clear discarded
genre/catalog entries, including case variants. Raw JPEG/PNG covers keep actual
format as folder.jpg/folder.png rather than transcoding artwork. No audio encode
or remux required. Existing staged publication, no-overwrite, cleanup, SQLite
history and verification policy preserved.

One process-wide Rayon pool capped at min(available CPU parallelism,8) shares
track tag reads, copy/tag/hash/readback/sync work, and existing-library comparisons
across --parallel release workers. Source/output hashes use bounded Rayon join.
No new CLI switch. Omitting --parallel still permits tracks within one release
to run concurrently. Track task results are joined before temp cleanup or publish;
subphase timings aggregate worker durations and can exceed wall time.

Dependencies added Symphonia (only supported formats/codecs), Lofty, SHA2, Rayon.
User ran cargo fetch on host because sandbox network unavailable; subsequently
installed/made 7z visible. Docker runtime no longer installs FFmpeg/flac, CI runtime
checks no longer require ffmpeg/ffprobe/metaflac. These tools remain independent
host-side fixture/oracle test dependencies. No Mutagen runtime dependency before
or after. README, reference, containers/TrueNAS guides and tag skill notes updated.
Existing README art and public johnsmith/US spelling preferences preserved.

Gates:117 tests passed,0 failed,2 ignored (existing parent-invoked fixture and
manual benchmark); fmt, strict offline all-target Clippy, optimized build, shell
syntax, maintenance/migration checks, git diff --check all passed. Manual optimized
benchmark passed:60sec stereo96kHz FLAC native0.580s vs previous subprocess0.603s,
rough parity, not a proven decode speedup/NAS benchmark. Independent synthetic
smoke /tmp/bcfunl-native-smoke.py published/verified/cleaned6 formats FLAC,WAV,
MP3,AIFF,ALAC/M4A,Ogg with PATH=/nonexistent; wall~0.017sec for tiny1sec fixtures.
Added multiple-track/two-release shared pool test and unknown-tag/genre/legacy
store regression coverage. Tests/fixtures stay disposable; production data untouched.

Cargo.toml/Cargo.lock bumped to0.1.29. Binary:
bcfunl v0.1.29 (built 2026-11-02T18:13:43Z).
No commit, push, Docker build, registry publish or NAS deployment performed by
agent. User's next push/image build and NAS --timings acceptance required; do not
promise NAS throughput from local fixtures. Current scheduled acquire/process
completion remains unverified; previous cron root-user fix and date flag/cap
handoff still apply. User previously planned restore dates after active run.

## 2026-11-02: v0.1.30 multiline lyrics and semicolon credits

User local acceptance:10 attempted,9 published/cleaned,1 failed Face Stabber with
"Multiline tag requires review". Read-only inspection of retained archive's FLAC
Vorbis headers found CRLF multiline UNSYNCEDLYRICS on11 tracks, not malformed
artist fields. Did not print lyric contents. Old newline validation was obsolete
with native writers. Removed newline/CR restriction on values, still reject NUL
and invalid keys; lyrics/comments/free text line breaks retained.

Added explicit semicolon splitting to shared parsed_names (used by artist_names
and split_fields). Expanded related credit fields composer/conductor/lyricist/
writer/producer/engineer/arranger/mixer/DJmixer/orchestra/ensemble. Native writer
normalizes cloned tags at boundary so semicolon genre/credit strings also work
for direct calls. Trim/drop empties/dedup credits, no splitting title/lyrics or
arbitrary commas/ampersands. Exact artist mapping behavior otherwise unchanged.

Regression covers all7 format variants (FLAC,WAV,MP3,AIFF,ALAC,M4A-AAC,Ogg),
semicolon multi-values for10 credit keys plus genre, title literal semicolon,
CRLF lyrics byte-for-byte, PCM unchanged.118 tests passed,0 failures,2 ignored;
fmt, strict all-target offline Clippy, optimized release build, shell syntax,
maintenance/migration scripts and diff whitespace checks all passed.

Actual retained Face Stabber track07 copied from original ZIP to disposable /tmp
staging, processed/published only to disposable /tmp destination, PATH=/nonexistent.
Success, exact original lyric bytes retained and source unchanged. Wall1.202sec
for this ONE track, not full album/NAS benchmark. Original production ZIP and
/data/bcfinal and live collection history not modified by agent. Failed archive
remains available for user's retry. Nine successes remain as reported by user.
No attempt to fix stylized Unicode Ombra catalog in ZHENG title or comma-separated
performer naming; outside this requested fix.

Cargo.toml/Cargo.lock now0.1.30; binary bcfunl v0.1.30
(built 2026-11-02T18:28:37Z). docs/REFERENCE.md and tag-rules updated. No image
build/deployment, commit or push by agent. Local test binary ready; NAS image
requires normal successful push build/redeploy. Native pipeline earlier changes
remain in workspace diff alongside this follow-up. Memory/status saved.

## 2026-11-02: v0.1.31 comma-separated credit lists

User explicitly requested comma-space artist lists (Six Strings release:
Amevicious, Phred Noir, Bardamu, Nici Palm, Rødel, Akrav) use same expansion as
semicolons. Updated shared credit parser with comma-space marker, disabled for
sort fields so surname commas survive. Whole artist-map matches precede splitting;
added single canonical Earth, Wind & Fire mapping to preserve existing band-name
regression. Unmapped literal-comma artist names now need artist-map exceptions.
Read-only split_fields preserves whole mapped identities for later expansion.
All callers use the shared parser: input normalization, performer/related credits,
known artists and native writer boundary. Existing primary-album-artist/folder
policy unchanged: Six Strings test uses primary Amevicious, all six track artist/
performer values retained. Sort fields can still use semicolon between names.
Titles/lyrics/free text keep commas. No arbitrary bare comma delimiter added.

New regression covers six native container types, all six artist values on tag
round trip, TrackArtist/Performer expansion through prepare_tags, surname sorts,
literal title comma and mapped Earth, Wind & Fire preservation.119 tests passed,
0 failed,2 ignored; fmt, strict all-target offline Clippy, optimized build and
diff whitespace checks passed. Version0.1.31 in Cargo.toml/Cargo.lock; binary
bcfunl v0.1.31 (built 2026-11-02T18:52:43Z). docs/reference and skill rules updated.
No live media/history retag/rename, deployment, push or commit performed. Previously
published Six Strings was already cleaned in user's run, so this code fix alone
does not retag or rename that existing server copy; no overwrite bypass added.
Earlier native media and0.1.30 lyric fixes remain in current workspace diff.

## 2026-11-02: v0.1.32 track-level source credits take precedence

User requested direct source tag dump for Six Strings. Read retained ZIP's FLAC
headers without transformations: track1 The Atonement ARTIST=Amevicious, Phred
Noir; track2 Strange People ARTIST=Bardamu; track3 Functional Freeze ARTIST=Nici
Palm, Rødel; track4 Satanisch ARTIST=Akrav, Solas. No TRACKARTIST fields present.
Solas is present on actual track4 despite absent collection-level artist roster.

User clarified individual source-track artists must populate TRACKARTIST and
PERFORMER with expansion. Audit found0.1.31 still seeded album primary on each
non-Various track, so correction required. prepare_tags now seeds first actual
track ARTIST then merges its remaining ARTIST/TRACKARTIST/remixer/title/explicit
guest values; falls back to album primary only without actual track names.
Preserved previous explicit guest/remixer ordering (David Holmes regression).
Does not prepend an unrelated album primary or whole collection artist roster.
Album-level ARTIST/ALBUMARTIST/folder policy otherwise unchanged.

New four-track regression reproduces source comma credits with full collection
roster as album artist; asserts exact TRACKARTIST/PERFORMER pairs per track,
including Bardamu alone and Akrav/Solas, then native FLAC round trip.120 tests
passed,0 failed,2 ignored; fmt, strict Clippy, optimized build and diff whitespace
checks passed. Cargo version0.1.32; binary bcfunl v0.1.32
(built 2026-11-02T19:07:40Z). Reference documentation updated. Native implementation
and earlier fixes still present in workspace diff. No live music/history changes,
image deployment, commit or push performed. User can test rebuilt local binary;
previously published files are not automatically retagged.

## Appointment break handoff

User requested memory/status saved before leaving for appointment. Current local
binary v0.1.32 (built 2026-11-02T19:07:40Z), with native Symphonia/Lofty pipeline,
bounded shared file workers, multiline lyrics, comma/semicolon credit expansion,
and corrected per-track artist/performer precedence.120 tests passed,0 failed,
2 ignored; formatting, strict all-target offline Clippy, optimized build and
whitespace checks passed. Exact four Six Strings combinations verified in
regression, not yet user full-release acceptance for0.1.32. Source ZIP unchanged.

Next: user tests local rebuilt binary, ideally --timings and inspection of
TRACKARTIST/PERFORMER on all four tracks. Track4 must include Solas; track2 must
be Bardamu alone. No unrelated album artist should be prepended. Already published
copies are not automatically retagged and no overwrite bypass was introduced.
All changes remain local workspace work; no commit/push or Docker image deployment
by agent. Do not start processing, cleanup or remote work during user break.

## 2026-11-02: v0.1.33 Discogs release enrichment and curated genre controls

User requested Discogs release lookups, especially missing catalog, preferred
Discogs genre/style over Bandcamp, and correct EBM descriptive alias expansion.
User clarified ONLY identity requirements are artist,title,country, stressing
curated genres grew from~700 to3000+. No server scrub authorized or performed.

New processing/discogs.rs reuses existing VCW Discogs provider and Client with
custom reqwest Transport (no curl/ureq/external tools). Provider net feature
remains off, bcfunl supplies existing reqwest HTTP. One shared Lookup per run,
all release threads share VCW60/min limiter,1-day disk cache, retry/timeout,
SignalClock Cancel wired to app shutdown. Reqwest one-worker shared Tokio runtime
(rt-multi-thread feature added), HTTPS api.discogs.com only, no redirects or
cookies,8MiB response cap, authheaders marked sensitive. Tokens loaded from
VCW_DISCOGS_TOKEN,DISCOGS_TOKEN,DISCOGS_API_TOKEN, never URLs/logs/cache/DB.
Token EXISTS in current execution environment (value not inspected/printed).

Searches artist/title then verifies candidate AND fetched detail artist/title/
ISO-normalized country (case/punctuation insensitive). Country missing/unknown
stays unresolved; no artist-only or title-only match. No trackcount/format
restriction per user's explicit clarification. Matching editions must agree on
normalized genre/style sets, otherwise no match. Catalog filled only when all
matched editions have same non-placeholder catalog; existing source catalog
preserved. Search provider's bounded candidate set applies, no claim to infer a
specific pressing beyond those identity criteria. Known aliases use primary
source artist; no fuzzy guessing.

Enrichment before prepare_tags/template/existing scan: confirmed genres/styles
REPLACE embedded/Bandcamp genres, do not merge polluted source vocabulary.
Catalog usable in template. Native tags DISCOGSID (possibly multiple) and
BCFUNL_GENRE_SOURCE store evidence. New nullable releases.discogs_metadata column
migrated with existing ALTER logic; manifest also holds evidence. applied=false
for equivalent existing server copy skipped unchanged, true on new successful
publication. Existing source releases.genre still Bandcamp capture evidence;
confirmed provider collection retained in discogs_metadata. No overwrite bypass,
existing completed rows still skip. User needs fresh destination/new staged
payloads for end-to-end enrichment acceptance; no retroactive file retag.

New --process --offline suppresses credential/network provider initialization.
Default processing uses lookup if token available; missing credentials warn once.
Unmatched/offline source genres are restricted to recognized shared/local map
inputs before normal hygiene/expansion. Bandcamp capture applies same recognition
filter. Artist-default resource fallback only passed during explicit --offline,
so online unresolved releases do not get artist-only estimates lacking country/
title confirmation. Offline library remains provenance-marked lower-confidence
existing feature. Explicit library schema checks still happen at startup.

Optional async user question asked whether unmatched should retain recognized
source genres or be empty; no answer received. After ample time defaulted to
recommended recognized-only and stated assumption in commentary. A later reply
can change this policy. Do not claim user explicitly selected it.

New data/genremap.dat overrides EBM, Electronic Body Music and descriptive
"EBM - electronic body music" => EBM;Electronic. Shared VCW built-in plus local
map OnceLocks, all inputs/outputs hygiene validated. Canonical short uppercase
acronyms (EBM,IDM,etc) retained instead of forced Ebm/Idm. Tests adjusted to IDM.
Local map is embedded; rebuild after edits. Sibling VCW/mcfunl not modified.
Source unknown alphabetic phrases no longer freely enter capture/published genre;
confirmed Discogs structured styles may pass unknown mapping, per trust policy.

Compose forwards all3 token aliases; .env.example token blank. Current Compose
contains user's acquire/process entrypoint and required BCFUNL_COLLECTION and
BCFUNL_TEMPLATE (preserved). Added johnsmith/default-template entries to example
so supplied example validates; did not change private .env. README/reference/
container guides/tag rules/dataREADME updated for online/--offline, metadata
precedence, strict matching, acronym handling and provenance. Existing art and
US English/no em dash/generic owner maintained.

Validation:127 automated tests passed,0 failures,3 ignored (parent subprocess
fixture, manual decode benchmark, manual live API check). Strict offline all-target
Clippy, fmt, optimized build, maintenance shell/Python checks, SQL migration script
checks, Compose --env-file .env.example config --quiet, diff whitespace passed.
Fixtures test3-field rejection, agreeing/conflicting editions/catalog ambiguity,
existing catalog preservation, publication catalog template and authoritative
genres/provenance, unknown-source rejection, EBM expansion and IDMacronym.
Processing unit tests explicitly --offline even if host has token. Native-only
6format PATH=/nonexistent smoke with --offline still publishes/verifies/cleans.

Manual live API smoke was attempted separately with token, FAILED because sandbox
cannot resolve api.discogs.com: failed to lookup address information/Name or
service not known. Not a successful live lookup; preserve this limitation in
reporting. Public Discogs developer docs also403 through web; provider behavior
reviewed directly from working VCW/mcfunl sources. No API credential printed.
No production music/history/data scrub, image build/deploy, commit or push by
agent. Provider fixtures passed; real host/NAS API acceptance still required.

Current Cargo.toml/lock version0.1.33; binary:
bcfunl v0.1.33 (built 2026-11-02T23:12:28Z).
All native0.1.29+ fixes and tag credit fixes remain in workspace changes.
Earlier tag dump exists /data2/bcfunl/test_tags.txt (10releases89FLAC); exact
/data2/test_tags.txt target outside writable roots, user given cp command.

## 2026-11-02: v0.1.34 Osees retrieval aliases and Worldwide country

User reports manual Discogs Osees/Off Course search hits with catalogDG011 but
processing said no confirmed match. Read-only checked /data/bcfstg DB: artist
Osees / The Oh Sees / Thee Oh Sees / Oh Sees, titleOFF COURSE, countryUS, no
catalog/discogs metadata. Read cached response under resources/discogs-cache:
structured API query artist=Osees&release_title=OFF%20COURSE returned pagination
items0/results[]. So original rejection occurred before country or catalog
checks. Do not claim the zero result was definitively caused by alias indexing;
API structured search differs from user's browser retrieval, and no fallback
existed. Existing matching also compared only primary spelling, ignoring aliases.

User clarified Bandcamp first-position artist flavor should stay primary and
music-server aliases enable browsing same band across name variants. Added
artist_aliases from reciprocal map entries only: expansion members/guests are
not automatically identity aliases. All4 Osees mappings mutually link and qualify;
Nick Cave membership in group expansion does not qualify without reciprocal link.
Query primary then all confirmed aliases, accumulate/de-duplicate release IDs
across these queries. Title-only retrieval fallback only if all structured queries
produce no valid matches. Candidate AND fetched detail still require recognized
alias, title, country. Source primary naming and server credit expansions kept.
Improved diagnostics show each query candidate count and rejection reason/ID.
Cached empty primary searches do not prevent new alias URLs/fallback; no cache
clearing required for those new queries.

User provided https://www.discogs.com/release/37546071-Osees-Off-Course,
attributed Osees, catalogDG011, countryWorldwide, and explicitly requested
Worldwide match any source country. Web page403 here, so these specifics are
user-provided, not independently fetched. Added explicit Worldwide wildcard at
candidate/detail checks, case-insensitive/trimmed. Preserves source ISO in tags/
filename, never invents a Worldwide ISO. Source-country unknown can match ONLY
Worldwide details; blank/unknown provider country does not match. Europe normalized
toEU, still requiresEU; UKvsUS and EuropevsGB rejected. Provenance now records
matched_artists, matched_countries and country_policy=exact-or-worldwide.
The previous detail check would have rejected Worldwide even had search found it.

Also removed an unintended fourth match blocker: genre/style differences among
identity-matched editions no longer discard a shared catalog. Catalog consensus
is independent; choose shared normalized genre/style intersection conservatively,
not a union that adds extra styles. Record genre_policy=shared-classifications.
Only artist/recognized alias,title,country-orWorldwide establish identity; no
trackcount/format restriction. Existing nonempty source catalog still wins.

Fixture regressions cover initial Osees0results, alias TheeOhSees match DG011,
title-only retrieval fallback, wrong country and unrelated artist rejection,
non-alias group members, worldwide withUS/GB/DE/unknown source and unchanged
sourcecountry. Catalog retained when editions differ on styles, shared genre
only.129 automated tests passed,0 failed,3 ignored. fmt, strict Clippy, optimized
build, diff whitespace and Compose example validation passed. No live Off Course
API acceptance test here; previous sandbox DNS restriction still applies.

Cargo version0.1.34; final binary:
bcfunl v0.1.34 (built 2026-11-03T00:27:47Z), still Nov2 local US/Eastern.
README/reference/tag rules updated. No production audio/history/cache writes,
retagging, scrub, Docker image deployment, commit or push by agent. Existing
published copies still skip; user needs fresh destination/new payload test to
observe new metadata. All earlier native media, credit and Discogs changes remain
in local workspace diff. Memory/status saved.

## 2026-11-02: v0.1.35 Discogs alternate-name asterisk comparisons

User's live v0.1.34 lookup now retrieves release37546071 but rejected candidate
artist "Osees*" as not an alias. User explains trailing Discogs asterisk marks
alternate-name attribution, requested strip during compare and case-insensitivity.
Root cause: shared processing key() only collapses whitespace and ASCII lowercase,
not punctuation. Earlier memory/docs had incorrectly described punctuation-
insensitive matching; corrected reference. No global key or title behavior change.

Added shared same_artist(): trim ends, remove trailing asterisks, collapse
whitespace, Unicode lowercase, compare nonempty. Applied to candidate artists,
fetched detail artist, reciprocal alias comparisons/dedup. Alias-map lookup strips
trailing marker from lookup key too; actual stored names/credits remain intact.
Internal asterisks and title punctuation retained; no arbitrary fuzzy matching.
Existing artist/title/country-orWorldwide validation, catalog consensus and source
primary naming preserved. Cached raw results re-evaluate under new comparator;
no cache clearing needed just for this fix.

Regression includes mixed-case oSeEs* search artist, OSEES* fetched artist,
Worldwide country and DG011 catalog, through existing alias/fallback test path.
Direct checks cover lowercase/Unicode RØDEL vs rødel, whitespace, internal star
not matching unstarred, empty-star name rejection and unchanged title-star rules.
130 automated tests passed,0 failed,3 ignored; fmt, strict all-target offline
Clippy, optimized release build and diff whitespace checks passed.
Cargo.toml/lock0.1.35; binary bcfunl v0.1.35
(built 2026-11-03T00:52:03Z), Nov2 local America/New_York.
Reference docs updated. No live API request, production media/history edits,
server retag, Docker deployment, commit or push performed by agent. User's next
rerun needed to confirm actual release metadata application; existing server
publications still skip rather than being retagged automatically.

## 2026-11-02: v0.1.36 simplify Discogs selection, composite countries

User reports country "UK & US" rejected despite sourceUS, multiple Discogs IDs
captured but missing metadata, requests first-list fallback rather than continued
candidate consensus. Wants deterministic closure and move on if no candidate,
asks whether agent more appropriate. Responded deterministic matching belongs in
executable; no tag-agent delegation required or started.

Read actual cached Osees responses and read-only history. Concrete catalog bug:
record37546071 labels[0].catno="DG 011", other matching records="DG011". Exact
BTreeSet catalog consensus incorrectly suppressed the catalog because of spaces.
Previous history did contain Rock/Krautrock from intersection, applied=true and
multiple IDs; do not claim no genres were applied. No live data/history modified.

Removed all-edition catalog consensus, style intersections, multi-ID pooling and
fetching further eligible candidates. Lookup now returns immediately on FIRST
candidate in provider result order passing artist/alias,title,country checks.
Its single record supplies catalog and full normalized genre/styles, exactly one
DISCOGSID. Preserve existing nonempty source catalog. Query primary/aliases/title
fallback sequence remains, but stops at first valid fetched record. If no match,
print reason and retain recognized source genres, move on. Network errors remain
reported/nonfatal enrichment failures. No weaker unrelated-artist match added.

Country matcher checks recognized whole names first, then listed components split
on &, comma, slash or semicolon, ISO-normalized. UK & US and UK&US matchGB orUS;
US, Canada acceptsCA; Europe & UK acceptsEU/GB; unlistedDE rejected. Worldwide
wildcard and source-country preservation unchanged. This avoids requiring an
agent to interpret simple country lists.

User explicitly requested tests WITH and WITHOUT catalog spaces. Regression
runs both candidate orders: DG011 first / DG 011 second, and reversed. Catalog
is populated from first record and exact chosen spelling preserved. Different
later catalog/genres cannot suppress first metadata. One chosen ID and source-
catalog precedence asserted. No forced catalog whitespace removal (other
catalogs legitimately have spaces). Composite country no-space variant tested.

Manual READ-ONLY cache replay test (new ignored test controlled by
BCFUNL_TEST_DISCOGS_CACHE env) parsed actual cached37546071 response through VCW
Discogs provider then applied selected metadata: catalog DG 011, genres
Rock;Krautrock, sole ID37546071, source countryUS retained. PASSED, no live HTTP
and no music writes. This proves actual response parsing/application, not live
search order or full production publication. Synthetic lookup tests cover first
candidate order and parser variants. Existing actual38597223 detail was not
fetched by agent; new composite rule allows host to fetch it on next run.

131 automated tests passed,0 failed,4 ignored (previous3 plus manual cache replay,
which was separately run and passed). fmt, strict all-target offline Clippy,
optimized build, diff whitespace, Compose example, maintenance shell/Python and
SQL migration checks passed. README/reference/tag rules updated to remove old
consensus/intersection policy and document first eligible record and composites.
No token values printed, no sibling repo edits, production retags/history/cache
writes, image deployment, commit or push by agent. Existing server copies still
skip unchanged; user needs fresh destination to test newly written metadata.

Current Cargo version0.1.36; binary bcfunl v0.1.36
(built 2026-11-03T01:17:19Z), Nov2 local America/New_York.
Memory/status saved. User preparing one final host acceptance pass; if unresolved,
move on rather than broadening arbitrary matching. Live acceptance still pending.

## Test dump refresh and catalog punctuation regression

User requested test-set tag dump at previous /tmp/test_tags.txt and catalog test
coverage with/without spaces and hyphens. Refreshed raw metaflac exports for10
published releases/10directories/89FLAC under /data/bcfinal to /tmp/test_tags.txt.
No audio writes. Extended existing first-eligible catalog regression to all12
ordered unequal pairs of DG011, DG 011, DG-011, DG - 011. Selected catalog spelling
preserved, one Discogs ID, full selected genres, existing source catalog precedence
asserted. Focused regression, strict all-target Clippy, fmt and whitespace passed.
Tests-only change; runtime and local release stayv0.1.36, no new release build or
live API/deployment performed. User's overall acceptance still pending.

## Generic track-title catalog extraction, v0.1.37

Stue requested a label-independent rule, not per-label mappings. Trailing
[album-catalog] or (album: catalog) track decorations now supply a missing
release catalog and are removed from track titles. Catalog tokens use existing
validation; spaces/hyphens compare equivalently. Existing confirmed catalogs win;
conflicting inferred codes require review. Ordinary remix/remaster suffixes and
references to another album are preserved. Literal catalog spelling is uppercased,
not guessed into a label name: Sex [Chiodi EP-Ombrantl033] becomes TITLE=Sex and
CATALOGNUMBER=OMBRANTL033, used by album decoration and catalog_num templates.

133 tests passed, 0 failed, 4 ignored. Formatting, strict all-target offline
Clippy, release build and diff whitespace checks passed. Binary v0.1.37 built
2026-11-03T01:42:48Z. A temporary copy of the actual SLF IMG track published
without external tools and independent metaflac readback confirmed title,
catalog and catalog-bearing folder. Original music remained untouched.
No deployment or live library retagging performed. Existing publications still
skip unchanged; use a fresh destination for acceptance testing.

## v0.1.38: year-qualified edition notes in Discogs lookup

Generic search-only title cleanup strips trailing year-qualified Remaster,
Remastered, Reissue, Reissued, RM/RE notes, including re-issued/re-mastered,
parentheses, brackets, dash suffixes, and reversed year order. Applied both to
search query and candidate/detail comparison. Source ALBUM, display titles and
folders unchanged. Requires a 1900-2099 year plus an edition keyword and only
recognized modifier words; remix/live/bonus-track text remains significant.
Mclusky (2026 Remaster) regression and fixture query/assertions pass. Existing
artist/country rules and first eligible result selection unchanged; cannot prove
the precise pressing from normalized title.
Research: Discogs Title guidelines specify main title; Format guidelines 6.1.5
place distinguishing non-title edition information in format free text. User's
38076354 release page could not be fetched in this environment, so no claim of
live matching that record. No credentials printed or live library edits.
134 tests passed, 0 failed, 4 ignored; fmt, strict offline all-target Clippy,
release build and whitespace checks passed. Version bumped to v0.1.38.

## v0.1.39: stale preorder flags after 30 days

Refreshed /tmp/test_tags.txt from 10 test releases / 89 FLAC, read-only metaflac.
User requested stale prerelease override for releases over 30 days old. Shared
collection::stale_prerelease now permits any track count when all available
release dates are strictly more than 30 UTC calendar days old. Existing confirmed
single-track-before-today exception retained and communicated. Exactly 30 days,
missing dates, and conflicting recent/future dates remain blocked for albums.
Capture, preparation and refreshed download links share this predicate. Keep
Prepared.released_single restricted to actual singles so album age evidence is
never misinterpreted as single-track evidence during refresh. Processing strips
explicit prerelease markers from all qualifying tracks, not just track zero.
Captured source titles retained; no publication overwrite or live changes.
134 tests passed, 0 failed, 4 ignored. Boundary/conflicting dates, album capture,
refresh, and multi-file marker cleanup covered. fmt, strict offline all-target
Clippy and diff whitespace passed; optimized release built as v0.1.39.
No deployment or live acquisition performed.

## Acceptance and requested repository publication

Stue reviewed the latest ten-release tag dump and confirmed the tags look good.
Metadata refinement is closed for this session; do not broaden matching or add
more speculative rules. Current release v0.1.39, 134 automated tests passed,
0 failed, 4 ignored; fmt, strict Clippy and optimized build passed. User explicitly
authorized committing and pushing the accumulated native media, tagging, Discogs,
genre, catalog/title and stale-prerelease changes to origin/main. Commit/push
completion must be verified from Git, not inferred from this request.

Publication blocked by session filesystem permissions: git add -u failed with
"Unable to create /data2/bcfunl/.git/index.lock: Read-only file system". The Git
metadata directory is read-only in this session. No commit or push occurred.
Application files and memory/status remain saved. Resume commit/push from a
session with writable Git metadata, or run the supplied host commands.

## 2026-11-03: native archive/resource I/O implementation in progress

Stue requested removal of external 7zip with shared concurrent Rust archive
handling, and no runtime subprocesses other than aria2. Explicit follow-up:
SQLite stays as is. No SQLite bindings/linkage changes were made. Previous
accepted work is now committed by host as f8f9936.

Working Cargo version is 0.1.40, NOT yet built or validated. Cargo.toml adds zip
9.3 (only deflate-flate2 feature), flate2 with pure Rust backend. Existing sha2
is reused. New src/archive.rs centralizes iterative sorted symlink-refusing
walking, cancellation-aware 128 KiB copy, SHA-256, ZIP preflight and streaming
parallel extraction. Reject unsafe paths, links/special files, raw/normalized
duplicate names, file-directory collisions, nonempty destination, CRC/size
errors. Limits 100k members, 64 GiB/member, 1 TiB total declared size. Stored and
Deflate + ZIP64 supported; other compression/encrypted ZIPs fail retaining source.
Uses chunks with independent File/ZipArchive per chunk, at most pool size chunks
per release. No unsafe archive API. zip9.3 chosen because older8.6 hides raw
duplicate names and lacks has_duplicate_names; new name API returns Result<Cow>.

Shared existing bounded pool moved to src/workers.rs, reused by extraction and
audio jobs. Processing no longer calls 7z. Command runner now cfg(test) only.
Resources replace gzip child with MultiGzDecoder and sha256sum with native SHA.
SQLite remains system-linked. Optional maintenance shell wrapper still needs
flock; Docker retains util-linux and optional sqlite3 inspection tool. Removed
explicit 7zip/gzip/coreutils dependencies from Docker/CI, updated runtime docs.

Added archive tests for concurrent Stored/Deflate/ZIP64, Unicode/nested/empty
files, traversal, symlinks, collisions, corruption/truncation, duplicate names,
no overwrite and known SHA vector; processing ZIP fixture now native ZipWriter.
Gzip test now native GzEncoder, concatenated members and truncated/corrupt CRC.
None of the Rust changes have been compiled yet: sandbox DNS cannot resolve
crates.io and ZIP/typed-path are not cached. User has an asynchronous request
to run cd /data2/bcfunl && cargo fetch on host; Cargo.toml is ready. Cargo.lock
still old until successful fetch. Do NOT report .40 as ready or gates green.

Passed so far: cargo fmt --check, git diff --check, maintenance shell syntax,
Python maintenance/migration checks, Docker Compose example validation.
Next: after host fetch run cargo test --offline, fix API/test/compiler errors,
Clippy -D warnings, release build, then /tmp/bcfunl-native-archive-smoke.py.
Smoke creates TWO disposable Stored/Deflated ZIP releases, concurrent native
publication+clean and gzip resource import with child PATH=/nonexistent. Script
not run yet. Confirm real archive extraction optionally using read-only source
and /tmp outputs. Update memory/status with final evidence. No live staged files,
music, history or Docker deployments modified.

## Native archive/resource I/O completed: v0.1.40

Host fetched dependencies. Implementation now compiles and is ready for host
acceptance. 138 tests passed, 0 failed, 4 ignored. Strict all-target offline
Clippy, fmt, release build, diff whitespace, maintenance shell/Python checks,
SQL path migration tests and Docker Compose example validation passed.
Production thread import corrected after Clippy caught its accidental cfg(test)
placement. No extraction or import logic fixes were needed after compilation.

Binary: bcfunl v0.1.40 (built 2026-11-04T02:11:10Z), Nov3 local.
PATH=/nonexistent end-to-end check passed: two disposable releases (Stored and
Deflate ZIP, two FLAC tracks each) processed concurrently with --parallel 2,
published, verified and cleaned. Native gzip artist/master import and hashes
passed in the same environment. Smoke fixture initially put ZIPs outside the
user staging subtree; cleanup correctly retained them. Fixed ONLY the temporary
fixture paths under stage/johnsmith and repeated successfully. No production
staging, music, cookies, history or snapshots changed. Test generator used host
FFmpeg/Python to prepare fixtures; the tested executable had no tools on PATH.

ldd confirms libsqlite3.so.0 is still dynamically linked, per Stue's explicit
instruction. Archive code is zip9.3 + flate2 pure Rust backend; SHA-256 reuses
sha2. No runtime 7z/gzip/sha256sum calls remain; other command helpers and audio
fixture tools are cfg(test). Optional maintenance shell wrapper still uses
flock, and Docker retains optional util-linux/sqlite3 admin utilities. Runtime
image no longer explicitly installs 7zip/gzip/coreutils. SQLite unchanged.

No NAS throughput benchmark, real purchased ZIP acceptance, Docker image build,
commit/push or deployment performed. No purchased archives were available in
the checked local staging roots; synthetic ZIP/ZIP64 and native pipeline fixtures
provide local coverage. Existing no-overwrite/publication guards remain intact.
Do not claim a measured NAS speedup. /tmp/bcfunl-native-archive-smoke.py available
for rerun. User can now run their staged batch on the rebuilt executable.

## Session closed, 2026-11-03

Stue acknowledged the v0.1.40 implementation and requested memory/status save
and session close. Native archive/resource changes are complete locally: 138
tests passed, 0 failed, 4 ignored; Clippy, fmt, optimized build and relevant
maintenance/migration/Compose checks passed. Concurrent ZIP publication/cleanup
and gzip resource import passed with the executable PATH=/nonexistent. SQLite
remains unchanged and system-linked. Aria2 is the only production executable
invoked; the optional maintenance shell wrapper still requires flock.

Next session: read this handoff, then host/NAS acceptance and throughput review
if requested. No measured NAS speedup or container deployment claimed. Changes
remain uncommitted in this session; no commit, push or deployment performed.
Previous accepted changes were committed by the host as f8f9936. The .git
directory remains read-only to this session. Do not reopen metadata refinement
or change the acquire count cap without a new request.

## Build workflow lockfile failure and readiness gate

User reports GitHub builds failing for a day with cargo +stable build --locked
--release requiring a Cargo.lock update. Local identical locked/offline build
passes. Both CI jobs previously checked out configurable VCW_REF/default main,
allowing dependency manifests to drift from the local lockfile. GitHub API and
remote DNS inaccessible here, so the exact VCW revision of failed runs was not
retrieved. Reproduced the same failure in a disposable clean CI/Docker source
layout by changing ONLY VCW workspace version; the clean pinned layout resolves
with Cargo.lock unchanged. Evidence supports dependency drift, not a reason to
remove --locked.

Added .github/vcw-revision = 69fa79c5f2d361faab2229a710fa83ed46edce23 (local
HEAD and cached origin/main). Both jobs read this pin; VCW_REF variable no longer
used. No sibling modifications. Cargo.lock stays unchanged. Added explicit
docker/setup-buildx-action@v3 to both jobs. New scripts/check-build.sh is run
by CI and local readiness: pinned clean VCW sources, fmt, strict locked Clippy,
locked release build and tests, Docker build/runtime checks, Linux packaging
under target/package. New standard-library scripts/test-build-workflow.py
checks command arguments and gate failure propagation with disposable stubs,
including pinned/dirty sources, build/Docker failure, successful packaging,
sudo-only Docker access and absent sudo credentials. Workflow runs those tests.
AGENTS.md and docs now REQUIRE exercising this workflow script for readiness,
reporting a blocked Docker gate explicitly. Configuration-only change: version
remains v0.1.40. No commit/push/deployment by agent.

Validation: actual shared gate ran fmt, strict locked Clippy, locked release
build, all138 tests (4ignored) and version successfully. Docker initially lacked
Buildx. User installed Ubuntu docker-buildx and confirmed Docker info works
with sudo. Session has no-new-privileges flag and cannot use sudo/daemon socket.
Script now preflights access, uses sudo -n for Docker ONLY when necessary,
never elevates Cargo. User host command: cd /data2/bcfunl && sudo -v &&
sh scripts/check-build.sh (do not run whole script with sudo). Buildx version
now installed locally. Complete actual container build still BLOCKED for agent;
await host script result before claiming full CI/image readiness. Packaging
commands tested with stubs and real tar; no actual container success claimed.
Workflow YAML and every shell block checked; both pinned checkouts, Buildx
setup and shared readiness/test entrypoints asserted. New gate regression tests
and shell syntax/whitespace checks passed. GitHub hosted run not observed.

## Host full build workflow acceptance

Stue added the host user to the docker group and ran scripts/check-build.sh
successfully. Reported final output: image bcfunl:ci built with SHA256
0aa7a99b008c3f03454cc6950bb4be4cc55a839855def924f6964e1b6aa7a26b;
container binary bcfunl v0.1.40 built 2026-11-04T02:49:40Z; aria2c, flock and
sqlite3 found; "Build workflow checks passed: locked native build, tests,
container and package." This closes the previously blocked LOCAL container
and packaging readiness gate, based on host output. Sudo workaround no longer
needed for this host user. Hosted GitHub Actions rerun, GHCR publication, NAS
deployment and NAS processing acceptance remain unverified. No agent push or
deployment performed.

## Session close: hosted Docker build clean

After the full local workflow passed, Stue reported the GitHub Docker build
was clean. Hosted build success is user-confirmed; agent did not independently
inspect the Actions run. GHCR publication/pull was not separately confirmed.
Next session: NAS deployment followed by in-place processing/performance tests.
No further implementation requested. Preserve the pinned VCW dependency and
shared build.yml readiness gate. Memory/status saved at user request.

## 2026-11-04: bcstag4 NAS logs and legacy-title matching fix, v0.1.41

NAS prior staging bcstag3 collection.sqlite3 was corrupt. Recovery integrity
passed and yielded8737 rows, but three usernames and other fields were visibly
damaged. No old backup remained; Stue chose new bcstag4 with fixed image, small
batches and no clean. Original corruption cause remains unknown; healthy ZFS
pool status does not establish application data integrity. Agent performed no
NAS database replacement/deletion or server file changes. New stack renamed
bcfunl4; initial service/path permission problems resolved by user.

Read /data2/bcfunl-acquire4-cron.log and process4-cron.log. Two old service-name
errors precede successful10/10 acquisition. Process runs: 10 attempted,2published,
5skipped,3failed,wall53.618s; next 10attempted,3published,10skipped (includes
history skips),4failed,wall43.405s. No SQLite malformed errors in these logs;
not an independent DB integrity certification. No cleanup ran. Discogs token
missing in new container, so enrichment disabled. Stue has now added Discogs
requirements to .env and is running another small acquisition, then processing.
Ensure Compose forwards token; current image still v0.1.40 until new build.

Three repeat failures traced read-only using mounted NAS plus fresh ZIPs: old
server TITLE fields have lossy Unicode substitutions (Gholü -> Ghol?, Nicolás
-> Nicol?S, Karassimeon decorative symbols -> ?), while disc/track counts and
positions agree. Existing matcher required exact normalized TITLE before hashes,
causing false rejection. Removed title precondition only: still require unique
disc/track positions, matching count/duration and full decoded PCM hashes for
every file. Added count/position/hash mismatch diagnostics. Never retags or
replaces existing files. Version bumped Cargo.toml/lock to v0.1.41.

139 tests passed,0failed,5ignored. New regression covers legacy title damage,
no writes, wrong disc and duplicate counts; existing differing-audio/no-overwrite
test passes. New ignored read-only acceptance fixture consumes BCFUNL_TEST_ARCHIVE
and BCFUNL_TEST_DESTINATION and extracts into private /tmp only. All THREE actual
NAS release comparisons passed with release-profile test binary (Karassimeon
1.38s, Fluke6.90s, FaceStabber22.08s on myth2017 mounted reads, NOT NAS benchmarks).
Debug acceptance run was cancelled for slowness, then optimized tests completed.
Read-only comparison details /tmp/bcfunl-existing-compare.txt; optimized test log
/tmp/bcfunl-existing-acceptance-release.log. Original ZIPs, DB and server files
unchanged.

Separate REAL damaged existing audio: /mnt/tankn4/F8NAS1/music/Zheng/Zheng [DE]/
01-04 - Love By Nature - [Ombra Intl036].flac. Native validation decodes8192 fewer
frames than header. Independently confirmed flac -t errors (lost sync/bad header)
and FFmpeg decode failure on corresponding /mnt/FN042018 mount. Fresh same-track
FLAC extracted from bcstag4 album3878676280 ZIP passes flac -t. No automatic
replacement/quarantine performed. Keep protection and source; needs user
review/quarantine/reprocess separately. Do not weaken decoder validation.

Fmt, locked strict Clippy and locked optimized build passed; binary v0.1.41
built2026-11-04T05:13:41Z. Build workflow regression checks passed. Actual shared
readiness script attempted and blocked at Docker preflight: session cannot access
Docker daemon or sudo. Container gate NOT green for .41; host must run
sh scripts/check-build.sh before push/deployment. No agent commit/push/deploy.

## NAS v0.1.41 with Discogs enabled: latest log review

Read /mnt/backup/bcfunl-acquire4-cron.log and process4-cron.log on Nov4. Logs
append prior .40 failures; isolate latest .41 block beginning line257. Latest
process run successful:10attempted,2published,21skipped,0failed,wall194.051s.
Eight existing releases verified in this attempted batch plus13 earlier-history
skips. Karassimeon, Fluke Second Bite Remixes and Oh Sees Face Stabber now
verify/skip on NAS, confirming the legacy-title fix. Zheng also now verifies;
log alone does not establish what happened to the previously damaged file.
A Saint Abdullah/Eomac alternate candidate has one audio mismatch but another
candidate verified and release skipped, not a processing failure. No cleanup
or SQLite corruption errors shown in the latest block.

Discogs credentials now work: selected Fluke38310132 (catalognone), Martyn
37388619 (3024MAR2LP), Oh Sees14055018 (CF-116), Actress38460711 (ZENDNL322).
Remaining metadata gap: Concrète Waves joint credits Actress And Suzanne Ciani
and Actress, Suzanne Ciani rejected against primary artist Actress. User earlier
closed metadata refinement; no new matching expansion made from log review.
Published Future City Punks Take My Breath Away and Actress Concrète Waves.
Latest acquisition:9downloads succeeded, Eve Maret Live At The Reading Room
rejected for future publication date; exact date not independently verified.
No code change needed for this review. NAS image .41 and small-batch processing
success observed from user logs; full host .41 check-build command not separately
observed. Retain no-clean test posture until user approves wider run.

## v0.1.42: joint Discogs credits and explicit performer/composer enrichment

User explained Concrète Waves is credited Actress, Suzanne Ciani on double
vinyl (A-D) and Actress And Suzanne Ciani on digital21tracks; Darren J.
Cunningham is a displayed performer credit/Actress real name. User clarified
this reference is for additional PERFORMER import, then explicitly requested
COMPOSER too. No artist real-name alias is inferred from performer credits.

Lookup now keeps all source ALBUMARTIST values for identity (including repeated
fields), compares complete joint credits across separators/case/order/trailing
stars, and checks full fetched release artist arrays instead of only provider's
first artist. Unique one-to-one member matching through confirmed reciprocal
aliases required: partial/extra/unrelated artists rejected. Existing Osees alias
group and mapped literal Earth, Wind & Fire remain supported. Query primary
artist/naming behavior remains, no format/track-count restriction added.

VCW provider currently drops extraartists and all but first release artist.
Without modifying sibling or making extra requests, bcfunl retains the same
cache Arc used by its provider Client and parses the already cached selected
release body. Supplement full artist validation and explicit extraartist roles.
Album-wide Performer credits append/deduplicate PERFORMER; Composer and
Composed By/Composed-By append COMPOSER. Displayed ANV preferred when present,
otherwise name. Existing values retained, no substitution of album artist.
Track-scoped credits excluded because physical side/digital track mapping is
not proven. Producer-only/other roles not guessed as performer or composer.
Credits included in existing discogs_metadata/genre-source provenance. Existing
verified publications still skip without retagging, so new metadata acceptance
needs a fresh destination. Original live music/history/archives untouched.

140 automated tests passed,0failed,5ignored. Synthetic end-to-end provider
fixtures cover two-artist array, And/& variants and reversal, case/asterisks,
repeated source ALBUMARTIST, physical2LP A-D and21fileMP3 releases, explicit
performer/composer roles, ANV, deduplication, existing-credit preservation and
track-scoped/unrelated exclusions. Prior genre/country/title/audio guard tests
pass. Locked strict Clippy, fmt, locked release build, workflow gate regression
checks and diff whitespace passed. Binary v0.1.42 built2026-11-04T06:22:09Z.
Actual sh scripts/check-build.sh attempted and BLOCKED at Docker preflight due
to session socket/sudo restrictions; full .42 image readiness not claimed.
Host command remains sh scripts/check-build.sh before deployment. No live
Discogs API acceptance of these actual two release details performed here;
source cached searches and source FLAC metadata were inspected read-only.
No agent commit/push/deploy or sibling code changes.

## v0.1.43: explicit conductor credits

Stue requested conductor support for classical/orchestral collections. Reused
the existing album-wide Discogs role extraction: Conductor and Conducted By
(including hyphenated spelling) append/deduplicate CONDUCTOR, preserving prior
values and displayed ANV. Added conductor provenance alongside performer and
composer credits. No extra HTTP, dependency, sibling or naming changes.
Track-scoped conductor credits remain excluded pending proven track mapping.
Extended joint-credit fixture covers conductor ANV, duplicate role variants,
existing credits and side-specific exclusion. 140 tests passed,0failed,5ignored;
locked strict Clippy, fmt, optimized build, workflow regression tests and diff
whitespace passed. Actual shared readiness script attempted and blocked by
Docker daemon access; full .43 container gate remains pending host execution.
Cargo.toml and Cargo.lock bumped together to0.1.43. No production file writes,
commit, push, deployment or live classical-release acceptance performed.

## Latest NAS process-only review and numeric Discogs identity fix, v0.1.44

User supplied updated /mnt/backup/bcfunl-*4-cron.log after process-only run.
Latest v0.1.43 block:10attempted,0published,33skipped,0failed,wall95.613s;
10 current verified-existing skips plus23 history skips. No acquire rerun
needed for metadata changes. New performer/composer/conductor tags were not
written by this run because existing publications skip unchanged. User saw
fresh-landing note and is changing destination for acceptance.

Found genuine regression introduced by raw joint-credit validation: MONO OATH
and Snowdrop rejected at fetched-detail stage although logged parsed artist,
title and Japan/Worldwide country match. Read-only cached actual responses
confirm raw artist name Mono (7), empty ANV, artist id208568. VCW provider
parsed it to Mono, but raw validator compared numeric disambiguator literally.
Fixed shared same_artist comparison to reuse vcw_metadata::positions::clean_artist
after trimming trailing star. Meaningful (Live) qualifiers and title numeric
parentheses remain significant. No sibling edits or source-name rewriting.

Also canonicalized Various and Various Artists for comparison; latest run
rejected Various Artists - American Dub Electronics, candidates38334492 and
38202567 both artistVarious. User asked which VA release and is researching
it; supplied exact title and both candidate IDs. No other alias broadening.

Added full lookup regression for Mono (7) Japan, Mono (7)* Worldwide and
Various compilation; negative qualifiers/other artists/title cases. 141tests
passed,0failed,5ignored. Locked strict Clippy, fmt, locked optimized build,
workflow regression and whitespace checks passed. Version0.1.44 in Cargo.toml
and Cargo.lock; binary built2026-11-04T07:05:22Z. Actual shared readiness
script attempted and blocked by session Docker preflight. Host must still
run sh scripts/check-build.sh before deployment; no .44 container success
or live Discogs lookup claimed. No agent push/deploy, NAS/history/tag writes.

## v0.1.45: bare VA compilation alias

Stue explicitly requested VA in the Various list. Added VA -> Various Artists
to shared data/artistmap.yaml (case-insensitive). Discogs comparison now reuses
map entries whose sole canonical value is Various Artists, so matching works
both directions for VA/V/A/etc rather than source-only expansion. No generic
artist alias loosening. Existing artist-specific track credits retained; album
artist/folder canonical Various Artists and COMPILATION=1. New preparation test
covers all7 spellings and per-track credits; lookup regression includes VA/va
as source and VA/V/A as provider artist.

142 tests passed,0failed,5ignored; fmt, locked strict Clippy, locked optimized
build, workflow regression and whitespace checks passed. Binary v0.1.45 built
2026-11-04T07:22:56Z. Actual shared readiness command attempted, blocked by
session Docker daemon access; host image gate still pending. No commit/push,
deploy or live data writes performed. NAS ./data bind mount overrides bundled
map, so synchronize the mounted artistmap.yaml including VA entry when deploying.
Docs/tag rules updated.

## v0.1.46: --varioustag compilation grouping

Stue requested --varioustag accepting {albumtitle}, {label}, {albumartist},
default {albumartist}, to organize VA compilations under a label such as Soul
Jazz Records. Implemented typed Clap enum, process-only explicit option,
resources conflict, validated default/choices. {albumartist} preserves existing
Various Artists behavior. {albumtitle} uses cleaned undecorated album title;
{label} uses common embedded LABEL, or missing labels supplied from confirmed
Discogs metadata, or proven label-compilation storefront evidence. Missing
label falls back to Various Artists with a message. Existing embedded labels
preserved; conflicting embedded labels require review for label selection.

Selected grouping populates ARTIST/ALBUMARTIST/sort aliases and artist template
fields/folder. Compilation bool is captured before replacement; COMPILATION=1
remains under all policies. TRACKARTIST/PERFORMER/composer/conductor credits
remain independent. No label/title inserted as missing performer, and chosen
album-title grouping does not filter a legitimate genre such as Dub. Only VA
classifications affected; noncompilation artist behavior unchanged. Existing
files are not moved/retagged automatically. Source release artist in history
remains unchanged. Use fresh landing to test.

Reused existing Args as process_one context (timings + varioustag), avoiding an
eighth argument/new options layer. prepare_tags gets explicit policy; baseline
test callers use AlbumArtist default. Discogs Match now supplies label to LABEL
only when absent, no extra request/DB column, label included in provenance.
README, reference and tag rules document parameter and fallbacks.

145 tests passed,0failed,5ignored. New tests cover all policies, VA flag and
track credits, genre preservation, missing-label fallback, noncompilations,
CLI validation/default, confirmed label fill/preservation, and actual offline
CLI FLAC publication under Soul Jazz Records with readback and PCM equality.
Fmt, locked strict Clippy, locked optimized build, workflow regression and
whitespace checks passed. Binary v0.1.46 built2026-11-04T07:37:50Z. Actual
readiness script attempted but blocked at Docker preflight in this session.
Host must run sh scripts/check-build.sh; full container readiness not claimed.
No agent commit/push/deploy or production data writes.

## v0.1.47: grouping punctuation in titles and queued NAS log review

Stue reports three short acquire/process sets queued/running and requests log
review; David Holmes Remixes Vol1 rejected solely for parentheses. Read latest
/mnt/backup/bcfunl-*4-cron.log (append logs). Fixed lookup_title shared by query
and title comparisons to replace grouping parentheses/brackets with spaces,
keeping all enclosed words/numbers. Display tags untouched. Vol1/2/3 now match
with/without grouping. YeahX3 vs YeahX3 Remixes and Necessary Genius vs its
Remixes remain distinct; no fuzzy/word-removal relaxation. Artist/country
checks unchanged. Added full lookup cases for three volumes and negative
volume/word/empty-title distinctions. Version .47 Cargo/lock together.

Latest .46 logs have REAL separate issues: multiple verified David Holmes
Vol1 server folders (with/without [GB]); refuse overwrite/cleanup. Added both
paths to duplicate-copy error, regression checks both paths. Liber Eleven
album129740024 ZIP CRC fails at different first members across native parallel
runs. Independent read-only 7z t on exact bcstag4 ZIP also FAILS CRC on track07
Greedy Gutter Guru Remixed By Richard H Kirk (exit2). This is not merely a Rust
reader assertion. Framewerk album1324801722 existing NAS track03 Hand In Hand
(Framewerk Rewerk) fails independent flac -t with frame CRC/lost sync (exit1).
Track19 Out Of Time had native incomplete-frame errors in an earlier run but
independent flac -t NOW PASSES (exit0); do not claim all errors permanent or
the cause established. Keep suspect source/archive and existing files, cleanup
off. No running/queued jobs stopped or deployments/config changes made by
agent. Recommend no image change midqueue and investigate integrity before
scaling batch volume. Eve Maret still future-date blocked in acquisition,
expected guard; exact publication date not independently checked.

146 automated tests passed,0failed,5ignored. Fmt, locked strict Clippy, locked
optimized build, workflow regression and whitespace checks passed. Binary
v0.1.47 built2026-11-04T10:19:19Z. Actual check-build.sh attempted and Docker
preflight blocked in this session; host image gate pending. No live Discogs
API acceptance or NAS audio/DB/ZIP writes. Docs/tag rules and this handoff saved.

## v0.1.48: volume abbreviation matching and ZIP/history verification

Stue reports weekly usage only15% remaining; keep work/replies focused. User
will scrub legacy tags/duplicates, asks verify ZIP implementation and earlier
errors, and whether Vol./Volume match. Verified zip-rs/zip2 zip9.3 + flate2
1.1.10 rust_backend (miniz_oxide), shared bounded pool and independent readers.
Correct maintained crate, but NOT claiming fastest measured backend: official
flate2 docs recommend zlib-rs + runtime_detection for maximum Rust performance
(at cost of internal unsafe). No backend/dependency tuning performed or native
production C dependency added. Leave such optimization to measured follow-up.

Found actual PRE-migration /mnt/backup/bcfunl-process-cron.log starts v0.1.28.
Line1932: Pop Ambient2026 7z CRC Failed on Micå New Drop. Lines2026/2033/2036:
7z errno5 Input/output error (Black Dog My Brutal Life, Kid Spatula Joozy,
Propaganda Remix Encounters). Earlier FFmpeg invalid subframe padding, residual
coding etc too. Integrity error class predates Rust ZIP (.40), not proof of root
cause. Earlier independent 7z failure on current Liber Eleven remains evidence.

Vol./Volume previously differed. Added comparison-only normalization of Vol.,
Vol, Volume (case-insensitive) immediately before a numeric identifier; words
and volume numbers preserved. Query spelling, source/display titles unchanged.
Tests across volumes1-3 and all4 marker cases, wrong-volume and nonnumeric
Volume Control negatives. Existing grouping punctuation/year-edition rules
retained. Cargo/lock version0.1.48.146 tests passed,0failed,5ignored; strict
locked Clippy, fmt, locked optimized build, workflow regression, diff checks
passed. Actual readiness script attempted, blocked at Docker preflight; host
container gate and live match acceptance pending. No queued job disruption,
NAS data writes, commit/push/deploy. Reference/tag rules and handoff updated.


## 2026-11-04: v0.1.49 CRC recovery and no-burn parking

Stue requested one complete re-acquisition after a ZIP CRC failure, preserving
failed payloads for review and protecting them from cleanup. Implemented in
src/processing/recovery.rs, called by the processing coordinator after a worker
returns the typed archive::CrcFailure. zip9 exposes its CRC failure as exact
InvalidData/"Invalid checksum"; only that error is classified. Unsafe paths,
unsupported archives, I/O errors and audio validation failures do not trigger
this ZIP recovery. Original ZIP worker path context remains in the error.

Automatic SQLite migrations add releases.parked and crc_reacquired (default0),
and parked_payloads(path,user,item_key,parked_date,reason,no_burn=1). Before any
new acquisition, set parked=1 and record protection, then preserve the failed
file via create-new hard link in a unique sibling .parked-no-burn-* directory,
sync, unlink the old name, and sync the parent. Failure/interruption leaves the
release excluded for manual review; files are never intentionally discarded.

Persist crc_reacquired=1 before network I/O. Reuse download::fetch with the
stored download page and format, fresh HTTP stream, force=false, no resume or
existing-file overwrite. Already captured public release metadata is retained,
so private/deleted public pages do not block recovery. This exceptional recovery
uses the existing Rust streaming downloader, not aria2 resume. SQLite stays as
is. Coordinator owns database/network recovery; other active workers continue.

Successful replacement updates the history source path and retries processing.
A second CRC failure parks that payload too, permanently exhausting automatic
re-acquisition. Failed acquisition, offline mode or cancellation keeps parked=1;
no automatic loop on later cron runs. A valid replacement with another processing
error is retained for normal processing retries. Parked releases are excluded
from process/clean selection and acquisition even with --force. No reset command
was added. Review evidence/errors before deliberately restoring a valid source
and clearing the release parked flag; do not erase the no-burn ledger.

Cleanup checks ledger paths both literally and canonically and cannot burn parked
evidence even after a successful replacement. Retried successful payloads remain
eligible for normal verified cleanup. No-burn protects application cleanup only,
not external shell deletion; evidence consumes disk space until reviewed.
README/reference include behavior and read-only review queries using johnsmith.

Validation: 149 passed,0failed,5ignored; locked strict Clippy, fmt, optimized build,
workflow regression and diff checks passed. Actual scripts/check-build.sh attempted:
Docker daemon unavailable in agent session, so container/package readiness remains
blocked for host verification. Live authenticated Bandcamp/NAS re-acquisition not
exercised. Focused tests inject fresh acquisition, cover preserved bytes, second
CRC durability across reopening SQLite, force exclusion, offline/network failure
parking and cleanup no-burn rejection. Existing real ZIP corruption test confirms
typed classification. No NAS payload/database changes, commit, push or deployment.
User requested memory/status and a break due to low remaining weekly usage.


## 2026-11-04: v0.1.50 daily database recovery and preorder availability

Stue requested --database as the daily first-touch job: test history, retain seven
verified timestamped backups, restore the newest healthy backup on failure, and
apply current DDL. Stue accepts a possible extra download after restoring history.
Implemented src/database.rs with the existing staging lock and native system
SQLite backup API (new small bindings in db.rs), not an external sqlite3 process.

Checks: full PRAGMA integrity_check, foreign_key_check, required releases columns
and composite primary key, owner/key syntax, nonempty artist/title, dates and
completion timestamps. No claim of detecting plausible but incorrect text.
Backups use native SQLite snapshotting, include committed WAL data, are checked
and synced, then published without overwrite under <output>/backups as
collection.sqlite3.YYYYMMDDTHHMMSS.nnnnnnnnnZ. New snapshot files are private
0600 and newly created backup/quarantine directories 0700 on Unix. Retain seven
healthy snapshots; preserve suspect backups for review. Repeated invocations each
create a snapshot, so schedule once daily. No resources/library.sqlite3 backup.

If live history fails checks, search backups newest first, rejecting bad ones.
Prepare a replacement in a private temporary directory, apply Database::open
migrations, and recheck before installation. Preserve original main database and
journal/WAL/SHM files via hard links under database-quarantine-*, sync, remove live
sidecars, atomically replace main, sync and recheck. Reject symlinks. With no healthy
backup, stop without installing empty history. Bootstrap initial acquisition once
before enabling the database task for new staging. Quarantine is never auto-pruned.
Restored history may forget a day's activity; server audio remains protected by
normal existing-copy verification. No disk-wide redundancy: independent backup
storage is still recommended. No NAS database or payload was modified in session.

Compose supports the database argument. scripts/maintenance.sh runs it first and
aborts on a failed/unrecoverable check, then retains existing acquisition/process
error semantics. Docs show chaining database && acquire && process rather than
relying on separate timing to order tasks. Did not alter the user's existing
Compose test count/date settings. Updated lock error to include database jobs.

Preorders/future dates no longer categorically prevent acquisition. Capture their
metadata and inspect the authenticated owned download page for supported audio.
An offered trusted audio URL permits a transfer even before publication, including
refreshed links. Actual completion is still recorded only after successful payload
transfer. Empty formats on a restricted release return typed Deferred, are logged
without failing the run, and remain eligible. Other network/payload/origin errors
still fail. Existing stale-marker title cleanup age rules remain intact.

New automatic columns: is_preorder (ever observed; MAX uses CAST bound value to
INTEGER), scheduled_release_date, availability_status, availability_checked_at,
release_private. States captured/pending/available/early-access. Retain source date
and preorder evidence. src/availability.rs loads recorded pending/early-access
releases before collection scanning, oldest check first, respecting artist and
shared count limit. Uses scheduled date fallback when original publish date absent.
Private state persists. A deferred availability check consumes one count slot.
For these entries, date filters match purchase date OR scheduled release date,
with both bounds applied to the same date. Shared SQL helper used in offline
processing/clean too; ordinary entries keep purchase-date filtering. Covered
from-only, to-only, both/neither, cross-window dates, artist/count, completed and
parked exclusions. Never-before-captured entries still need collection discovery;
missing download-page URLs retain existing skip behavior. No migration guesses
legacy preorder state solely from old error strings.

Cleanup also refuses .parked-no-burn-* ancestry independently of the SQLite ledger,
protecting evidence when recovery restores a snapshot predating its parking.

Validation: 157 passed,0failed,5ignored; locked strict Clippy, fmt, locked release
build, whitespace, shell syntax, workflow-regression checks all passed. Compose
config validated with dummy values and --env-file /dev/null, no secret output.
Actual check-build.sh attempted: Docker daemon unavailable in this agent session,
so image/package readiness blocked for host verification. Binary v0.1.50 built
2026-11-04T20:27:44Z. Live Eve Maret subscription acceptance and NAS restore not
performed. Tests cover native committed-WAL snapshot, retention, corruption/older
backup fallback, malformed timestamps, no-healthy/missing DB refusal, schema
migration, lock/CLI conflicts, no-burn without restored ledger, future authenticated
links, unsafe-origin rejection and nonfatal deferral. No new runtime dependencies,
commit/push/deploy, queued-job interruption or NAS writes.


## Backup deployment preference

Stue accepts the current `<output>/backups` location on the NAS; a configurable
backup destination is not required. Stue plans to configure a separate job or
app to rsync the database backup copies to cloud storage after the daily
`--database` job finishes. This provides an independent copy in addition to the
seven local verified snapshots. Cloud sync is planned, not configured or verified
by this application or this session. No cloud provider has been selected here.


## 2026-11-05: v0.1.51 retry phases, five-failure parking, and status grids

Stue reports NAS processing has run for roughly24hours; daily database backups
and independent Dropbox cloud backup are now stood up. This supersedes the prior
cloud-sync planning note. Deployment was reported by Stue, not independently
inspected. Backup folder is still <output>/backups, currently expected NAS
/mnt/backup/bcfstag4/backups. No configurable destination or new cloud integration
was added. Keep the existing seven verified snapshots policy.

Stue observed prior acquisition failures consuming --count slots repeatedly.
Changed acquisition to snapshot the eligible retry queue before new work, scan
for count-limited fresh entries, complete that batch, then run the retry queue
outside --count even when fresh work failed. Same owner/artist/date filters apply.
New failures/deferred checks enter next run's queue, not a second immediate try.
Capture excludes already-attempted/deferred rows from fresh slots. Retries reload
stored download URLs, canonical release URLs, dates and private status. Query
previously named availability::pending is now availability::retries; no LIMIT.
Both sequential and parallel acquisition use the same two phases. Jobs with a
large retry backlog can exceed --count substantially by design.

Processing likewise sorts prior errors after fresh rows, limits only fresh
attempts, and drains/records all active fresh workers before starting retries.
Previously failed rows with historical process_date no longer incorrectly skip
solely on the old success timestamp. Existing publication verification, staging
and server locks, offline restrictions, CRC re-acquisition limit, and no-overwrite
server safeguards remain. Cleanup's standalone removal count behavior unchanged.

New automatic columns: download_failures, process_attempts, process_failures.
Five genuine failures in either stage set parked=1, retaining errors and payloads
and excluding future work/clean even under --force. Success resets its stage
failure counter. Deferral/cancellation does not increment a failure counter;
preorder checks remain eligible without exhausting the five-failure budget.
Download attempts continue to count every attempted check; processing attempts
are counted separately. CRC recovery/no-burn may still park sooner.

Migration seeds legacy failed acquisitions from MIN(attempts,5) when download_date
is NULL and last_error present; exhausted rows park immediately. Legacy attempts
did not distinguish historical deferrals, so this is an estimate from existing
history, not reconstructed event-level evidence. Prior process-error rows seed
one known process attempt/failure because no historical process count existed.
Database open reasserts parking at exhausted budgets: manual review/reset must
address the relevant failure counter as well as parked flag. No automatic unpark.

With --summary, preserve old acquisition table and processing totals. Processing
adds a per-release table for attempted keys (status, attempts, error), and both
modes add an owner-scoped whole-history status grid. Show processed, awaiting
processing, acquisition failed, processing failed, deferred, parked and pending
acquisition even when zero, with release/downloaded/processed totals. Parked has
precedence; historical timestamps are counted as stored completions. Current
artist/date filters do not narrow this second collection-wide grid. Canceled
unfinished processing is not falsely labeled success. Shared table formatting
retains Unicode alignment and escaped terminal controls. README/help/reference
updated to remove contradictory old retry/count documentation.

Validation:161 passed,0failed,5ignored; locked strict Clippy, fmt, locked optimized
build, workflow-regression and whitespace checks passed. New tests prove fresh
count slots exclude failures, parallel fresh work finishes before retry starts,
fifth failure parks in acquisition and processing, deferred checks do not exhaust
budget, exhausted legacy migration, owner-scoped status totals, and parking's
force/selection exclusion. Processing barrier test uses a SQLite trigger requiring
fresh publication to be recorded before the retry attempt may begin.
Binary v0.1.51 built2026-11-06T04:49:09Z (Nov5local). Actual check-build.sh attempted,
blocked at Docker daemon preflight in agent session; full image/package and NAS
acceptance remain host-side work. No new dependencies, NAS writes, deployment,
commit/push, cloud changes, or disruption of running jobs. Memory/status saved.


## 2026-11-06: v0.1.52 legitimate package keys in database maintenance

Stue's first NAS --database run stopped with Invalid release key in history and
No healthy database backup available. Root cause was our semantic validator, not
evidence of structural corruption: acquisition item_key preserves item_type and
accepts package records, while maintenance allowed only album/track prefixes.
Read-only NAS query at /mnt/backup/bcfstag4/collection.sqlite3 first found54rejected
keys with package examples. A later read confirmed album1567,track376,package54,
and ZERO keys rejected by the corrected album/track/package plus numeric-ID rule.
One intervening read hit an active SQLite lock; did not stop jobs or remove locks.
No independent full NAS integrity check or maintenance/restore was performed.

Fix: src/database.rs accepts package numeric keys and prints the escaped rejected
key on a real validation failure. No stored keys, files, history or collection
producer were rewritten. Regression creates an older album-only snapshot, then
adds track/package records through collection::item_key and verifies maintenance
keeps all3records without restoring/quarantining healthy history. Invalid empty
IDs, nonnumeric suffixes and unknown prefixes still fail with identifying errors.

Version0.1.52 Cargo/lock synchronized.162 tests passed,0failed,5ignored; locked
strict Clippy, fmt, locked optimized build, workflow regression and diff checks
passed. Binary built2026-11-06T12:07:01Z. Actual check-build.sh attempted, Docker
daemon preflight still unavailable in this agent session. Image/package gate and
actual NAS --database rerun remain pending. User should deploy the new image and
rerun database maintenance when staging jobs are idle; no manual SQL repair is
needed for this false rejection. Earlier stopped maintenance retained live history
unchanged and made no healthy snapshot. No commit/push/deploy or NAS data writes.


### Session close acknowledgment

Stue acknowledged the v0.1.52 package-key validation fix and requested memory
and status before the break. Both are current. Next step remains deploying the
new container image and rerunning `database` maintenance while other staging
jobs are idle. No further implementation or NAS changes were requested.


## 2026-11-08: SIGILL isolated to damaged copied executable, not AVX

Stue's NAS Atom C3558 has SSE4/AES but no AVX. bcfunl --version exits132,
aria2c/sqlite3 run. Kernel reports repeatable bcfunl offset0x84ec3e; older runs
offset0x84dc3e. User copied exact latest image binary via docker create/cp to
/mnt/backup/bcfunl-crash.bin. SHA256:
c220f64764555113869e6322937581d0921fd245ef4e8667e7d60dfd0ee35a8b
BuildID7267153d3bb9bce36a2c0cf0dcf58f64af132304; v0.1.52 built12:17:01ZNov6.

Exact copied binary also exits132 on myth2017 (core disabled). Disassembly finds
UD2 at0x84ec3e inside Tokio push_remote_task, reached during Rust std runtime
initialization BEFORE application main. This is NOT an unsupported AVX opcode.
GDB ptrace denied; temporary LD_PRELOAD diagnostics and temporary copies under
/tmp established correct GOT relocation and localized failure to Rust SIGPIPE
startup jump table at0x202568. Entry0=0x0064c6cf ->target0x84ec37 (middle of an
unrelated instruction); entry2=0x0064c6c8 ->0x84ec30. Setting bit0x00080000 in
those two words restores destinations0x8cec37/0x8cec30 inside std runtime.
ONLY this change in /tmp/bcfunl-jumptable-probe makes --version succeed. This is
a diagnostic proof, NOT an approved repair: no patched binary deployed and no
NAS original/image/database changed. All modifications were /tmp copies.

Other /tmp probes: bcfunl-startup-inspect.c/.so (constructor GOT and SIGILL
handler diagnostics), bcfunl-main-probe, bcfunl-runtime-probe, bcfunl-call-probe,
bcfunl-checkpoint-probe. Do not use these as application executables.

Next: compare the exact same OCI image digest's /app/bcfunl checksum and --version
on another machine (myth2017 or media2026) against NAS. Need NAS docker image
inspect RepoDigests output and host Docker run. Agent Docker daemon access remains
blocked and external ghcr.io DNS unavailable, so cannot obtain pristine image
independently. Need distinguish published artifact damage from NAS extracted image
cache/storage/RAM. Earlier NAS DB/text errors included single-bit character
changes and FLAC/ZIP corruption; possible common integrity issue but root cause
NOT established. Do not blame CPU, change target-cpu, deploy patched instructions,
or label RAM proven faulty. Native repo binary still works; no source changes
or version bump made during this diagnosis. Preserve failed binary as evidence.


### Exact-image comparison confirms published image is healthy

Stue supplied NAS RepoDigest:
ghcr.io/shunte88/bcfunl@sha256:edf37c6bc1d72dbc43fe5599099790e226dd6e85ee8bf995aa9caafe00faf52d
User pulled this exact digest on myth2017 and tested without staging mounts.
/app/bcfunl SHA256=910e7d766dc14e43a1ec6037466487485678d76b181126936f07dfd52bc109b9
and --version succeeds: v0.1.52 built2026-11-06T12:17:01Z. Failing copied NAS binary
hash remains c220f64764555113869e6322937581d0921fd245ef4e8667e7d60dfd0ee35a8b.
Same digest/build timestamp, different bytes and startup behavior: published
artifact is good; NAS-local copy is damaged. Root cause within NAS extracted
image/storage/memory path remains unknown. Not an AVX requirement or source fix.

Next: at an idle maintenance window, pause scheduled jobs, remove the affected
local image only (no broad prune/force), re-pull exact digest, verify expected
checksum and --version before resuming. An ordinary pull alone may reuse the
existing affected layer. If removal reports container/tag references, inspect
those references rather than force removal or disrupting other services. Keep
/mnt/backup/bcfunl-crash.bin as evidence. No application source/version changes
are necessary; no NAS image or job was changed by agent. Recurrent corruption
requires NAS RAM/storage/image-cache investigation, not CPU build flags.


### NAS image replacement accepted, 2026-11-08

Stue removed stack bcfunl4 and the affected local image (actual layers deleted),
re-pulled exact digest edf37c6bc1d72dbc43fe5599099790e226dd6e85ee8bf995aa9caafe00faf52d,
and verified /app/bcfunl checksum OK against
910e7d766dc14e43a1ec6037466487485678d76b181126936f07dfd52bc109b9.
NAS --version now succeeds: v0.1.52 built2026-11-06T12:17:01Z. Startup SIGILL
resolved by replacing the damaged local image; no source change/version bump.
The n62 script accidentally included explanatory prose, producing line10
If: command not found; subsequent pull/check/version completed successfully.
Remove that prose line from script. Next run one database task and a small
acquisition/processing batch before resuming normal scheduling. Stack service
name in NAS output is bcfunl4 (not repository example bcfunl). Underlying
corruption cause remains unproven; retain failed binary evidence and investigate
NAS memory/storage if corruption recurs. Agent made no NAS mutations.


## Session close: NAS full cycle running, 2026-11-08

Stue confirmed NAS is back up after replacing the damaged local image. Process
listing shows docker compose run --rm -T bcfunl4 acquire active as of06:30local,
using BCFUNL_COUNT, parallel8 and --summary. Date filters remain commented out
intentionally, confirmed by Stue: this is a count-based full batch cycle expected
to take several hours. NAS process branch uses --process, parallel8, template,
summary, timings and --clean, without a count argument. Do not infer cleanup is
disabled from stale comments in that entrypoint. Database mode is also present.

Startup checksum/version acceptance passed; full acquisition/processing/cleanup
cycle completion and database-maintenance success are NOT yet reported. Leave
running jobs and deployment unchanged until the cycle completes. No source or
version changes required for the SIGILL fix. Local release remains v0.1.52.
User requested memory/status and a break. Next session review the finished batch
summary and database job result, retain bad-binary evidence, and remember the
underlying NAS-local corruption cause remains unknown.


## 2026-11-12: v0.1.53 cleanup alias and cooperative processing governor

Stue requested standalone --cleanup and process --nice N (10..50, default20)
CPU/RAM percentage budgets, automatic backoff when resources are tight, and early
exit when insufficient. Follow-up explicitly asked for a governor monitoring usage
and throttling/stopping work. Clarified cooperative application control versus hard
Docker/cgroup ceilings; implementation does not claim a strict kernel resource cap.

--cleanup is a visible alias on the existing clean argument. Reuses standalone
verified cleanup with no processing, staging lock, manifest verification and
parked/no-burn safeguards. Existing --clean and combined --process --clean work.
Compose adds cleanup mode; it must be copied into the NAS custom bcfunl4 service.

New src/limits.rs governor starts only from main's --process dispatch, before
processing initialization. It runs one lightweight Rust monitoring THREAD, not
an external watchdog process or tool. Guard stops/joins thread on exit. Samples
effective capacity and RSS/free memory every250ms; Linux /proc and inspectable
cgroupv2 required. CPU available_parallelism plus ancestor cpu.max constrain cores;
MemTotal plus ancestor memory.max constrain RAM, current cgroup headroom accounts
for reclaimable inactive_file cache. Missing resource metrics fail closed.

Shared process CPU time covers all threads. A token budget throttles native I/O,
archive, decode, tag and publication checkpoints; idle credit capped at100ms so
network idle cannot bank unlimited CPU bursts. Sleeps are cancellable50ms slices.
Budget is N% OF effective container allocation, so adding tighter container caps
is conservative (2cores allocation with20% targets0.4cores). Not Unix nice priority.
On Atom4cores,20% targets0.8cores averaged over time. No CPU target build flags
changed and this does not repair hardware corruption.

Shared file pool and release scheduling use the initial derived worker limit,
max8; --parallel can only lower it. Fresh admission rechecks current memory and
drains workers to reduce concurrency. Scheduling estimate128MiB per admitted job
plus64MiB free-memory headroom. If no working reserve fits, stop early. Governor
requests an orderly stop if RSS reaches percentage budget, available memory drops
below headroom, or monitoring fails. Thread-pool/release-thread creation failures
also stop safely rather than panic or mark healthy releases bad. Release worker
spawn now uses fallible Builder::spawn_scoped. Active workers reach safe checkpoints,
completed publications are recorded, unfinished sources retained, automatic cleanup
skipped on resource stop. Resource errors are recorded without incrementing the
five-failure parking budget and are not retried as audio corruption.

These are COOPERATIVE limits: native library calls, kernel I/O and allocations can
burst before another checkpoint. RSS estimate is not a maximum album working set.
Use Docker cpus/mem_limit/pids_limit for hard ceilings; no privileged container,
Docker socket, writable cgroup mount, or new runtime dependency was introduced.
Actual memory-pressure/OS thread exhaustion and hard-cap behavior not live tested.

CLI/env/docs: --nice valid10..50/default20, requires process only when explicitly
specified. Compose BCFUNL_NICE defaults20 and process branch passes it. .env.example
includes it. README/reference/container guide explain soft controls, lower
concurrency, early stop, standalone cleanup, and hard-limit boundary. Did not
change NAS jobs or existing count/date settings. No new SQLite columns.

Readiness adds scripts/test-processing-cli.py to scripts/check-build.sh, thus CI
exercises real main dispatch/governor and cleanup. Uses existing FFmpeg only to
create a synthetic60second FLAC, then native app PATH=/nonexistent to publish
and standalone --cleanup to remove only verified source. Checks CPU accounting
against reported budget with startup allowance. Workflow regression stub updated
to assert this command is invoked. Manual CLI smoke passed repeatedly.

Validation:166 passed,0failed,5ignored; locked strict Clippy, fmt, locked optimized
build, Compose config with dummy env, workflow-regression, shell syntax and diff
checks passed. Binary v0.1.53 built2026-11-12T12:35:26Z. Actual check-build.sh
attempted; agent Docker socket still unavailable, so full image/package gate and
NAS acceptance pending. One transient parallel test run hit server-lock reacquire
failure in existing processing_zip_records_completion_and_skips_verified_server_audio;
isolated rerun and subsequent complete suites passed. No production lock changes
made. If repeated, investigate inherited-lock lifetime around concurrent test
FFmpeg forks rather than deleting lock files. No live NAS data writes, job stops,
commit/push/deploy, source-binary patching or cloud changes.


### Stand-down, 2026-11-12

Stue acknowledged v0.1.53 and requested memory/status before standing down.
Both files are current. Next session: complete the host Docker readiness gate,
build/deploy the image, merge cleanup mode and BCFUNL_NICE into the NAS custom
bcfunl4 Compose configuration, and test standalone cleanup plus a small governed
processing batch. Native tests/build and CLI smoke passed; full Docker/NAS
acceptance and live resource-pressure behavior remain pending. Budgets are
cooperative, with hard ceilings requiring container/kernel limits. Leave existing
NAS jobs and data unchanged until Stue resumes deployment/testing.


## 2026-11-14: v0.1.54 persistent stop-file trigger

Stue requested a specified trigger file that cleanly stops the active step and
causes subsequent sequential invocations to exit while the file remains. Preserve
environment/history, skip payload cleanup; exact resume optional if expensive.
Reused existing safe cancellation rather than adding byte/frame checkpoints.

New global --stopfile PATH; default <output>/.bcfunl-stop for every mode. Main
configures an absolute path before run, and run immediately checks shutdown before
validation, directory/lock creation, database migrations, networking or governor
startup. Help/version still print only information. Existing trigger -> successful
exit0 with path in stop message. Trigger is NEVER consumed. Removing after jobs
stop permits subsequent invocations to proceed. Use same output or custom path
for a sequence. No SQL schema change or new runtime dependency.

shutdown::requested shares presence checks across all workers at most once per
50ms via atomic polling deadline. Presence (including symlink/directory entry)
latches stop for the current invocation. Missing file is normal; other stat errors
fail closed and exit1. SIGINT/SIGTERM retain existing128+signal exit codes. Async
requests, sleeps, transfer children, CPU governor and file/decode workers reuse
existing stop checkpoints. Stop may wait for blocking/native operations or atomic
steps already underway. Completed publications/history are recorded. --process
--clean skips payload cleanup on detected stop; already-running standalone cleanup
stops further removals but cannot undo removals completed before detection.

Private partial transfers and temporary process work are still safely discarded
and child processes reaped. This is intentionally not exact byte/frame resume:
completed entries skip on rerun; unfinished items restart under existing history.
Explicitly explained this boundary to Stue. Stops do not consume genuine-failure
parking budgets. No live jobs, NAS data, or trigger files were changed by agent.

Compose optional BCFUNL_STOPFILE (blank uses native default) prepends the override
centrally for database/acquire/process/cleanup. Override must be visible inside
a mounted path. Do not also supply duplicate --stopfile extra args when env sets
it. .env.example, README and reference/container docs updated. A large concurrent
user rewrite of docs/CONTAINERS.md appeared during work; retained it, appended
our section, and removed one trailing space flagged by diff-check. No unrelated
content was reverted.

Validation166passed,0failed,5ignored; locked strict Clippy, fmt, locked release
build, workflow-regression, Compose dummy-env config and diff checks passed.
Existing shutdown subprocess matrix now tests stop-file requests in six modes
(pause/save/aria2/async/parallel/processing) in addition to SIGINT/SIGTERM: exit0,
trigger retained, prior completion intact, unfinished row not falsely complete,
SQLite integrity and child reaping verified. Real CLI smoke extended to ensure
acquire/database/resources/process/cleanup skip at startup, a custom trigger
prevents even output-directory creation, inaccessible/malformed trigger paths
exit1, removing trigger enables verified standalone cleanup, and no external
programs on application PATH are needed. Missing cookies/resource fixture paths
prevent accidental networking in startup regression tests.
Binary0.1.54 built2026-11-14T19:53:02Z. Actual check-build.sh attempted: Docker
daemon access remains blocked in agent session; full image/package gate and NAS
stop-file acceptance pending. No commit/push/deploy or application binary patch.

## 2026-11-14 handoff: v0.1.55 full Discogs release library

Stue reports moving runtime back to media2026 because the NAS backup drive was
showing many storage errors. The new stage path was not supplied. No runtime
jobs, remote files, or deployed containers were changed in this session.

`--resources` now fetches artists, masters, AND detailed releases for the current
UTC month. Retain masters for canonical grouping and distinct-master artist
genre evidence, avoiding pressing-count bias. `--release-file` accepts existing
XML/gzip; supply it alongside --resource-file and --master-file for fully local
imports. Existing master-only libraries remain usable until rebuilt.

Detailed releases stream one XML record at a time into indexed releases and
release_artists tables. Full record JSON retains catalogs/labels, country/date,
genres/styles, credits, formats, tracklists and additional XML fields/attributes.
Provenance stores all three source hashes. Gzip/manifest verification, private
staged rebuild, integrity check, cancellation and atomic replacement remain.
Full dumps were not downloaded/imported during development; allow disk space
for archives plus old/new databases and validate import time/size on media2026.

Processing now queries the local title index first, loading one candidate body
at a time and reusing the online artist/title/country matching and genre
sanitizer. Deterministic first eligible release ID supplies catalog, label,
genres/styles and record-wide performer/composer/conductor credits. Complete
tracklists are retained for research; existing source track-credit handling
remains. Each release worker opens a read-only SQLite connection. Local matching
has no API pacing; the cooperative CPU/RAM governor remains. API fallback stays
rate-limited unless --offline disables it. Artist-default genre fallback retains
its existing offline-only behavior. Shared title identity moved into
src/discogs_identity.rs so import keys and online/local comparisons stay aligned.

Prior v0.1.54 persistent stop-file implementation remains included. Version
bumped to v0.1.55, built 2026-11-14T20:17:10Z. Locked offline tests: 168 passed,
0 failed, 5 ignored. Formatting, Clippy with denied warnings, locked optimized
build, build-workflow regression, and actual CLI smoke test passed. CLI test
uses local release SQLite matching, --offline and PATH=/nonexistent, confirms
catalog publication, governor, stop-file startup paths and standalone cleanup.
Import fixtures cover plain/gzip, preserved nested metadata, provenance,
idempotence and failed rebuild retaining old library. Local matching covers
country rejection/composite acceptance, artist asterisk, remaster title,
genre expansion and scoped credits.

Actual `sh scripts/check-build.sh` was attempted and stopped at unavailable
Docker daemon access. Container/package gates and full monthly import/runtime
acceptance are NOT complete. No commit or push performed. Preserve Stue's
concurrent extensive docs/CONTAINERS.md rewrite.

## 2026-11-14 documentation follow-up

README updated with beginner-friendly local Discogs setup and processing examples,
--offline, --library, optional API fallback, monthly refresh, existing-library
compatibility, and keeping staged downloads until cleanup is requested. Project
art/heading preserved. Simplified resource-budget and installation explanations,
removed technical material trailing the license, and corrected outdated preorder
summary. Documentation defaults now recorded in AGENTS.md: US English, no em or
en dashes, friendly minimally technical explanations in the spirit of For Dummies,
short practical examples, implementation details in the reference guide.
Markdown fences, local links, punctuation preferences and git diff whitespace
checked. Documentation-only change; no new runtime/build tests or version bump.

## 2026-11-14 handoff: v0.2.0 collection tag maintenance

Stue requested a new --collection verb to tidy an existing mixed-vendor music
library in place. He explicitly corrected the initial --walk reference:
--collection ONLY, no --walk alias. Existing staging publication stays separate.

CLI: --collection --serverdir MUSIC --output HISTORY [--library LIBRARY]
[--parallel 3..16] [--count N] [--artist PATH_SUBSTRING] [--force] [--summary].
Defaults: 3 collection workers, library at <output>/resources/library.sqlite3.
Other modes retain parallel 2..8; mode-specific upper bounds checked in validate.
Collection conflicts with staged process/clean/resources/database, purchase dates,
templates, --offline and --nice. Collection is inherently offline and never uses
cookies/API. Native Windows returns an explicit unsupported-platform error.
No new crate or runtime tool dependency was introduced. --nice remains specific
to staged --process; use container CPU/RAM ceilings for collection walks.

src/processing/collection.rs walks real directories, skips symlinks, unsupported
formats and private temporary folders, and groups audio by folder then album tag.
Supported: FLAC, MP3, WAV, AIFF/AIF/AIFC, M4A ALAC/AAC, Ogg Vorbis. Filenames and
folders are never renamed. Count limits attempted folders, not tracks. A bounded
Rayon pool handles 3..16 folders, without nested track pools or API pacing. Each
worker opens a read-only Discogs connection. Horizontal operation partitions
nonoverlapping music roots; no distributed dispatcher or automatic partitioning.

Reuse local indexed artist/title matching. Collection constrains existing
Discogs IDs, catalog numbers (ignoring punctuation/spaces/case), and known
countries. Missing country allows any country for artist/title matches in this
mode only. First eligible local release ID wins. Download processing's stricter
country matching remains. Missing/conflicting identity skips enrichment and
performs safe cleanup only. Confirmed records supply album artists/sorts,
track artists (unique exact track-title match), record/track-scoped explicit
performer/composer/conductor/lyricist/writer/producer/engineer/arranger/remixer/
mixer credits, genre/style, release date/year, label, catalog aliases and Discogs
ID. Exact listed track positions support scoped credits; ambiguous ranges are
not guessed. Sort-name commas are retained, and existing source credits/tags
are carried forward. Without a match, derive only supported missing fields from
existing tags; no artist-default genre guessing or invented track numbers/art.

Resources now build sanitized_genres from distinct master+release genre/style
values via the shared validation/case/spelling/expansion logic. Rebuild with
--resources before collection mode. Same-source hash skipping also checks for
this new table, so an old library gets rebuilt. Collection requires a nonempty
vocabulary before writing music. Every expanded final genre must belong to it;
unlisted entries are removed. The library remains read-only during collection.

History adds collection(path,artist,title,walk_date), primary key path/artist/title,
to <output>/collection.sqlite3 via normal DDL. Only completed folders are recorded;
all album identities in a mixed folder are kept. Repeat runs skip those folders;
--force revisits them after new files or refreshed metadata. Shared collection
history locks allow peer collection jobs but exclude acquisition/process/cleanup/
database maintenance on the same history root, preventing restore swaps mid-walk.
Music-root locks exclude another collection/publication job on the same root.
Do not use overlapping parent/child roots across jobs. Shared histories require
SQLite-compatible storage; separate histories are fine. All locks are advisory.

Updates use a private same-directory copy, native tag writing, decoded PCM hash
comparison, verification of ALL embedded pictures and all requested named tags,
then atomic per-file replacement. File modification time (subsecond precision),
owner/group and standard permissions are preserved; Linux xattrs copied too.
Source tag changes since initial read and inode/size/mtime/ctime changes before
replacement cause refusal. Read-only/hard-linked/corrupt media remains unchanged.
Unknown named tags are retained and no artwork is downloaded or added. Raw tag
comparison also rewrites stored semicolon lists into canonical repeated values,
rather than skipping because normalized read values happened to match. File
locks explicitly unlock on drop, including error paths, so concurrent test
helper forks cannot extend an advisory lock lifetime. Folder changes are NOT
transactional: successful files remain updated if another file fails; that
folder gets no completion history and can be retried. Back up music first.

README, reference, container guide, AGENTS source map, Compose collection mode,
and actual CLI smoke updated. Compose uses configured music/staging paths and
native default concurrency; extra --parallel can override it without duplicate
flags. No --clean is passed. Stop-file startup and cancellation checkpoints apply.
Version v0.2.0 built 2026-11-15T01:18:00Z (November 14 local time).

Validation: 176 passed, 0 failed, 5 ignored; locked optimized build, strict locked
Clippy, fmt, workflow-regression, Compose config, and actual offline CLI smoke
with PATH=/nonexistent passed. Tests cover mixed containers, unlisted/all-removed
genres, expanded lists, dates/credits/catalog, source custom tags, exact mtimes,
multiple embedded pictures, parallel folder processing, read-only library access,
ID/catalog/country exclusions, Linux xattrs, changed-source refusal, corrupt files,
hardlinks, history skips, and shared-lock exclusion/compatibility. Actual CLI
exercises collection without network/tools plus original governor publication,
stop-file startup and cleanup. check-build.sh attempted: Docker daemon access
unavailable, so full image/package gate remains BLOCKED. Full monthly extract
acceptance, production media2026 walk, remote multi-machine execution, and live
resource-pressure behavior have NOT been exercised. No live music/data changes,
remote deployment, commit or push performed.

### Documentation confirmation and stand-down

Confirmed README contains "Tidy an existing music collection", resource refresh
and --collection examples, genre restrictions, preserved file modification dates
and artwork, supported formats, concurrency 3 through 16, history skips and
--force guidance. Memory and status are current for v0.2.0. Native validation
remains 176 passed, 0 failed, 5 ignored; Docker/full-import/live-walk acceptance
is still pending. No runtime checks were repeated for this handoff-only update.

## 2026-11-14 handoff: v0.2.1 selective resource refresh and genre-only testing

Stue found rebuilding resources for the new genre table too expensive for tests
and requested datestamps/sizes for selective file download/import. Added
--resources --genres-only --output ROOT. It requires an existing resource
library at ROOT/resources/library.sqlite3, makes no network requests and reads
no extract files. No cookies or artist-map loading is needed. It refreshes the
approved genre list transactionally and leaves artist/master/release records
intact. Missing library fails without creating an empty database. Extract-file
overrides conflict with --genres-only. --force with --genres-only rescans the
library's saved classification fields rather than using the cached vocabulary.

New genre_vocabulary table stores a small raw genre/style JSON array per masters
and releases dataset. XML imports collect this once in a bounded BTreeSet while
streaming; no extra release JSON scan is needed. Legacy libraries seed caches
with a one-time SQL scan of their existing classification data. Future genre-only
refreshes read these tiny caches. Empty/failed genre refreshes roll back and
retain the prior approved dictionary. Existing collection mode uses the refreshed
sanitized_genres table unchanged.

resource_import adds source_size (bytes), source_modified (local UTC file mtime),
snapshot_date (Discogs filename date), and source_signature. Provenance DDL
migrates existing libraries. On Unix, verified digests can be reused when exact
canonical path, size, mtime, device, inode and ctime match. Cache downloads still
compare against the latest official checksum manifest. New/changed files are
hashed; other platforms continue hashing. --resources --force rehashes and
reimports all inputs. Metadata reuse cannot detect silent bit rot that leaves
metadata unchanged; this limitation is documented and marked shortcut in code.

Normal resource builds compare hashes per dataset. Unchanged snapshots skip XML
imports; missing genre tables are built from existing library data. Changed
subsets rebuild privately from a native SQLite backup, retain unchanged datasets
and imported_at timestamps, update source file metadata even when bytes are the
same, drop/rebuild only affected indexes, validate and atomically replace the
library. When all datasets change, start fresh instead of copying the old DB.
Master changes rebuild artist genre evidence; artist changes reapply that evidence
without reimporting masters/releases. Source signatures are rechecked around
hashing/import to reject mid-operation file edits. Original library survives
failed imports. A partial rebuild still needs old+new database disk space and a
copy of the existing library; genre-only mode avoids that cost.

README/reference/container docs updated with the quick recipe, source metadata,
cache behavior, monthly refresh and --force semantics. Compose now accepts the
resources mode, so docker compose run --rm -T bcfunl resources --genres-only
uses the configured staging root. Current local release v0.2.1, built
2026-11-15T02:14:30Z (November 14 local time).

Validation: 178 passed, 0 failed, 5 ignored; strict locked Clippy, formatting,
locked optimized build, build-workflow regression, Compose config and actual
CLI smoke all passed. Added tests prove genre refresh with deleted extracts,
legacy provenance/genre-cache migration, selective master-only import preserving
artist data/timestamps, file size/date capture, cached vocabulary reuse and
empty refresh rollback. Existing atomic import/gzip CRC/idempotence tests pass.
CLI smoke runs genre-only mode with PATH=/nonexistent and missing cookies, then
original governed publication, collection walk, stop-file checks and cleanup.
Full check-build.sh remains blocked at Docker daemon access. Live remote download,
full monthly dataset performance, container image/package and media2026 acceptance
remain unverified. No live data, jobs, remote deployment, commit or push changed.

## 2026-11-14 handoff: v0.2.2 folder metadata and master fallback

Stue tested Tears For Fears collection files: 19 folders, 219 updated, only 2
release matches, 10 skipped. Logs showed bracketed country/catalog decorations
still in ALBUM lookup strings. He requested two-letter bracket country and
catalog hints, reuse of existing RM/RE title rules, and master fallback. Explicit
policy: if we do not know a fact, do not invent it.

Added shared sanitizing::release_hints, wrapping existing title/catalog logic
rather than label-specific rules. Extract country and catalog from ALBUM and
immediate templated album-folder names; CD1/Disc 1 subfolders use their parent.
UK normalizes to GB, EU is a supported region. Ambiguous standalone CD/RE notes
are not inferred as countries. Numeric/spaced catalogs such as 876 249-2 and
558 105-2 are supported by the shared catalog parser, with year/edition/disc/
audio-resolution exclusions. Source catalog spellings are retained once known.
Fills RELEASECOUNTRY, COUNTRY_ISO, CATALOGNUMBER/CATALOGNO even without a Discogs
match. Existing tags win over folder conflicts and warnings identify them.
Recognized annotations are removed from ALBUM but paths remain unchanged. A
malformed bracketed ALBUM can use the complete templated folder title. Original
tag maps are captured before hint injection so source-change verification stays
valid. Country/catalog aliases normalize through the shared tagging reader.

Shared Discogs RM/RE handling moved into edition_note, used by the existing
search_title path and catalog exclusion. Handles attached RM1999/RE2014 and
SDE RM RE 2014. Edition text remains in source tags. Shared master_title strips
explicit deluxe/expanded edition notes for lookup; live-session/B-sides/remix
component notes are stripped only within explicit deluxe/expanded context.
Standalone live recordings keep their title identity. Acquisition/processing
Discogs identity now uses the same shared metadata hint/title rules. No new
force-uppercase data entries or label-specific rules were added.

Collection matching is safer now. The earlier no-identifier fallback could pick
the first different pressing and write its catalog/ID. Detailed release metadata
now requires an existing release ID or catalog selector, then validates artist,
normalized title, known country and all selectors. Indexed SQL uses title_key IN
(clean key, shared edition-base key), ordered by release ID. Catalog comparison
ignores case/spacing/punctuation. Country alone does not identify a pressing.

If detailed release lookup is unavailable or no selector exists, lookup masters
via indexed case-insensitive base title and indexed release title/master links,
then validate artist aliases/title and any existing DISCOGSMASTERID. A matched
master supplies album identity and genres plus DISCOGSMASTERID, never a release
ID, catalog, label, country or pressing date/year. The master main-release pointer
is not used as ownership evidence. Master evidence is explicitly marked as such.
All genres still pass the approved library dictionary. Unknown facts stay as-is;
source track credits and other named tags/artwork/mtime/path guarantees remain.

No resource rebuild is required for this matching update if the existing library
contains the already supported artists, masters and releases tables. README and
reference describe extraction, master/release distinction and actual SQL queries.
Use --force to revisit already walked folders. IMPORTANT: re-copy untouched test
files for a clean comparison. Earlier tests may already contain speculative
release IDs/catalogs written by the old first-release fallback, and new code
intentionally does not automatically erase existing metadata. Those IDs can pin
the old pressing even under --force. No live test copies were changed by agent.

Local version v0.2.2, built 2026-11-15T03:38:41Z (November 14 local time).
181 tests passed, 0 failed, 5 ignored; strict locked Clippy, formatting, locked
release build, workflow regression, Compose config and actual CLI smoke passed.
Fixtures cover numeric/spaced catalogs, UK/EU parsing, disc parents, explicit
conflicts, RM/RE variants, edition/component identity, master-only enrichment
without invented pressing fields, repeat idempotence and a catalog-selected
Europe release preserving folder catalog spelling and exact mtimes. Existing
native-format, artwork, concurrency and metadata tests remain green.
check-build.sh attempted, still blocked by Docker daemon access. Real Tears For
Fears dataset acceptance and container/package gates remain pending. No remote
jobs, source music, resource imports, deployment, commit or push changed.

## 2026-11-15 handoff: v0.2.3 lightweight direct collection metadata writes

Stue reported slow collection walking at --parallel 15. Current live command is
--collection --serverdir /mnt/FN042018/F8NAS1/music/ --output /data/bcfstag4
--library /data2/bcfunl/downloads/resources/library.sqlite3 --parallel 15
--count 1000 --summary. User confirms /data and /data2 are NVMe, music is on NAS.
Library workers are read-only SQLite connections, history writes occur on the
main thread, so ordinary inter-worker SQLite writer contention is unlikely.
The earlier collection design placed full copies next to the NAS music and ran
two PCM decode/hash passes plus picture comparison and file/directory syncs.
This was a costly safety layer, not required for preserving file mtime.

Stue explicitly requested removing this stress work: construct metadata and write
it directly with the native backend, retaining graceful file-unit shutdown. He
also confirmed the trigger file worked perfectly. No active job was stopped or
changed by the agent. SSH diagnostics were unavailable (system config permission
error, then alias DNS failure with explicit user config); no live NAS/CPU/IO
benchmark or hardware-fault finding is available.

Collection update now edits the original path with Lofty, checks metadata
readback and discarded genres, and restores original modification/access times
on success OR returned write/readback failure. Unchanged tags skip writes.
Removed application-created temp copies, PCM hashes/decodes, picture comparison
passes, ownership copying/chown, xattr copying, replacement renames and forced
per-file/directory fsyncs. Native in-place editing keeps the existing inode,
ownership, permissions, xattrs and artwork. Read-only/hard-linked rejection,
file advisory locks and stale-source tag/inode checks remain. Failed folders
stay retryable and error text now acknowledges that some direct updates may
already have completed; no rollback copy is promised.

Metadata-only reads disable Lofty property/duration parsing and do not need
Track.duration; the collection-only internal unused duration is zero and never
written to tags or used in matching. Shared raw/native tag reads and the tag
writer's alternate-store lookup also skip unnecessary property parsing.
Staged --process still obtains/validates durations, copies and verifies audio
as before. Existing custom-tag/artwork preservation tests cover the backend.
Pictures/PCM validation helpers remain in tests, not the live collection path.
The backend may still shift encoded data if tags outgrow available padding.

New shutdown thread-local UnitTask defers cooperative check() cancellation for
the current metadata write/readback/timestamp-restoration unit. requested() is
still globally visible; other threads stop new work. After the guard drops,
normal stop behavior resumes. Stop-file, SIGINT and SIGTERM subprocess tests
exercise completion markers, restored timestamps, and cancellation visibility
on another thread. Forced termination/SIGKILL, power loss, or storage failures
cannot be made safe by this guard and may interrupt a direct edit. This is the
explicitly authorized direct-tagging tradeoff. Existing stop-file path remains
<output>/.bcfunl-stop; remember to remove it before resuming queued jobs.

README/reference updated. Local version v0.2.3, built 2026-11-15T06:52:20Z.
183 tests passed, 0 failed, 5 ignored. Strict locked Clippy, formatting, locked
release build, actual CLI smoke, build-workflow regression and Compose config
passed. Tests verify PCM/artwork/mtime preservation in the test suite, unchanged
inodes, xattrs, failed-edit timestamp restoration, skipped unchanged tags, and
that a file with damaged audio frames can still get a metadata-only update.
The latter proves the live update no longer requires audio decoding. The
existing shutdown matrix now includes collection units for file/SIGINT/SIGTERM.
check-build.sh remains blocked by Docker daemon access, so image/package and
live media2026 performance acceptance remain pending. No deployment/commit/push.

Known remaining scheduling behavior: collection still dispatches batches of N
folders and waits for each batch's slowest folder. This can leave workers idle
and delay progress/history output. It was identified but not changed in this
writer-focused update. Full directory enumeration precedes filtered work too.
--timings still applies only to staged --process. Measure the new lighter walk
before deciding whether scheduling or NAS metadata latency needs further work.

Final direct-write check: media files are opened read/write with O_NOFOLLOW before
exclusive locking, matching direct edits and NFS lock requirements. Latest locked
build/tests and CLI smoke passed after this adjustment. Native staging process
verification is unchanged; no live NAS speed measurement has been performed.

## 2026-11-15 handoff: v0.2.4 hard collection modification-date cutoff

Stue requested a hard modified-after date window, defaulting to today minus two
months. Implemented --fromdt for --collection only with file modification-date
semantics. Other modes retain their existing inclusive purchase-date filters
and no automatic default. Collection --todt remains unsupported.

The collection cutoff is computed once per run: explicit --fromdt or UTC today
minus two calendar months (end-of-month clamping, including leap years). Strict
UTC DATE comparison: the cutoff day itself is excluded; e.g. November 15 gives
September 15 and media modified September 16 onward qualifies. The CLI prints
the active cutoff. Qualification uses the newest supported media-file mtime in
each folder, not directory/artwork/sidecar timestamps or tag DATE/YEAR values.
Any qualifying media file admits the whole folder/release to keep tags consistent.
Date filtering occurs during enumeration before tag reads or per-release library
lookups and before --count admission. --force never bypasses the date window.
Use --fromdt 1900-01-01 for historical work. Summary includes outside-date-window
folders. Directory traversal/file timestamp reads are still necessary on NAS.

History skip now compares MAX(collection.walk_date) with the newest media mtime,
so a qualifying folder modified after its last completed walk is retried without
--force. Unchanged completed folders still skip. Direct metadata edits continue
to preserve mtime and do not make old files look new. New files copied with old
preserved mtimes may still need --force within an explicitly broad enough window.
No history schema or dependency changes were needed. A mechanical version edit
briefly touched an unrelated zerotrie lock entry; locked checks caught it and
it was restored exactly. Final Cargo.lock diff changes only bcfunl's own version.

README/reference/help updated with date semantics, strict boundary, default,
whole-release admission, history behavior and full-history override. Local
v0.2.4 built 2026-11-15T07:00:50Z. 185 tests passed, 0 failed, 5 ignored. New tests
cover calendar-month defaults/leap/end-of-month arithmetic, explicit cutoff,
UTC boundary, exclusion before count, --force obeying the cutoff, no tag reads
on an excluded invalid-container file, and modified-since-walk admission.
Strict locked Clippy, formatting, locked optimized build, actual CLI smoke,
workflow regression and Compose config passed. check-build.sh still blocked at
Docker daemon access. Live NAS performance/window acceptance and image/package
gates remain pending. No running jobs or live files were changed/deployed.
