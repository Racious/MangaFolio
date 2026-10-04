"""Read-only evidence from the specifically isolated Linux native test workflow.
Run after the documented 28-book fixture, bookmark and next-volume UI operations.
This validates saved artifacts, not Windows/native UI accessibility acceptance.
"""
from pathlib import Path
import json, sqlite3

root = Path('/tmp/mangafolio-third-phase-native')
out = Path(__file__).resolve().parents[1] / 'docs/third-phase-results'
conn = sqlite3.connect('file:' + str(root / 'data/com.racious.mangafolio/library.sqlite3') + '?mode=ro', uri=True)
assert conn.execute('PRAGMA user_version').fetchone()[0] == 3
assert conn.execute('SELECT COUNT(*) FROM books').fetchone()[0] == 28
progress = list(conn.execute('SELECT id,last_index,last_page_name FROM books WHERE id IN (3,4) ORDER BY id'))
assert progress == [(3,119,'120.png'),(4,0,'001.png')]
marks = list(conn.execute('SELECT book_id,page_name,page_index,name,note FROM bookmarks ORDER BY id'))
assert len(marks) == 2 and all(m[:3] == (3,'082.png',81) for m in marks)
settings = conn.execute('SELECT enabled,retention,last_success,last_error FROM backup_settings').fetchone()
assert settings[2] is not None and settings[3] == ''
files = list((root / 'data/com.racious.mangafolio/backups').glob('*.json'))
assert files
for path in files:
    payload = json.loads(path.read_text())
    assert payload['version'] == 3 and len(payload['books']) == 28 and len(payload['bookmarks']) == 2
result = {'exitCode':0,'checks':['isolated schema v3 and 28 books','book 3 last page saved before next volume 4','native bookmark page-name/index saved','native immediate backup succeeded with v3 books and bookmarks'], 'boundary':'Read-only saved artifacts of isolated Linux native UI workflow, not Windows acceptance'}
(out / 'native-artifacts.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n')
print(json.dumps(result,ensure_ascii=False,indent=2))
