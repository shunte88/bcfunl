# Payload Review

Reviewed before adding staging cleanup.

## Inspected artifact

`/home/stuart/Downloads/bcfunl-258991-08a164a98c37e1a3/payload`

The file is 235,970 bytes of HTML, not a ZIP archive or audio file. Its Bandcamp
page data identifies **Sleaford Mods - The Demise Of Planet X (Deluxe)**. Stue
confirmed this was the previous transfer; parallel progress lines can precede
results from earlier workers. Failure messages now identify the release.

The page advertises downloads and includes separate FLAC/WAV links on
`popplers5.bandcamp.com/download/album`. Those links contain encoding, item ID,
and signature parameters. No cookie values, email addresses, signed links,
or full page data are reproduced here.

## Likely missing step

The implementation handed the advertised broker URL directly to aria2c.
The inspected payload demonstrates that a successful HTTP transfer can deliver
another download page. A successful aria2c exit alone cannot mark that complete.

[yt-dlp's Bandcamp implementation](https://github.com/yt-dlp/yt-dlp/blob/master/yt_dlp/extractor/bandcamp.py)
uses a `/statdownload/` request to resolve format links and reads the returned
download/retry URL. The shared Rust preparation path now performs this step,
parses JSON/JSONP without executing JavaScript, validates the returned origin,
and polls pending responses within a bounded limit. Both transfer modes use it.
This is an evidence-based fix for a missing protocol step, not live confirmation
that every previously failing release now downloads correctly.

## Cleanup behavior

At parallel startup, remove only recognized `~/Downloads/bcfunl-<pid>-<hex>`
directories whose owner is no longer running. On Linux, also preserve directories
referenced by active transfer `--dir` arguments, including orphaned workers.
Skip symlinks, unrelated files, malformed names, and live/inaccessible owners.
New staging directories start empty. Invalid HTML/empty payloads and failed
partial transfers are cleaned up; valid completed payloads remain recoverable
when destination publication fails. SQLite and final output files are untouched.

The inspected artifact was read, not manually removed from this restricted
session. Cleanup takes effect when the rebuilt utility next runs in parallel
on Stue's host. Actual Bandcamp status responses still need live verification.

## Follow-up in 0.1.3

The mandatory /statdownload/ lookup introduced in response to this artifact
caused all thirty releases in Stue's next run to fail before a transfer began.
It is no longer required. Direct downloads are tried first, and returned HTML
can now supply a fresh link for bounded recovery. Status lookup is optional in
that recovery flow; ordinary failures fall back to the page's direct link.
Bandcamp-domain cookies now reach the broker and aria2c through scoped cookie
handling. Detailed error chains replace the generic resolver label. These
changes are fixture-tested and still require a live rerun to establish which
response caused the original HTML behavior.
