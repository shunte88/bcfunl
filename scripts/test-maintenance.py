"""Run with python3 scripts/test-maintenance.py; uses disposable command stubs only."""
import fcntl
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

with tempfile.TemporaryDirectory(prefix="bcfunl-maintenance-test-") as temporary:
    root = Path(temporary)
    (root / "scripts").mkdir()
    shutil.copyfile(Path(__file__).with_name("maintenance.sh"), root / "scripts/maintenance.sh")
    stub = root / "bcfunl"
    stub.write_text('''#!/bin/sh
if [ "$1" = "--process" ]; then
    echo process >> calls
    exit "$TEST_PROCESS_STATUS"
fi
echo acquire >> calls
exit "$TEST_ACQUISITION_STATUS"
''')
    stub.chmod(0o700)
    command = ["sh", str(root / "scripts/maintenance.sh"), "johnsmith", "/unused/staging", "/unused/music"]
    for acquisition, processing, expected in [(0, 0, 0), (7, 0, 7), (0, 9, 9), (7, 9, 7)]:
        (root / "calls").unlink(missing_ok=True)
        environment = dict(os.environ, TEST_ACQUISITION_STATUS=str(acquisition), TEST_PROCESS_STATUS=str(processing))
        result = subprocess.run(command, env=environment, capture_output=True, text=True)
        assert result.returncode == expected, result.stderr
        assert (root / "calls").read_text() == "acquire\nprocess\n"
        assert f"acquisition exit {acquisition}, processing exit {processing}" in result.stdout
    with (root / ".bcfunl-maintenance.lock").open("w") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX | fcntl.LOCK_NB)
        (root / "calls").unlink()
        result = subprocess.run(command, capture_output=True, text=True)
        assert result.returncode != 0 and not (root / "calls").exists()
print("Maintenance checks passed: ordering, failure statuses, and overlap rejection.")
