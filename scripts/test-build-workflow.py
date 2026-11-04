"""Run with python3 scripts/test-build-workflow.py; disposable command stubs only."""
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

repo = Path(__file__).resolve().parent.parent
revision = (repo / '.github/vcw-revision').read_text().strip()
with tempfile.TemporaryDirectory(prefix='bcfunl-build-gates-') as temporary:
    root = Path(temporary)
    app = root / 'bcfunl'
    for directory in ['scripts', '.github', 'data', 'assets', 'skills', 'docs']:
        (app / directory).mkdir(parents=True)
    shutil.copyfile(repo / 'scripts/check-build.sh', app / 'scripts/check-build.sh')
    (app / '.github/vcw-revision').write_text(revision + '\n')
    for file in ['scripts/maintenance.sh', 'scripts/migrate-nas-paths.sql', 'README.md',
                 'LICENSE', 'docs/REFERENCE.md', 'docs/TRUENAS.md', 'docs/CONTAINERS.md']:
        (app / file).write_text('fixture\n')
    (root / 'vcw').mkdir()
    (root / 'vcw/LICENSE').write_text('fixture license\n')
    tools = root / 'bin'
    tools.mkdir()
    stubs = {
        'git': '''case "$*" in
  *rev-parse*) echo "$TEST_REVISION" ;;
  *diff*) exit "$TEST_DIRTY" ;;
esac
''',
        'cargo': '''echo "cargo $*" >> "$TEST_CALLS"
case "$*" in
  *build*)
    [ "$TEST_BUILD_STATUS" -eq 0 ] || exit "$TEST_BUILD_STATUS"
    mkdir -p target/release
    printf '#!/bin/sh\necho bcfunl-fixture\n' > target/release/bcfunl
    chmod +x target/release/bcfunl ;;
esac
''',
        'docker': '''echo "docker $*" >> "$TEST_CALLS"
if [ "$1" = info ]; then
    [ "$TEST_NEEDS_SUDO" -eq 0 ] || [ "${SUDO_DOCKER:-0}" -eq 1 ]
    exit $?
fi
case "$*" in
  'buildx build '*) exit "$TEST_DOCKER_STATUS" ;;
esac
''',
        'sudo': '''echo "sudo $*" >> "$TEST_CALLS"
[ "$TEST_SUDO_STATUS" -eq 0 ] || exit "$TEST_SUDO_STATUS"
shift
SUDO_DOCKER=1 exec "$@"
''',
    }
    for name, body in stubs.items():
        path = tools / name
        path.write_text('#!/bin/sh\nset -eu\n' + body)
        path.chmod(0o700)
    calls = root / 'calls'
    command = ['sh', str(app / 'scripts/check-build.sh')]
    for wrong_ref, dirty, build_status, docker_status, needs_sudo, sudo_status in [
        (False, 0, 0, 0, 0, 0), (True, 0, 0, 0, 0, 0), (False, 1, 0, 0, 0, 0),
        (False, 0, 101, 0, 0, 0), (False, 0, 0, 125, 0, 0),
        (False, 0, 0, 0, 1, 0), (False, 0, 0, 0, 1, 1),
    ]:
        calls.unlink(missing_ok=True)
        environment = dict(os.environ, PATH=str(tools) + os.pathsep + os.environ['PATH'],
                           TEST_CALLS=str(calls), TEST_REVISION='0' * 40 if wrong_ref else revision,
                           TEST_DIRTY=str(dirty), TEST_BUILD_STATUS=str(build_status),
                           TEST_DOCKER_STATUS=str(docker_status), TEST_NEEDS_SUDO=str(needs_sudo),
                           TEST_SUDO_STATUS=str(sudo_status))
        result = subprocess.run(command, env=environment, capture_output=True, text=True)
        invoked = calls.read_text() if calls.exists() else ''
        if wrong_ref or dirty:
            assert result.returncode != 0 and not invoked, result.stdout + result.stderr
        elif needs_sudo and sudo_status:
            assert result.returncode != 0 and 'sudo -v' in result.stderr and 'cargo ' not in invoked
        elif build_status:
            assert result.returncode == build_status and 'docker buildx build' not in invoked
        elif docker_status:
            assert result.returncode == docker_status and 'checks passed' not in result.stdout
        else:
            assert result.returncode == 0, result.stdout + result.stderr
            if needs_sudo:
                assert 'sudo -n docker buildx build' in invoked
                assert 'sudo -n cargo' not in invoked
            assert 'cargo +stable build --locked --release\n' in invoked
            assert 'cargo +stable test --locked\n' in invoked
            assert 'docker buildx build --load --build-context vcw=../vcw --tag bcfunl:ci .' in invoked
            assert 'docker run --rm bcfunl:ci --version' in invoked
            with tarfile.open(app / 'target/package/bcfunl-linux-x64.tar.gz') as archive:
                assert {'./bcfunl', './VCW-LICENSE', './scripts/maintenance.sh'} <= set(archive.getnames())
print('Build gate checks passed: pinned sources, locked arguments, failure propagation, image commands and packaging.')
