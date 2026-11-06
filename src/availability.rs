//! Retain preorder observations and revisit deferred purchases within the requested window.
use crate::{Args, Release, db::Database};
use anyhow::Result;
use chrono::{NaiveDate, Utc};

#[derive(Debug)]
pub(crate) struct Deferred;
impl std::fmt::Display for Deferred {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Preorder/future release has no supported audio download yet")
    }
}
impl std::error::Error for Deferred {}

pub(crate) fn record(
    db: &Database,
    args: &Args,
    key: &str,
    preorder: bool,
    dates: &[NaiveDate],
    status: Option<&str>,
) -> Result<()> {
    db.query("UPDATE releases SET is_preorder=MAX(is_preorder,CAST(? AS INTEGER)),scheduled_release_date=COALESCE(NULLIF(?,''),scheduled_release_date),availability_status=COALESCE(NULLIF(?,''),availability_status,'captured'),availability_checked_at=CASE WHEN ?='' THEN availability_checked_at ELSE ? END WHERE user=? AND item_key=?", &[if preorder {"1"} else {"0"},&dates.iter().max().map(ToString::to_string).unwrap_or_default(),status.unwrap_or_default(),status.unwrap_or_default(),&Utc::now().to_rfc3339(),&args.user,key])?;
    Ok(())
}

pub(crate) fn window(sql: &mut String, values: &mut Vec<String>, args: &Args) {
    if args.fromdt.is_some() || args.todt.is_some() {
        sql.push_str(" AND (");
        for (i, column) in ["purchase_date", "scheduled_release_date"]
            .iter()
            .enumerate()
        {
            if i > 0 {
                sql.push_str(" OR ((is_preorder=1 OR availability_status IN ('pending','early-access')) AND ");
            }
            sql.push('(');
            if let Some(from) = args.fromdt {
                sql.push_str(&format!("{column}>=?"));
                values.push(from.to_string());
            }
            if args.fromdt.is_some() && args.todt.is_some() {
                sql.push_str(" AND ");
            }
            if let Some(to) = args.todt {
                sql.push_str(&format!("{column}<=?"));
                values.push(to.to_string());
            }
            sql.push(')');
            if i > 0 {
                sql.push(')');
            }
        }
        sql.push(')');
    }
}

pub(crate) fn retries(db: &Database, args: &Args) -> Result<Vec<Release>> {
    let mut sql = String::from(
        "SELECT item_key,artist,COALESCE(sanitized_title,release_title),download_url,COALESCE(publish_date,scheduled_release_date),release_url,release_private FROM releases WHERE user=? AND parked=0 AND download_date IS NULL AND (attempts>0 OR availability_status IN ('pending','early-access'))",
    );
    let mut values = vec![args.user.clone()];
    if let Some(artist) = &args.artist {
        sql.push_str(" AND UPPER(artist) LIKE '%' || ? || '%'");
        values.push(crate::sanitizing::artist_display(artist).to_uppercase());
    }
    window(&mut sql, &mut values, args);
    sql.push_str(" ORDER BY availability_checked_at,purchase_date DESC,item_key");
    Ok(db
        .query(&sql, &values.iter().map(String::as_str).collect::<Vec<_>>())?
        .into_iter()
        .map(|row| {
            let get = |i: usize| row[i].clone().unwrap_or_default();
            Release {
                key: get(0),
                artist: get(1),
                title: get(2),
                url: get(3),
                publish: row[4]
                    .as_deref()
                    .and_then(|v| NaiveDate::parse_from_str(v, "%Y-%m-%d").ok()),
                release_url: row[5].clone(),
                release_custom_host: None,
                private: get(6) == "1",
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    #[test]
    fn deferred_queue_combines_independent_windows_artist_and_count() {
        let db = Database::open(":memory:").unwrap();
        let mut args = Args::try_parse_from(["bcfunl", "--user", "johnsmith"]).unwrap();
        let dates = [
            ("2026-01-01", "2026-11-05"),
            ("2026-11-03", "2026-12-01"),
            ("2026-01-01", "2026-10-01"),
            ("2026-01-01", "2026-12-01"),
        ];
        for (i, (purchase, scheduled)) in dates.iter().enumerate() {
            db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,purchase_date,scheduled_release_date,is_preorder,availability_status) VALUES('johnsmith',?,'Artist','Title','https://bandcamp.com/download',?,?,1,'pending')", &[&format!("album{i}"),purchase,scheduled]).unwrap();
        }
        for from in [None, Some("2026-11-01")] {
            for to in [None, Some("2026-11-07")] {
                args.fromdt = from.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap());
                args.todt = to.map(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap());
                let result = retries(&db, &args).unwrap();
                for (i, (purchase, scheduled)) in dates.iter().enumerate() {
                    let expected = args.matches(Some(
                        NaiveDate::parse_from_str(purchase, "%Y-%m-%d").unwrap(),
                    )) || args.matches(Some(
                        NaiveDate::parse_from_str(scheduled, "%Y-%m-%d").unwrap(),
                    ));
                    assert_eq!(
                        result.iter().any(|r| r.key == format!("album{i}")),
                        expected
                    );
                }
            }
        }
        args.fromdt = None;
        args.todt = None;
        args.artist = Some("rTiS".into());
        args.count = Some(1);
        assert_eq!(retries(&db, &args).unwrap().len(), 4);
        args.artist = Some("other".into());
        assert!(retries(&db, &args).unwrap().is_empty());
        args.artist = None;
        args.count = None;
        db.query("UPDATE releases SET parked=1 WHERE item_key='album0'", &[])
            .unwrap();
        db.query(
            "UPDATE releases SET download_date='2026-11-01T00:00:00Z' WHERE item_key='album1'",
            &[],
        )
        .unwrap();
        assert_eq!(retries(&db, &args).unwrap().len(), 2);
    }

    #[test]
    fn observation_preserves_dates_flags_and_deferral_is_not_a_failure() {
        let db = Database::open(":memory:").unwrap();
        let args = Args::try_parse_from(["bcfunl", "--user", "johnsmith"]).unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url) VALUES('johnsmith','album1','Artist','Title','https://bandcamp.com/download')",&[]).unwrap();
        let date = Utc::now().date_naive().succ_opt().unwrap();
        record(&db, &args, "album1", true, &[date], Some("pending")).unwrap();
        record(&db, &args, "album1", false, &[], None).unwrap();
        let release = retries(&db, &args).unwrap().remove(0);
        assert_eq!(release.publish, Some(date));
        assert_eq!(
            crate::record_result(&db, &args, &release, Err(Deferred.into())).unwrap(),
            0
        );
        let row=db.query("SELECT is_preorder,scheduled_release_date,availability_status,last_error,download_date FROM releases",&[]).unwrap().remove(0);
        assert_eq!(
            row,
            vec![
                Some("1".into()),
                Some(date.to_string()),
                Some("pending".into()),
                None,
                None
            ]
        );
    }
}
