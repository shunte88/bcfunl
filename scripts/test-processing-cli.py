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
 env=dict(os.environ,PATH='/nonexistent')
 before=resource.getrusage(resource.RUSAGE_CHILDREN);started=time.monotonic()
 result=subprocess.run([binary,'--process','--offline','--nice','20','--parallel','8','--user','johnsmith','--output',str(stage),'--serverdir',str(server),'--count','1','--summary'],env=env,capture_output=True,text=True,timeout=60)
 elapsed=time.monotonic()-started;after=resource.getrusage(resource.RUSAGE_CHILDREN)
 cpu=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime
 print(result.stderr);assert result.returncode==0,result.stdout
 cores=float(re.search(r'20% of ([0-9.]+) CPU',result.stderr).group(1))*0.2
 assert cpu<=cores*(elapsed+0.25)+0.2,(cpu,elapsed,cores)
 assert 'Processing budget: 20%' in result.stderr
 assert source.exists()
 with sqlite3.connect(stage/'collection.sqlite3') as db:
  assert db.execute('SELECT process_date FROM releases').fetchone()[0]
 result=subprocess.run([binary,'--cleanup','--user','johnsmith','--output',str(stage)],env=env,capture_output=True,text=True,timeout=10)
 assert result.returncode==0,result.stderr
 assert not source.exists()
 assert list(server.rglob('*.flac'))
 print('Governor-enabled native publication and standalone --cleanup passed with PATH=/nonexistent.')
