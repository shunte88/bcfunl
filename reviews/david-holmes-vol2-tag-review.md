# David Holmes Vol. 2 tag agent trial

Executed the copied `tagskill.md` as a read-only tag collection task against seven disposable FLAC files. No media or production outputs were modified. The JSON handoff contains actual native multi-value original tags and proposed field-level patches.

[Discogs release 30152429](https://www.discogs.com/release/30152429-David-Holmes-Blind-On-A-Galloping-Horse-Remixes-Vol-2) agrees on primary artist, all seven ordered titles after punctuation/case comparison, and all seven durations within 0.84 seconds. WAV versus FLAC is an edition limitation: this validates recording content for credit enrichment, not an identical container edition.

| Track | Proposed remixer correction |
| --- | --- |
| 3 | Replace `Hardway Bros Meet Monkton Uptown` with separate `Hardway Brothers` and `Monkton` values. Discogs also supplies the printed alias `Hardway Bros`. |
| 4 | Add `X-Press 2`, missed by the current title parser. |
| 5 | Add `Cosmodelica`, missed by the current title parser. |
| 6 | Replace `Sonic Boom & Panda Bear Reset` with separate `Panda Bear` and `Sonic Boom` values in structured provider order. `(2)` is retained in evidence as the Discogs name disambiguator. |

Each correction also updates TRACKARTIST and PERFORMER while keeping David Holmes first and Raven Violet second, as Stue explicitly requested. ARTIST, ALBUMARTIST, ARTISTSORT and ALBUMARTISTSORT already contain only David Holmes. Titles, mix/version distinctions, album identity, country suffix, artwork, dates, genres and unrelated fields are preserved.

Tracks 1, 2 and 7 need no tag changes under this scope. Track 7 has no structured Discogs remixer credit; its existing Ammonite value remains supported only by the explicit source title. The provider catalog is `none`, so it stays missing. Raven Violet on every track is user policy, not provider-confirmed per-track participation, including the instrumental mixes.

Result: 7 ready patches including combined genre/style values, 0 applied writes. No automatic JSON-import path exists yet. MusicBrainz and Bleep were not needed to identify these corrections and were not queried. Playback compatibility and audio hashes were not checked because no media was written.

Genre enrichment combines existing genres with Discogs genres and styles in one multi-value GENRE collection, using the existing VCW genre map and case-insensitive deduplication. All seven tracks receive: `Electronic`, `Alternative`, `Synth-pop`, `Acid House`, `House`, `Progressive House`. House comes from the established Acid House mapping. Existing Alternative is retained. These are separate values, not a joined string.
