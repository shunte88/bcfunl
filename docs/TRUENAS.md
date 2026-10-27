# Stue's TrueNAS SCALE 25.10 deployment

Deployment paths supplied by Stue:

| Content | Path |
| --- | --- |
| Application, maps, cookies, logs | `~/bcfunl` under the task account's persistent home |
| Download staging and collection history | `/media/backup/bcfstage` |
| Published music | `/mnt/tankn4/F8NAS1/music` |
| Default research library | `/media/backup/bcfstage/resources/library.sqlite3` |

These are prepared commands, not installed or enabled tasks. Wait for the NAS
update to finish before checking the runtime. Use the same task account for all
jobs, with a persistent home and read/write access to both datasets.

## Runtime check

The executable below is `~/bcfunl/bcfunl`, not a Cargo build tree. Copy the release
binary and `data/` directory there, and privately supply `bandcamp.cookies.txt`.
Keep the existing collection database with its staged payloads; do not initialize
a different empty staging directory for an existing collection.

Run in the environment that will execute bcfunl:

```sh
cd "$HOME/bcfunl" || exit 1
./bcfunl --version
for tool in aria2c 7z ffprobe ffmpeg metaflac gzip sha256sum; do
    command -v "$tool" || printf '%s: MISSING\n' "$tool"
done
ldd ./bcfunl
for directory in /media/backup/bcfstage /mnt/tankn4/F8NAS1/music; do
    test -d "$directory" && test -r "$directory" && test -w "$directory" \
        || printf 'Directory missing or not accessible: %s\n' "$directory"
done
mkdir -p logs "$HOME/Downloads"
chmod 600 bandcamp.cookies.txt
```

The version command must run successfully, `ldd` must report no missing libraries,
and all utilities/directories must be available before enabling jobs. In
particular, a binary built on newer Ubuntu may need a newer glibc than the NAS
provides. Rebuild in the chosen Linux environment if that happens.

TrueNAS is an appliance. If host utilities are missing, install bcfunl and its
utilities in a Linux container or VM and expose the datasets at the same paths
inside it. The direct commands below then run **inside** that environment; they
are not host-side container-launch commands. Do not enable unsupported appliance
[developer mode](https://www.truenas.com/docs/scale/systemsettings/advanced/developermode/)
just to install media tools. See TrueNAS
[container and dataset configuration](https://www.truenas.com/docs/scale/containers/managingcontainers/).

## Initial processing check

Start without cleanup, using a small number of releases:

```sh
cd "$HOME/bcfunl" && ./bcfunl --process --user stuarthunter \
  --output /media/backup/bcfstage --serverdir /mnt/tankn4/F8NAS1/music \
  --parallel 2 --count 2 --summary
```

Existing matching server releases skip; conflicting audio is never overwritten.
Inspect published files and history before enabling the cleanup schedule.
Cleanup permanently removes eligible staged payloads. It preserves collection
history, the music library, failed sources and unverifiable publications.

## Scheduled tasks

If running directly on the NAS with a verified complete runtime, create tasks in
the TrueNAS Cron Jobs UI using the account whose home contains `bcfunl`. See
[Managing Cron Jobs](https://www.truenas.com/docs/scale/systemsettings/advanced/managecronjobs/).
If using a Linux VM/container, configure equivalent tasks in that environment's
scheduler instead. Do not configure both schedules.

The commands use `$HOME` for the selected account. Ensure the scheduler's `PATH`
contains the utility locations reported by the runtime check. Application maps
are loaded from `~/bcfunl/data/` because each command changes directory first.

### Daily collection acquisition

Suggested schedule: **01:00 daily** (`0 1 * * *`).

```sh
cd "$HOME/bcfunl" && ./bcfunl --user stuarthunter --output /media/backup/bcfstage --cookies "$HOME/bcfunl/bandcamp.cookies.txt" --parallel 2 --summary >> logs/download.log 2>&1
```

### Hourly processing and cleanup

Suggested schedule: **20 minutes past every hour** (`20 * * * *`).

```sh
cd "$HOME/bcfunl" && ./bcfunl --process --clean --user stuarthunter --output /media/backup/bcfstage --serverdir /mnt/tankn4/F8NAS1/music --parallel 2 --summary >> logs/process.log 2>&1
```

### Monthly Discogs resource refresh

Suggested schedule: **04:00 on the fifth day of each month** (`0 4 5 * *`).

```sh
cd "$HOME/bcfunl" && ./bcfunl --resources --output /media/backup/bcfstage >> logs/resources.log 2>&1
```

Only current-month snapshots are selected. Missing snapshots produce a failure;
they do not replace the existing library with older data. Processing picks up
the library at its default path when present. Allow space for the compressed
extracts and a new SQLite library alongside the old one during rebuilding.

## Operational behavior

Updated download/process/clean commands share the staging lock. A task that
finds another running exits with an error; the next scheduled invocation retries.
An initial large acquisition can therefore postpone hourly processing until it
finishes. Cleanup runs after the workers finish, including successful releases
from a partially failed processing run. Failed processing still returns failure.

Review `logs/download.log`, `logs/process.log`, and `logs/resources.log`; arrange
log rotation. Expired cookies require a fresh browser export. Keep `~/Downloads`
on persistent storage with enough room for parallel download handoffs.

No runtime compatibility, dataset permissions, TrueNAS tasks or real cleanup have
been verified on the NAS by this session. The installation directory's exact
absolute home path and execution environment are still to be confirmed there.
