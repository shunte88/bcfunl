"""Run with python3 scripts/test-migration.py; no real database is opened."""
from pathlib import Path
import sqlite3

connection = sqlite3.connect(":memory:")
connection.execute("CREATE TABLE releases(file_path TEXT, processed_path TEXT, download_date TEXT, process_date TEXT, attempts INTEGER, last_error TEXT)")
rows = [
    ("/data/bcfstag3/johnsmith/Artist/one.flac", "/mnt/FN042018/F8NAS1/music/Artist/Album", "done", "done", 3, None),
    ("/data/bcfstag30/keep.flac", "/mnt/FN042018/F8NAS1/music-other/keep", "done", None, 2, "error"),
    ("/other/data/bcfstag3/keep.flac", None, None, None, 1, "error"),
    (None, None, None, None, 0, None),
]
connection.executemany("INSERT INTO releases VALUES(?,?,?,?,?,?)", rows)
connection.commit()
sql = Path(__file__).with_name("migrate-nas-paths.sql").read_text()
connection.executescript(sql)
updated = connection.execute("SELECT * FROM releases").fetchall()
assert updated[0][:2] == ("/mnt/backup/johnsmith/Artist/one.flac", "/mnt/tankn4/music/Artist/Album")
assert updated[0][2:] == rows[0][2:]
assert updated[1:] == rows[1:]
connection.executescript(sql)
assert connection.execute("SELECT * FROM releases").fetchall() == updated
print("Path migration checks passed: prefix boundaries, NULLs, history preservation, and repeat runs.")
