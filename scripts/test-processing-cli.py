"""Exercise native publication with the governor, then standalone cleanup."""
from pathlib import Path
import tempfile,subprocess,sqlite3,os,time,resource,re
binary=str(Path(__file__).resolve().parent.parent/'target/release/bcfunl')
with tempfile.TemporaryDirectory(prefix='bcfunl-governor-smoke-') as temp:
 root=Path(temp);stage=root/'stage';server=root/'server';source=stage/'johnsmith'/'Fixture'/'source.flac'
 source.parent.mkdir(parents=True)
 subprocess.run(['ffmpeg','-v','error','-f','lavfi','-i','sine=frequency=440:duration=60','-metadata','artist=Fixture','-metadata','album=Governor Proof','-metadata','title=One','-metadata','track=1','-metadata','disc=1',str(source)],check=True)
 with sqlite3.connect(stage/'collection.sqlite3') as db:
  db.execute('CREATE TABLE releases(user TEXT NOT NULL,item_key TEXT NOT NULL,artist TEXT NOT NULL,release_title TEXT NOT NULL,publish_date TEXT,purchase_date TEXT,download_date TEXT,download_url TEXT NOT NULL,format TEXT,file_path TEXT,last_error TEXT,PRIMARY KEY(user,item_key))')
  db.execute("INSERT INTO releases(user,item_key,artist,release_title,download_date,download_url,format,file_path) VALUES('johnsmith','album1','Fixture','Governor Proof','2026-11-12T00:00:00Z','https://bandcamp.com/download','flac',?)",(str(source),))
 library=stage/'resources'/'library.sqlite3';library.parent.mkdir()
 import json
 with sqlite3.connect(library) as db:
  db.executescript("CREATE TABLE sanitized_genres(genre TEXT PRIMARY KEY); INSERT INTO sanitized_genres VALUES('Electronic'),('Ambient'); CREATE TABLE artists(id INTEGER,name TEXT); CREATE TABLE artist_genres(artist_id INTEGER,genre TEXT,master_count INTEGER); CREATE TABLE resource_import(dataset TEXT,source_file TEXT,source_sha256 TEXT); CREATE TABLE releases(id INTEGER PRIMARY KEY,title_key TEXT,data TEXT); CREATE INDEX release_title_key ON releases(title_key,id);")
  db.executemany('INSERT INTO resource_import VALUES(?,?,?)',[('masters','masters.xml','fixture'),('releases','releases.xml','fixture')])
  body={'title':'Governor Proof','artists':[{'name':'Fixture'}],'country':'Worldwide','labels':[{'name':'Fixture Label','catno':'LOCAL-1'}],'genres':['Electronic'],'styles':['Ambient']}
  db.execute('INSERT INTO releases VALUES(?,?,?)',(1,'governor proof',json.dumps(body)))
 env=dict(os.environ,PATH='/nonexistent')
 result=subprocess.run([binary,'--resources','--genres-only','--output',str(stage),'--cookies',str(root/'missing.cookies.txt')],env=env,capture_output=True,text=True,timeout=20)
 assert result.returncode==0,result.stderr
 assert 'no downloads or XML imports' in result.stderr
 with sqlite3.connect(library) as db:
  assert db.execute('SELECT count(*) FROM sanitized_genres').fetchone()[0]==2
 before=resource.getrusage(resource.RUSAGE_CHILDREN);started=time.monotonic()
 result=subprocess.run([binary,'--process','--offline','--nice','20','--parallel','8','--user','johnsmith','--output',str(stage),'--serverdir',str(server),'--count','1','--summary'],env=env,capture_output=True,text=True,timeout=60)
 elapsed=time.monotonic()-started;after=resource.getrusage(resource.RUSAGE_CHILDREN)
 cpu=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime
 print(result.stderr);assert result.returncode==0,result.stdout
 cores=float(re.search(r'20% of ([0-9.]+) CPU',result.stderr).group(1))*0.2
 assert cpu<=cores*(elapsed+0.25)+0.2,(cpu,elapsed,cores)
 assert 'Processing budget: 20%' in result.stderr
 assert 'Discogs local [Fixture - Governor Proof]: selected 1 (catalog LOCAL-1)' in result.stderr
 assert list(server.rglob('*LOCAL-1*'))
 assert source.exists()
 with sqlite3.connect(stage/'collection.sqlite3') as db:
  assert db.execute('SELECT process_date FROM releases').fetchone()[0]
 published=list(server.rglob('*.flac'));stamps={p:p.stat().st_mtime_ns for p in published}
 result=subprocess.run([binary,'--collection','--parallel','3','--user','johnsmith','--output',str(stage),'--serverdir',str(server),'--count','1','--summary'],env=env,capture_output=True,text=True,timeout=60)
 assert result.returncode==0,result.stderr
 assert '1 matched' in result.stdout,result.stdout
 assert all(p.stat().st_mtime_ns==stamps[p] for p in published)
 with sqlite3.connect(stage/'collection.sqlite3') as db:
  rows=db.execute('SELECT artist,title FROM collection').fetchall()
  assert len(rows)==1 and rows[0][0]=='Fixture' and rows[0][1].startswith('Governor Proof'),rows
 stop=stage/'.bcfunl-stop';stop.touch()
 for mode in [None,'--database','--cleanup','--resources','--process','--collection']:
  command=[binary]+([mode] if mode else [])+['--user','johnsmith','--output',str(stage),'--cookies',str(root/'missing.cookies.txt')]
  if mode=='--resources':command+=['--resource-file',str(root/'missing-artists.xml')]
  if mode in ('--process','--collection'):command+=['--serverdir',str(server)]
  result=subprocess.run(command,env=env,capture_output=True,text=True,timeout=10)
  assert result.returncode==0,result.stderr
  assert 'Stop file detected' in result.stderr
  assert source.exists() and stop.exists()
 assert not (stage/'backups').exists()
 missing=root/'never-created'
 result=subprocess.run([binary,'--database','--stopfile',str(stop),'--output',str(missing)],env=env,capture_output=True,text=True,timeout=10)
 assert result.returncode==0 and not missing.exists(),result.stderr
 bad=root/'not-a-directory';bad.write_text('fixture')
 result=subprocess.run([binary,'--database','--stopfile',str(bad/'stop'),'--output',str(missing)],env=env,capture_output=True,text=True,timeout=10)
 assert result.returncode==1 and not missing.exists(),result.stderr
 assert 'Cannot inspect stop file' in result.stderr
 stop.unlink()
 result=subprocess.run([binary,'--cleanup','--user','johnsmith','--output',str(stage)],env=env,capture_output=True,text=True,timeout=10)
 assert result.returncode==0,result.stderr
 assert not source.exists()
 assert list(server.rglob('*.flac'))
 print('Genre-only refresh, governor publication, offline collection tags, persistent stop-file startup checks, and standalone cleanup passed with PATH=/nonexistent.')
