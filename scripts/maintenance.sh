#!/bin/sh
# Keep the full acquisition/processing workflow serialized across scheduled runs.
set -u
if [ "$#" -ne 3 ]; then
    echo "Usage: $0 COLLECTION_OWNER STAGING_DIRECTORY MUSIC_DIRECTORY" >&2
    exit 2
fi
cd "$(dirname "$0")/.." || exit 1
mkdir -p logs || exit 1
exec 9>.bcfunl-maintenance.lock
flock -n 9 || { echo 'Another maintenance/resource job is running' >&2; exit 1; }
trap 'exit 130' INT
trap 'exit 143' TERM
acquisition_status=0
./bcfunl --user "$1" --output "$2" --cookies "$PWD/bandcamp.cookies.txt" \
    --parallel 2 --summary >> logs/download.log 2>&1 || acquisition_status=$?
processing_status=0
./bcfunl --process --clean --user "$1" --output "$2" --serverdir "$3" \
    --parallel 2 --summary >> logs/process.log 2>&1 || processing_status=$?
printf 'Maintenance: acquisition exit %s, processing exit %s\n' \
    "$acquisition_status" "$processing_status"
if [ "$acquisition_status" -ne 0 ]; then
    exit "$acquisition_status"
fi
exit "$processing_status"
