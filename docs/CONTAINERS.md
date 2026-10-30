# Run bcfunl in a container

The Compose service is a one-shot command runner, not a web server. Run it on the
NAS itself with datasets bind-mounted from local paths. This removes the NFS
client from the processing path, while retaining audio verification and
no-overwrite safeguards. A container does not fix damaged audio or conflicting
existing music-library copies.

## Configure

Compose uses the prebuilt `ghcr.io/shunte88/bcfunl:latest` image. No local
compilation, Rust installation, or sibling VCW checkout is required on the NAS.
The final Linux image contains aria2c, 7z, ffmpeg, ffprobe, metaflac, gzip,
sha256sum, flock, SQLite and CA certificates. Cookies, music, staging history,
and resource databases are not baked into the image.

```sh
cp --update=none .env.example .env
mkdir -p container-state/logs container-state/Downloads
id -u
id -g
```

Edit `.env` with that account's UID/GID and the actual NAS dataset paths:

```dotenv
BCFUNL_UID=1000
BCFUNL_GID=1000
BCFUNL_STAGING=/mnt/backup
BCFUNL_MUSIC=/mnt/tankn4/music
BCFUNL_COOKIES=./bandcamp.cookies.txt
BCFUNL_IMAGE=ghcr.io/shunte88/bcfunl:latest
```

Both datasets and the cookie file must already exist and be accessible to that
account. Keep cookies mode600; `container-state` must be writable by the configured
UID/GID. The account also needs write access to the music dataset. Do not run the
image as root to work around dataset ACL problems. Compose refuses to create
missing host paths, avoiding an accidental empty replacement for a dataset.

## Move existing history from media2026

Stop all jobs, then copy staging payloads and their consistent SQLite database
together into `/mnt/backup`. Preserve relative paths beneath staging. Back up the
idle database before changing paths. The supplied migration changes only known
path prefixes, keeps completion/error history, and is safe to repeat:

```sh
sqlite3 /mnt/backup/collection.sqlite3 ".backup '/mnt/backup/collection.before-path-migration.sqlite3'"
sqlite3 -bail /mnt/backup/collection.sqlite3 < scripts/migrate-nas-paths.sql
```

The transaction updates `file_path` from `/data/bcfstag3/` to `/mnt/backup/` and
`processed_path` from `/mnt/FN042018/F8NAS1/music/` to `/mnt/tankn4/music/`.
It prints the number of rows changed. Files are not moved by SQL. Inspect any
other stored prefixes before running jobs; those rows are intentionally unchanged.
The image includes the optional sqlite3 command for migration/manual inspection.
Mount sources and targets use the same NAS-local paths; no client NFS mount or
container path aliases are needed.

## Pull and check

```sh
docker compose config --quiet
docker compose pull bcfunl
docker compose run --rm bcfunl --version
```

The default `docker compose up` prints help and exits; it never starts a download
or deletion job implicitly. `docker compose run` passes arguments directly to
bcfunl. Use the printed version to confirm the deployed image is current.

## Use a published image

After a successful GitHub workflow, the image is available as
`ghcr.io/shunte88/bcfunl:sha-<full-commit-sha>`. The Linux build
must pass before image publication begins. The publication job
builds and smoke-tests its image before pushing. Default-branch builds also
update `latest`; other branches never move that tag. Image publication is not
gated on a version change. The initial image targets Linux amd64 for an x64 NAS.

Set `BCFUNL_IMAGE` in `.env` to the published tag, for example:

```dotenv
BCFUNL_IMAGE=ghcr.io/shunte88/bcfunl:latest
```

Pull the image before the next run to pick up a newly published `latest`:

```sh
docker compose pull bcfunl
docker compose run --rm bcfunl --version
```

For a predictable deployment, use a commit tag or the registry's image digest
instead of `latest`. Keep Compose, `.env`, `data/`, and the mounted credentials
and datasets configured as above. Compose has no local build configuration.
If an existing `.env` sets `BCFUNL_IMAGE=bcfunl:local`, remove or update that
setting; environment overrides take precedence over the default. If the registry package is
private, sign in with `docker login ghcr.io` using an account/token with package
read access, or deliberately make the package public in its GitHub settings.
The workflow publishes using its repository-scoped `GITHUB_TOKEN`; no Docker Hub
credentials are required. Image visibility is not changed automatically.

## First processing test

After migration, or for new history:

```sh
docker compose run --rm bcfunl --process --user johnsmith \
  --output /mnt/backup --serverdir /mnt/tankn4/music \
  --artist MONO --count 1 --parallel 2 --summary --timings
```

Start without cleanup and review output before adding `--clean`. Timing lines
identify setup/history checks, local extraction, tag reading/preparation,
existing-copy verification, genre fallback, copy/tag/audio verification, artwork,
manifest syncing and publication. Failed stages also report their elapsed time.
ZIP extraction is local to `--output`; prepared output remains on `--serverdir`
for same-filesystem final publication. Both locations need temporary free space.

## Other jobs

Use NAS-local paths:

```sh
docker compose run --rm bcfunl --user johnsmith \
  --output /mnt/backup --parallel 2 --summary
docker compose run --rm bcfunl --resources --output /mnt/backup
```

For the sequential maintenance wrapper:

```sh
docker compose run --rm --entrypoint sh bcfunl \
  /app/scripts/maintenance.sh johnsmith \
  /mnt/backup /mnt/tankn4/music
```

The wrapper runs acquisition then processing with cleanup. Logs and workflow
locks persist under `container-state`. `HOME=/state` keeps aria2's temporary
Downloads on that same persistent storage. Cookie and mapping mounts are read-only.

Schedule the wrapper command from the NAS task account, starting with `cd` to
the absolute directory containing `compose.yaml`. Keep acquisition/processing
as one workflow. For monthly resources, use the same persistent workflow lock:

```sh
docker compose run --rm --entrypoint flock bcfunl \
  -n /state/.bcfunl-maintenance.lock /app/bcfunl \
  --resources --output /mnt/backup
```

Jobs sharing staging are also protected by bcfunl's staging lock. Standalone
`--clean` covers filtered historical successes. Combined `--process --clean`
cleans only releases attempted in that processing invocation, so a limited test
does not audit or remove hundreds of older payloads.

## Push builds

The GitHub Actions workflow runs on every push and manual dispatch, independent
of version changes. It currently packages a Linux x64 executable and publishes
the Linux image with mapping files and runtime tooling. macOS and Windows jobs
have been removed from the active build matrix.

The workflow checks out `shunte88/vcw` alongside this repository. If that
repository is private, configure a read-only `VCW_READ_TOKEN` Actions secret with
access to it. Optionally set repository variable `VCW_REF` to a specific commit
or branch; the default is `main`. The lockfile remains enforced. Actions artifacts
are named with the platform and commit SHA, so pushes with an unchanged version
still produce distinct packages. Each artifact contains a tar.gz archive to
preserve Unix executable permissions. Tools required by native binaries are installed
separately; the Linux container includes them.

Image execution needs a working Docker daemon and dataset access. A successful
image pull does not by itself verify dataset permissions or processing.
