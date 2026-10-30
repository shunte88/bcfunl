-- Run against a backed-up, idle staging database after copying its payloads to the NAS.
BEGIN IMMEDIATE;
UPDATE releases
SET file_path = '/mnt/backup' || substr(file_path, length('/data/bcfstag3') + 1)
WHERE substr(file_path, 1, length('/data/bcfstag3/')) = '/data/bcfstag3/';
SELECT 'payload paths updated', changes();
UPDATE releases
SET processed_path = '/mnt/tankn4/music' || substr(processed_path, length('/mnt/FN042018/F8NAS1/music') + 1)
WHERE substr(processed_path, 1, length('/mnt/FN042018/F8NAS1/music/')) = '/mnt/FN042018/F8NAS1/music/';
SELECT 'published paths updated', changes();
COMMIT;
