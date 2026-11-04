## Next-session reminder: acquire limit

On 2026-10-31 (America/New_York), remind the user that the acquire cron job is
still capped at `--count 100`. Review whether to raise or remove that limit.
Do not change it automatically. No proactive notification was scheduled because
this session has no general reminder/scheduling tool.

# Project Memory

Updated: 2026-11-03
Current release: v0.1.40. Latest native media implementation details are in the final handoff entry.

## Purpose and preferences

`bcfunl` is a Rust CLI for the owner's private Bandcamp collection (user
`stuarthunter`, approximately 9,000 releases). Prefer the smallest complete
change; keep the implementation in Rust. Use US English spelling and avoid
em dashes in documentation, CLI text, comments, and identifiers. The user explicitly authorized
optional aria2c subprocesses for parallel transfers. Preserve the original
`AGENTS.md`; its initial description predates the application.

## Current behavior

- Named options: `--user`, `--fromdt`, `--todt`, `--count`, `--output`,
  `--format`, `--force`, `--cookies`, `--parallel`, `--summary`.
- Date filters use inclusive UTC purchase dates. Unknown purchase dates are
  excluded when filters are active. Count limits eligible download attempts;
  without a count the main collection is processed in full.
- Capture links in SQLite first, then download. Collection preorders,
  prereleases, future releases, and missing links are skipped. Hidden items
  use a separate endpoint and are not included.
- Prefer highest advertised lossless bit depth/sample rate. At equal or unknown
  resolution, honor the requested lossless container, then the fallback order:
  FLAC, WAV, AIFF, ALAC, AAC, Vorbis, MP3 320, MP3 V0. ALAC is capped at 48 kHz
  by Bandcamp, so AIFF precedes it by default. Lossy order is a preference,
  not a universal quality guarantee. Do not transcode or upsample.
- Default transfers are sequential. `--parallel 2..8` runs bounded aria2c
  workers with one connection per release, using the process-launch pattern
  inspected in `/data2/nuascn/src/main.rs`.
- Page requests and transfer launches wait a random 1.1-5 seconds. aria2c
  redirects do not use the Rust delay.
- `<output>/collection.sqlite3` records metadata and completion. Skip non-NULL
  download dates unless forced. Write successful completion only after atomic
  file publication. Preserve previous successful records on failed force runs.
- With --summary, dump artist, title, download date, attempts, and last error
  for this user's attempts with last_attempt_date >= process start. No table
  is printed without the flag; failures are included using the attempt timestamp.
  show `success` when a download date exists and last error is NULL. Otherwise
  show the stored error, or NULL for pending entries. Progress goes to stderr.
  Never display cookie values or signed download links.

## Code and operation

`main.rs` handles arguments, capture, scheduling, and history updates;
`collection.rs` parses cookies/page data; `net.rs` uses cancellable async HTTP
behind a synchronous interface; `download.rs` chooses formats and publishes
files; `aria2.rs` manages children/staging; `db.rs` binds system SQLite;
`shutdown.rs` handles SIGINT/SIGTERM. Tests also live in `tests.rs`.

Defaults: `bandcamp.cookies.txt`, output `downloads`, format FLAC.
Keep cookie exports and databases private and ignored. ZIP albums remain
archives. SIGINT/SIGTERM cancel work, reap children, clean owned partial files,
record already completed publications, dump the table if requested, and close SQLite.
SIGKILL/power loss cannot run cleanup. Never remove SQLite journals/WAL files.

## Verification context

The user confirmed the collection walker and host aria2c 1.35.0 work. This
agent's environment cannot resolve Bandcamp and its Snap aria2c launcher fails
confinement; do not diagnose these as failures of the user's installation.
Synthetic tests cover 100 releases, concurrency, persistence, force, and actual
SIGINT/SIGTERM subprocess cleanup. Live audio transfers remain unverified here.

## Runtime regression fixed

Reqwest 0.12 constructs its request timeout when `RequestBuilder::send()` is
called, before the returned future is polled. All sends now use the shared
`Network::send` helper, with the call inside an async block entered by Tokio.
Do not move `.send()` outside that block. The GET/POST regression test uses a
closed local port to exercise startup without live credentials or a server.
Test directories use randomized names to avoid stale-directory collisions.

## Downloads staging requirement

The user's Snap aria2c cannot create directories under the project/output path
(exit status 18). nuascn already uses `/home/stuart/Downloads`; this is a bind
mount onto other storage. Stage aria2c in visible `~/Downloads/bcfunl-*`
directories. Pass the HOME-based path as-is; never canonicalize it to the
backing path. Set `out=payload` inside each stdin input-file entry, since
aria2c ignores the global output option with input files. Rust copies completed
payloads into the destination's atomic saver, then removes staging after
success. Preserve the staged payload and print its path if publication fails;
graceful cancellation still removes the current run's staging. The default
sequential path continues writing directly to the requested output directory.

## Attempt tracking

`releases.attempts` is a non-NULL integer defaulting to zero. Database opening
adds it transactionally to legacy tables without changing existing history.
Increment once before processing a selected release in either transfer mode;
skipped completed releases do not increment. Preparation failures, retries,
force runs, and interrupted attempts count. Historical attempts cannot be
reconstructed and remain zero after migration. Display attempts in the dump.

## Personal preferences

Address the user as Stue, not Stuart. Use US English and no em dashes.
The user explicitly authorized maintaining the global Codex instructions file.
The global file `/home/stuart/.codex/AGENTS.md` is now installed and its contents
were verified. Stue installed it from the prepared file using
`cp --update=none /tmp/stue-global-AGENTS.md ~/.codex/AGENTS.md`.
Use `cp --update=none` instead of `cp -n` for no-overwrite copies on this system.
The agent's global directory was read-only when writing was attempted; user
authorization does not override filesystem restrictions.

## Download-history investigation

Stue's test command uses `--user stuarthunter --count 30 --output /data2/bcfstg
--format FLAC --parallel 4`. Its history is `/data2/bcfstg/collection.sqlite3`,
not the project's default downloads database. Read-only inspection at the time
found 100 entries: 26 completed, 42 payload-validation failures, and 32 with no
completion date or error. These are historical counts, not current totals.
The retained payload samples were Bandcamp download-page HTML, not audio.
The completion skip is present in capture and both processing paths; failed
validation leaves download_date NULL and therefore eligible for retry. The
HTML response cause remains unresolved. Do not mark these attempts successful
merely because aria2c exited zero. Successful publication clears last_error;
a failed forced rerun preserves an older download_date while recording its error.

## Run summaries and expanded formats

`last_attempt_date` is migrated as nullable TEXT and updated with the attempt
counter before processing. Run start and attempt timestamps use fixed-width UTC
RFC3339 nanoseconds so the inclusive SQL comparison is exact. `--summary`
filters by both username and last_attempt_date, including failures and forced
retries whose download_date is older. Without --summary there is no final table.
Another concurrent process using the same user/history can meet the time filter;
this is a timestamp filter, not a separate per-run event ledger.

All eight requested encodings now have selection and single-track header/extension
handling. AIFF/AIFF-C use .aiff, ALAC/AAC use .m4a, Vorbis uses .ogg; ZIPs stay
ZIPs. Header validation does not decode audio or distinguish AAC from ALAC
inside M4A. Lossy CLI names keep the quality-first fallback order.

## Version 0.1.2 handoff

The release includes --summary and the expanded eight-format fallback. Summary
output is now a Unicode-width-aware aligned table with readable headers, a
separator, and right-aligned attempt counts. It retains full values and escapes
terminal control characters. Without --summary, no status table is printed.
The filter remains current username plus last_attempt_date >= process start.
Formats use the corrected default order FLAC, WAV, AIFF, ALAC, AAC, Vorbis,
MP3 320, MP3 V0. Bandcamp's documented ALAC sample-rate limit is 48 kHz.
Prefer higher advertised lossless resolution before a requested lossless
container; the lossy ordering is only a preference, not a quality guarantee.
No source format is converted. HTML payloads were inspected; broker URL resolution has since been added, pending live confirmation.

Version 0.1.2 verification: 29 tests passed, plus the subprocess fixture invoked
by its parent; Clippy, formatting, and the offline release build passed.
CLI checks verified summary output is gated by --summary, including the
empty-run/error case. Live Bandcamp/Snap transfers were not rerun here.

## Payload review and staging cleanup

Stue requested inspection before cleanup of
`/home/stuart/Downloads/bcfunl-258991-08a164a98c37e1a3/payload`.
The inspected 235,970-byte file was Bandcamp download-page HTML for Sleaford Mods,
not the neighboring Cabaret Voltaire progress line. Stue confirmed Sleaford Mods
was the previous concurrent transfer. Error lines now include artist/title.
The review is in PAYLOAD_REVIEW.md without signed URLs or private account data.

Shared format preparation now resolves broker /download/album|track links via
/statdownload/, accepts validated download_url/retry_url responses, parses
JSON/JSONP without execution, and caps pending polls at twelve paced requests.
This missing protocol step is supported by yt-dlp's source; live confirmation
is still required. Do not equate a zero aria2c exit with validated audio.

At parallel startup, remove recognized stale Downloads/bcfunl-<pid>-<16hex>
directories. Preserve live/inaccessible owners, symlinks, unrelated files and,
on Linux, paths used by active --dir process arguments (including orphans).
Invalid/empty payloads are now removed immediately; valid files retained after
publication failure are removed by the next stale-directory cleanup. Actual
home-directory files were not manually deleted in this restricted session.

## Version 0.1.3: HTML recovery, cookies, diagnostics, and names

Stue reported all 30 releases failing with only "Resolve Bandcamp audio download"
after the mandatory status resolver was added. That change was a regression:
remove the mandatory lookup and try direct links first. Exact live endpoint
errors were hidden by outer-context-only formatting; full sanitized error
chains now go to stderr and last_error. HTTP errors remove their URL upstream.

Shared with_recovery allows three transfer tries per processing invocation.
When HTML is returned, parse pagedata, verify the original artist/title,
select a fresh preferred format, optionally use /statdownload/, and retry.
Ordinary stat failures fall back to the HTML page's direct link; explicit
verification challenges stop. Pages are limited to 2 MiB. Recovery never marks
HTML as success and preserves original names for identity checks. The attempts
column still increments once per processing invocation, not per internal try.

Cookie matching now respects Bandcamp subdomains. aria2c previously received
no cookies; it now loads a mode-0600 Bandcamp-only Netscape export inside its
visible Downloads staging directory. Its cookie guard removes that file even
when payload retention is enabled. Do not log cookie values or signed URLs.

Stue also requested initial capitals for artist/title and filesystem names,
except when the entire original value is uppercase. Apostrophes start words:
jim o'rourke -> Jim O'Rourke; steamroom 68 -> Steamroom 68; SWANS stays SWANS.
Apply normalization during collection capture; original page names are kept
separately for recovery. Existing completed files are not renamed or redownloaded.

0.1.3 verification: 39 tests pass, plus the ignored fixture invoked by its parent.
Checks cover HTML-to-fresh-audio recovery, a three-transfer cap, identity/origin
validation, detailed errors, cookie scoping/private export cleanup, and initial
capitals alongside the earlier suites. Live Bandcamp/Snap transfers remain
unverified in the restricted agent environment. Rebuild/restart before testing.

## Live acceptance and media2026 handoff

Stue supplied two successful live summaries from version 0.1.3 on
2026-10-25 UTC. The first batch completed 30/30 previously failing releases;
older attempt counts incremented and every last_error became success. A second
run reused the same database without --force and completed the next 30/30
releases, each on attempt 1. Stue observed the expected next-batch selection.
This validates actual downloads, retry/error clearing, history preservation,
skip-and-advance behavior, and run-scoped summaries on Stue's host. It
supersedes earlier notes that live confirmation was pending. The agent itself
still has network/Snap restrictions; those are not problems with Stue's setup.

The recovery/cookie changes are live accepted; the exact contribution of each
fix to the old HTML response was not isolated. No additional fixes are required
on the basis of the old failures. Do not erase or reset the working database.

Stue will move the solution to media2026 and queue the roughly 9,000-release
collection. No remote transfer or full run was performed by the agent. Keep
version 0.1.3, its source/Cargo.lock, and private Bandcamp cookies. After the
utility has stopped, transfer the output tree together with collection.sqlite3
so completion history and files stay together. Prefer the same /data2/bcfstg
output path on the new machine; stored file_path values are absolute. If the
path changes, completed rows still skip based on download_date rather than
checking that stored files exist. Do not discard SQLite journal/WAL files.

media2026 needs aria2c on PATH and the system SQLite runtime; building there
also needs Rust/Cargo and SQLite development files. ~/Downloads must exist or
be creatable, with enough backing storage for parallel staging; preserve the
visible Downloads path if it is a bind mount. Use the same cookies filename or
supply --cookies explicitly. Never expose cookie values or signed links.

Full-run command, intentionally omitting --count and --force:

```sh
./target/release/bcfunl --user stuarthunter --output /data2/bcfstg \
  --format FLAC --parallel 4 --summary
```

The utility captures all eligible links before processing, so the initial
collection walk precedes audio transfers. The full 9K run and execution on
media2026 remain untested. Latest code checks remain 39 tests passed plus the
parent-invoked subprocess fixture, Clippy, formatting, and release build.
This handoff update changed documentation only; tests were not rerun.

## Version 0.1.4: ASCII-only initial capitals

Stue requested that Greek and other non-ASCII letters retain their original
case. The shared initcap helper now runs only when ASCII a-z is present and
uses ASCII-only case conversion, preserving all non-ASCII characters. Existing
word boundaries and all-uppercase preservation remain. Examples: µ-Ziq stays
µ-Ziq, αβγ stays αβγ, and jim o'rourke becomes Jim O'Rourke. This applies to
captured metadata, new folder/file names, and download-page identity matching.
Existing completed files are not renamed, and completed rows still skip.

Version 0.1.4 verification (2026-10-24): 39 tests passed, with the ignored
subprocess fixture invoked by its parent; formatting, Clippy with warnings
denied, and offline release build passed. Cargo.toml and Cargo.lock are updated;
the release executable reports bcfunl 0.1.4. Tests include mixed ASCII/Greek,
non-ASCII lowercase-only names, and the micro sign regression. No live downloads
were rerun for this casing change. The full 9K run remains unverified here.

## Release genres and country metadata (2026-10-25)

Added nullable releases.genre and releases.country_iso through the existing
transactional, idempotent schema migration. Shared download preparation now
reads the public release page even when publication date is already known,
and stores metadata before audio transfer in both sequential and aria2c modes.
The existing page read is reused for publication/preorder checks. SQLite stays
on the main thread. Missing public URLs leave the fields NULL.

src/metadata.rs extracts .tralbum-tags a.tag, removes duplicates in page order,
and joins retained tags with semicolons. Exclude sidebar location components,
recognized country names/aliases, and explicitly marked geographical links.
Other artist tags remain; arbitrary geographic tags unrelated to the sidebar
cannot always be distinguished from genres. Country is derived from the artist
sidebar #band-name-location .location using bundled English ISO names and
aliases (UK -> GB), with city/full-US-state support. Unknown/city-only locations
and ambiguous city/Georgia stay NULL rather than guessing. The page owner can
be a label, so this does not establish an individual performer's nationality.
Country data and its LGPL notice/license are bundled under data/; no new runtime
dependency or external geocoding service is required.

Completed releases still skip without metadata backfill. Do not recommend
--force just to enrich history: it also redownloads audio. Metadata does not
change the summary columns, completion dates, attempts, or file names.

Validation: 41 tests passed plus the parent-invoked signal fixture; formatting,
Clippy with warnings denied, and offline release build passed. Existing legacy
migration tests now check both new nullable columns. Fixtures cover Belfast,
London, Tokyo, Portland, unknown/ambiguous locations, tag filtering/deduplication,
and preservation of completion history. Version remains 0.1.4. Raw live HTML
could not be fetched from this environment (DNS restriction); user-host live
validation of the new extraction remains pending.

## Persist public release URLs (2026-10-25)

The collection already provided item_url as Release.release_url in memory for
public page metadata. It now also persists in nullable releases.release_url,
separate from download_url, through the existing migration and collection upsert.
Recaptured completed rows get the current URL before the completion skip;
missing/empty incoming URLs preserve existing stored values. This does not
backfill genre/country for skipped releases or change attempt/completion history.
Tests cover insertion, updates on completed rows, missing-URL preservation, and
legacy migration. Latest checks: 42 tests passed plus the parent-invoked signal
fixture, Clippy with warnings denied, formatting, and offline release build.
Version remains 0.1.4; no live requests or user databases were modified.

## Origin rejection investigation (2026-10-25)

Stue reported Black Dog Liber Eleven and Feral Grace failing with bare
"Rejected unexpected URL origin". Release-page GET previously lacked context;
other download stages have context. The exact stored URLs were unavailable to
the agent, so the source/scheme cause is probable, not confirmed. Asked Stue for
only the public release_url. Bandcamp requests already scope cookies by host,
path and expiry on every redirect; rejection occurs before cookie/network use.

Network GET now upgrades legacy HTTP Bandcamp URLs to HTTPS before validation,
including redirects, while continuing to reject external hosts, credentials,
nonstandard ports and HTTP CDN URLs. Release GET errors now say "Fetch release
metadata page". Rejected-origin details show only scheme, host and port, never
paths, query tokens or credentials. No external/custom-domain access was added.
43 tests passed plus parent-invoked fixture, Clippy, format, offline release
build. No live reproduction here due DNS restrictions; version remains 0.1.4.

## Latest output inspection (2026-10-25)

Stue supplied shell listings for /data/bcfstg2, a different output root from
the earlier /data2/bcfstg tests. It contains collection.sqlite3 (12,288 bytes)
and stuarthunter/ with The Xx/ and Framewerk/. Framewerk contains:
Framewerk Selects Vocals Vol 2 (Limited Release) - album1324801722 [flac].zip
at 2,857,413,652 bytes, timestamp Oct 25 09:11. This confirms the listed file
exists, not that its ZIP contents, SQLite completion, genre/country fields,
Black Dog retry outcome, or the full collection have been verified. The listing
does not establish which executable/version or command produced the file.

Use /data/bcfstg2/collection.sqlite3 for any follow-up on this output, rather
than assuming the old database path. The correct username directory is
stuarthunter/, not stuart/hunter/. ll is unavailable in the user's command
shell; use ls -ltrA. history produced no command history in that shell.
No databases or payloads were opened or modified for this documentation update.
Code and release version remain 0.1.4; latest checks remain 43 tests passed,
plus the parent-invoked signal fixture, formatting, Clippy, and release build.
Tests were not rerun for this documentation-only update.

## Confirmed Black Dog origin cause and canonical host fix (2026-10-25)

Stue supplied collection data showing Black Dog item_url uses the custom host
duststoredigital.com, while url_hints.subdomain is theblackdog. This confirms
the initial release URL was outside the allowlist, rather than a redirect-status
or cookie error. The earlier HTTP explanation was a hypothesis only.

Collection capture now replaces a custom item's host with the validated single
DNS label from url_hints.subdomain plus .bandcamp.com, forces HTTPS/default port,
and preserves path/query. The canonical URL is used for metadata fetching and
stored as release_url, including updates to encountered existing rows. Normal
Bandcamp URLs remain unchanged. Missing hints leave the original URL subject
to the existing request validation; no arbitrary external hosts are allowed.
Tests use sanitized album/track examples, invalid hints, and SQLite capture.
44 tests passed plus parent-invoked signal fixture, Clippy, formatting, and
release build. No live Black Dog retry performed. Version remains 0.1.4.
Do not copy the supplied account data or signed links into fixtures/docs.

## Canonical release redirect back to custom host (2026-10-25)

Stue confirmed canonical release_url is stored but Black Dog still fails at
Fetch release metadata page with host=duststoredigital.com. The canonical
Bandcamp page can redirect back to the custom domain; rewriting only the initial
URL was incomplete. Release retains an in-memory custom host only when the
collection marks custom_domain_verified as 1/true and it exactly matches the
HTTPS/default-port, credential-free original item_url host.

Network.release_page uses the shared GET loop with that exact host additionally
allowed for this release's metadata request only. Every redirect still validates
scheme/credentials/port/host, remains paced and bounded, and recomputes cookie
scoping. No Bandcamp cookies go to external hosts. Normal download/CDN requests
have no custom-host allowance. SQLite continues storing the canonical Bandcamp
release_url. This supersedes the prior statement that no custom-domain access
was added. Missing/unverified/mismatched custom metadata stays disallowed.

46 tests passed plus parent-invoked fixture, Clippy, formatting, and offline
release build. Tests cover verified-host extraction, scope rejection, and cookie
non-disclosure. Live redirect chain/transfer still not reproduced in the agent
environment. Version remains 0.1.4; rebuild/restart before retrying.

## Black Dog redirect loop remains unresolved (2026-10-25)

Stue's next run reached the six-request limit on both Black Dog releases after
the verified custom host was permitted. That follow-up did not fix the loop.
Do not claim otherwise or increase the redirect limit as a fix. Exact live hops
are unavailable in this agent's DNS-restricted environment. Added safe failure
trace entries with source origin, HTTP status, and target origin only. No URL
paths, queries, userinfo or cookies appear. This will distinguish host bouncing
from HTTP/HTTPS redirects on the next user retry. Root cause is still pending.
47 tests passed plus parent-invoked fixture; formatting, Clippy and offline
release build passed. Version remains 0.1.4. Diagnostic change only, not a
verified redirect-loop fix.

## Reboot checkpoint (2026-10-25)

Stue is rebooting. No downloader process was started or stopped by the agent.
Resume with the unresolved Black Dog release-page redirect loop. The latest
release binary is built at target/release/bcfunl, version 0.1.4, with sanitized
redirect-chain diagnostics. Next step: Stue reruns the same cargo run --release
command without --force and shares the new error containing origin/status hops.
Use the user's actual output path; the latest reported root is /data/bcfstg2,
with history at /data/bcfstg2/collection.sqlite3. Do not reset that database or
remove completed downloads. Failed releases remain eligible; completed rows skip.
Latest validation remains 47 passed plus parent-invoked signal fixture, Clippy,
formatting, and release build. This checkpoint is documentation only.

## Incognito and command-line comparison (2026-10-25)

Stue confirms Feral Grace opens in an unsigned-in Incognito window. Awaiting its
final public address (without query) to compare hosts. The provided latest run
shows a repeating canonical Bandcamp 301 -> duststoredigital.com 303 -> Bandcamp
chain. The Xx completed, Black Dog failed; Ctrl+C left Framewerk and David Holmes
incomplete with NULL last_error, consistent with cancellation handling.

Agent curl access became intermittently available. Anonymous and exported-cookie
requests to the canonical URL both returned 301. Following with a browser-style
user agent, a response cookie jar, or HTML Accept/language/no-cache headers still
hit the same six-redirect loop. The custom-domain 303 probe had no Set-Cookie;
Bandcamp responses did set cookies, but retaining them did not fix curl's loop.
Therefore do not claim either cookie persistence or user agent is the confirmed
cause. Public page text is accessible through cached web browsing, which does
not prove the live HTTP chain works. Raw headers/bodies and the temporary public
cookie jar stayed in /tmp and were not printed or added to the repository.

No downloader code/dependency changes were made for these comparisons. An attempt
to check reqwest's optional cookie feature could not fetch crates due DNS; that
feature was not enabled and no cookie dependency was added to Cargo.lock.
Latest implemented code remains version 0.1.4 with 47 passing tests plus the
parent-invoked fixture, Clippy, formatting, and release build. Redirect cause and
fix remain unresolved; avoid another speculative patch or larger redirect cap.

## Black Dog redirect fix: full browser-compatible release user agent (2026-10-25)

Stue confirmed Incognito ends at theblackdog.bandcamp.com/album/feral-grace.
Live curl comparison isolated a user-agent difference: bcfunl/0.1.4 and minimal
Mozilla/5.0 loop, while a full Chrome-compatible user agent returns HTTP 200
with data-tralbum and genre markup. This succeeds for both Feral Grace and Liber
Eleven. A Feral Grace probe also succeeded with the existing exported cookies
and the bcfunl/0.1.4 identifier appended. Thus the earlier generic statement that
browser-style headers did not help applied only to the incomplete user agent.

Network.get_request now overrides the user agent for release_page requests only,
including their redirect hops, using full Mozilla/WebKit/Chrome/Safari tokens
plus bcfunl/<Cargo version>. Collection/download/audio request defaults remain
unchanged. Existing scoped cookies, origin checks, pacing, six-request limit,
and sanitized diagnostics remain. No cookie-jar dependency was added.

Latest checks: 48 tests passed plus parent-invoked signal fixture; Clippy with
warnings denied, formatting, and release build passed. The initial new test
incorrectly expected client-default headers at RequestBuilder.build time; fixed
to check ordinary requests have no override, since defaults apply at send time.
Version remains 0.1.4. Live public metadata page GETs succeeded via curl with the
chosen header; a full downloader retry and Black Dog audio success remain to be
confirmed by Stue. No output databases or downloaded audio were modified here.

## Live metadata acceptance and media2026 sync request (2026-10-25)

Stue supplied a successful five-release run against /data/bcfstg3 with --parallel
8 and --summary. Both Black Dog releases, Framewerk, and two David Holmes
releases completed; prior errors cleared. The subsequent /data/bcfstg3 SQLite
listing confirms six total completed rows including the earlier The Xx release,
with genre lists and canonical release URLs. GB is recorded for The Xx,
Framewerk and David Holmes; Black Dog country stays NULL because its sidebar
only says Sheffield. Stue accepts metadata holes and will fill them during
staged-file processing. This live evidence supersedes pending downloader retry
notes above. /data/bcfstg2 is an older database, not this successful run's history.

Stue requested rsync of code to media2026:/data2/bcfunl, where he copied the
project around 6 PM yesterday. The existing SSH alias maps to media2026.local,
user stuart, with his configured identity. In this agent environment the default
SSH config reports bad permissions on the system proxy include. Explicitly using
/home/stuart/.ssh/config bypasses that issue but media2026.local still fails DNS.
An IP was requested. Do not say the SSH alias is missing; it exists but its target
hostname cannot resolve here. No remote files have been transferred yet.
Sync only source, data, Cargo manifests/lock, .gitignore and project docs. Do not
copy cookies, download history, payloads, target builds or environment-managed
directories; do not delete remote files or overwrite remote AGENTS.md.

## Version 0.1.5: offline processing pipeline (2026-10-25)

Stue authorized implementation of the agreed single-tool processing path. CLI:
--process --output <existing staging> --serverdir <library> --template <rule>.
The exact spelling is --serverdir. Default template:
{artist}/{album} [{country_iso}] [{catalog_num}]/{disc}-{track} - {title}
Missing bracketed country/catalog sections disappear; disc/track numbers have at
least two digits and the actual extension is appended. Paths are sanitized and
traversal, unknown placeholders, collisions and inconsistent release roots fail.
Disc subdirectories are supported after the stable album directory. --parallel
is downloader-only and conflicts with --process. Count and purchase-date filters
apply to processing; --summary prints attempted/published/skipped/failed counts.
No --filter was added: Stue clarified he was remembering mcfunl's flag.

Processing uses completed SQLite rows and embedded tags only. No network/cookies
or Discogs/MusicBrainz/Bleep fallback is included. Stue may add provider gap-filling
later, after seeing how far existing data takes us. Unknown metadata stays absent.
The new src/processing module adapts credits, FLAC/WAV/MP3 tag I/O and private
staging from mcfunl, with no sibling-project dependency or new Cargo dependency.
The partner session's /data2/mcfunl code was inspected read-only and not changed.

Preserve multi-valued ARTIST, TRACKARTIST, PERFORMER, ARTISTSORT, REMIXER and
existing album artist/sort, composer/conductor, identifiers and custom tags.
Use mcfunl's explicit featuring/remix credit parser; commas and band ampersands
are preserved. Album artist stays primary; compilations keep track performers.
Label compilation evidence uses multiple performers, a nonperforming storefront,
and matching artist-prefixed track titles; sets Various Artists and preserves
the label. A sole known performer fills missing album artist before storefront.
Source country/catalog/album tags precede SQLite fallbacks; ALBUM includes known
country/catalog suffixes. SQLite genre lists become multi-valued genre tags.
Cover images survive, with folder.jpg copied from the archive or extracted from
an embedded picture when available. Original source archives/files stay intact.

Linux processing requires 7z for ZIP, ffprobe, ffmpeg and metaflac for FLAC tags.
All advertised download formats have tested container handling: FLAC, WAV, MP3,
AIFF, ALAC/AAC in M4A and Vorbis/Ogg. Other containers use audio stream copy,
not audio re-encoding. Validate decoded float64 PCM SHA-256 plus sample rate and
channel count, and read every expected tag back before atomic publication.
CUE inputs and unsupported RF64/ID3 variants require existing mcfunl/manual review;
failures leave sources and existing destinations intact. Uncertain required
identity/track metadata and conflicting values also fail for review.

Existing exact destinations and mcfunl-style artist/album folders are verified
by track count, disc/track identity, title, duration and PCM/layout hashes. This
recognizes different filenames and country/catalog folder suffixes. Different
existing audio is an error, never overwritten even with --force. Processing
--force retries completed work; download --force keeps its previous behavior.
Already processed outputs can skip via completion history plus the manifest and
present nonempty track files. External outputs without our manifest reverify.

SQLite migration adds catalog_num, process_date, process_error and processed_path.
Processing does not increment download attempts or alter download completion/error.
A successful new download clears prior processing completion so changed payloads
are reconsidered. Completion is saved only after verified server publication or
a verified existing match. Publication uses Linux renameat2 RENAME_NOREPLACE on
the server filesystem. A per-library processor lock prevents concurrent writers
using this tool; parent/path/symlink checks protect output paths. SIGINT/SIGTERM
reap processing children and remove staging; SIGKILL can leave hidden staging.
The server filesystem must support atomic no-replace directory renames, and
extraction plus prepared copies need temporary space. Processing is sequential.

0.1.5 validation: 63 tests passed plus the parent-invoked signal fixture, now
covering processing SIGINT/SIGTERM too. Formatting, strict Clippy, offline release
build and CLI help/version checks passed. Tests cover all supported containers,
credits/remix/compilation rules, layout-aware audio hashes, templates, migrations,
server mismatches, artwork, error history, repeat skips and download invalidation.

Real local smoke: copied the complete 14-track The Xx Coexist (Deluxe Edition)
archive into disposable /tmp staging with a minimal metadata-only database (no
private signed links copied). Successfully tagged, audio/tag verified and
published all tracks using the release binary. Repeat skipped. Renamed the test
folder suffixes and every track filename and removed the manifest to simulate a
prior external deployment: verified audio matched and skipped without replacing.
The final 0.1.5 build also published all 14 tracks to a fresh disposable server
root. Original /data/bcfstg3 database was opened read-only; its downloads were
only read/copied. No real library or remote server files were changed.

media2026 update from Stue: 4,977 complete + 3,716 pending = 8,693 captured versus
the earlier 8,707 collection count. The 14-item difference is not diagnosed;
exclusions and test history are possibilities. Stue plans a final --count 9000
run without --force after this batch to catch eligible stragglers. Do not launch
another downloader, interrupt the active batch, or deploy this processing release
without a new instruction. Version 0.1.5 is local only; production NAS/full-library
processing and large-batch performance remain unverified.

Disposable real-album smoke copies were removed after verification. Earlier
agent-created curl response/header files and the temporary public cookie jar
were also removed; originals, live cookies and production data were untouched.


## 2026-10-25: 0.1.6 credits and requested skill agent

Fixed David Holmes repeated artist/featured album prefixes and primary artist
policy, preserving separate TRACKARTIST/PERFORMER guests and REMIXER. Faithful
native tag readback replaces repeated semantic inference, avoiding GLOK/Glok
ARTIST verification drift. 66 tests plus parent fixture, fmt, strict Clippy and
release build passed. Full disposable Vol. 1/2 copies (19 tracks) published with
verified unchanged audio. Production outputs unchanged; older wrong folders
need explicit repair or a fresh test destination.

Stue explicitly requested copying /data2/mcfunl/tagskill.md and standing up an
agent with it. Copied to repository tagskill.md and spawned tag_sanitize_trial
for a read-only Vol. 2 handoff under reviews/. User primary artist/guest policy
overrides the skill's album guest conservatism. No connected music MCP server
exists here. Separate mcfunl resolver dry run skipped due to provider DNS errors.
Public Discogs release 30152429 retrieved successfully separately; structured
Remix extraartists can resolve composite title-parser credits. No provider
import path or runtime service has been added to bcfunl.

Stue confirmed genre and style belong together in GENRE multi-values. Updated all seven Vol. 2 agent proposals with existing genres plus Discogs genres/styles normalized using VCW genre.dat; case-insensitive unique values, source Alternative retained, Acid House expands to House. No media writes or runtime enrichment integration yet.

Confirmed standardized genre routine: vripr src/metadata/genre.rs sanitize_genres; VCW shared crate vcw_metadata::Genres::builtin().normalize_all, used by mcfunl src/main.rs. VCW Discogs provider already chains genres and styles before normalizing. Updated local tagskill.md to name shared routine explicitly; bcfunl runtime integration still pending.

Shared genre integration completed locally in 0.1.6: Cargo dependency on
../vcw/crates/metadata default-features=false, matching mcfunl. prepare_tags
constructs one genre map per album and normalizes combined embedded/release
genres, replacing prior SQL-only overwrite. Focused edge-case test and full
67-test suite passed, one parent fixture ignored; strict Clippy/fmt passed.
README documents new source build sibling checkout requirement. Runtime binary
does not need VCW files. No provider lookup/import integration or live media
changes performed.

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
