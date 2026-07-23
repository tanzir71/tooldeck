use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::db::Database;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: i64,
    pub client_name: String,
    pub name: String,
    pub color: String,
    pub billable: bool,
    pub rate: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeEntry {
    pub id: i64,
    pub project_id: Option<i64>,
    pub project_name: Option<String>,
    pub project_color: Option<String>,
    pub note_path: Option<String>,
    pub description: String,
    pub started_at: i64,
    pub stopped_at: Option<i64>,
    pub source: String,
    pub billable: bool,
    pub rate_snapshot: f64,
    pub duration_seconds: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimerReport {
    pub total_seconds: i64,
    pub billable_seconds: i64,
    pub amount: f64,
    pub entries: Vec<TimeEntry>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IdleDecision {
    Keep,
    Discard,
    Reassign,
}

pub struct TimeService;

impl TimeService {
    pub fn projects(database: &Database) -> Result<Vec<Project>, String> {
        database.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT p.id, c.name, p.name, p.color, p.billable, p.rate
                     FROM projects p LEFT JOIN clients c ON c.id = p.client_id
                     WHERE p.archived = 0 ORDER BY c.name, p.name",
                )
                .map_err(|error| error.to_string())?;
            let rows = statement
                .query_map([], |row| {
                    Ok(Project {
                        id: row.get(0)?,
                        client_name: row.get::<_, Option<String>>(1)?.unwrap_or_default(),
                        name: row.get(2)?,
                        color: row.get(3)?,
                        billable: row.get::<_, i64>(4)? != 0,
                        rate: row.get(5)?,
                    })
                })
                .map_err(|error| error.to_string())?;
            rows.map(|row| row.map_err(|error| error.to_string()))
                .collect()
        })
    }

    pub fn create_project(
        database: &Database,
        name: &str,
        color: &str,
        billable: bool,
        rate: f64,
    ) -> Result<Project, String> {
        if name.trim().is_empty() {
            return Err("project name is required".into());
        }
        if rate < 0.0 || !rate.is_finite() {
            return Err("project rate must be zero or greater".into());
        }
        let now = now_seconds();
        database.with_connection(|connection| {
            let client_id: i64 = connection
                .query_row(
                    "SELECT id FROM clients WHERE name = 'Personal' LIMIT 1",
                    [],
                    |row| row.get(0),
                )
                .map_err(|error| error.to_string())?;
            connection
                .execute(
                    "INSERT INTO projects(
                        client_id, name, color, billable, rate, created_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![client_id, name.trim(), color, billable as i64, rate, now],
                )
                .map_err(|error| error.to_string())?;
            let id = connection.last_insert_rowid();
            Ok(Project {
                id,
                client_name: "Personal".into(),
                name: name.trim().into(),
                color: color.into(),
                billable,
                rate,
            })
        })
    }

    pub fn status(database: &Database) -> Result<Option<TimeEntry>, String> {
        database.with_connection(|connection| {
            query_entries(
                connection,
                "WHERE e.stopped_at IS NULL ORDER BY e.started_at DESC LIMIT 1",
                now_seconds(),
            )
            .map(|entries| entries.into_iter().next())
        })
    }

    pub fn start(
        database: &Database,
        project_id: Option<i64>,
        note_path: Option<&str>,
        description: &str,
        billable: bool,
    ) -> Result<TimeEntry, String> {
        Self::start_at(
            database,
            project_id,
            note_path,
            description,
            billable,
            now_seconds(),
        )
    }

    fn start_at(
        database: &Database,
        project_id: Option<i64>,
        note_path: Option<&str>,
        description: &str,
        billable: bool,
        now: i64,
    ) -> Result<TimeEntry, String> {
        let description = if description.trim().is_empty() {
            "Focus block"
        } else {
            description.trim()
        };
        database.with_connection(|connection| {
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "UPDATE time_entries
                     SET stopped_at = CASE
                        WHEN started_at >= ?1 THEN started_at + 1
                        ELSE ?1 END,
                         updated_at = ?1
                     WHERE stopped_at IS NULL",
                    [now],
                )
                .map_err(|error| error.to_string())?;
            let rate = if let Some(project_id) = project_id {
                transaction
                    .query_row(
                        "SELECT rate FROM projects WHERE id = ?1 AND archived = 0",
                        [project_id],
                        |row| row.get::<_, f64>(0),
                    )
                    .optional()
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| "project not found".to_string())?
            } else {
                0.0
            };
            transaction
                .execute(
                    "INSERT INTO time_entries(
                        project_id, note_path, description, started_at, stopped_at,
                        source, billable, rate_snapshot, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, NULL, 'timer', ?5, ?6, ?4, ?4)",
                    params![
                        project_id,
                        note_path,
                        description,
                        now,
                        billable as i64,
                        rate
                    ],
                )
                .map_err(|error| error.to_string())?;
            let id = transaction.last_insert_rowid();
            transaction.commit().map_err(|error| error.to_string())?;
            entry_by_id(connection, id, now)
        })
    }

    pub fn stop(database: &Database) -> Result<TimeEntry, String> {
        Self::stop_at(database, now_seconds())
    }

    fn stop_at(database: &Database, now: i64) -> Result<TimeEntry, String> {
        database.with_connection(|connection| {
            let running: Option<(i64, i64)> = connection
                .query_row(
                    "SELECT id, started_at FROM time_entries
                     WHERE stopped_at IS NULL ORDER BY started_at DESC LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .optional()
                .map_err(|error| error.to_string())?;
            let (id, started_at) = running.ok_or_else(|| "no timer is running".to_string())?;
            if now <= started_at {
                return Err("a timer must run for at least one second".into());
            }
            connection
                .execute(
                    "UPDATE time_entries SET stopped_at = ?1, updated_at = ?1 WHERE id = ?2",
                    params![now, id],
                )
                .map_err(|error| error.to_string())?;
            entry_by_id(connection, id, now)
        })
    }

    pub fn add_manual(
        database: &Database,
        project_id: Option<i64>,
        description: &str,
        started_at: i64,
        stopped_at: i64,
        billable: bool,
    ) -> Result<TimeEntry, String> {
        if stopped_at <= started_at {
            return Err("the end time must be after the start time".into());
        }
        database.with_connection(|connection| {
            let rate = project_id
                .map(|id| {
                    connection
                        .query_row("SELECT rate FROM projects WHERE id = ?1", [id], |row| {
                            row.get::<_, f64>(0)
                        })
                        .map_err(|error| error.to_string())
                })
                .transpose()?
                .unwrap_or(0.0);
            connection
                .execute(
                    "INSERT INTO time_entries(
                        project_id, description, started_at, stopped_at, source,
                        billable, rate_snapshot, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, 'manual', ?5, ?6, ?7, ?7)",
                    params![
                        project_id,
                        description.trim(),
                        started_at,
                        stopped_at,
                        billable as i64,
                        rate,
                        now_seconds()
                    ],
                )
                .map_err(|error| error.to_string())?;
            entry_by_id(connection, connection.last_insert_rowid(), now_seconds())
        })
    }

    pub fn edit_entry(
        database: &Database,
        id: i64,
        project_id: Option<i64>,
        description: &str,
        started_at: i64,
        stopped_at: i64,
        billable: bool,
    ) -> Result<TimeEntry, String> {
        if stopped_at <= started_at {
            return Err("the end time must be after the start time".into());
        }
        database.with_connection(|connection| {
            let rate = project_id
                .map(|project_id| {
                    connection
                        .query_row(
                            "SELECT rate FROM projects WHERE id = ?1",
                            [project_id],
                            |row| row.get::<_, f64>(0),
                        )
                        .map_err(|error| error.to_string())
                })
                .transpose()?
                .unwrap_or(0.0);
            let changed = connection
                .execute(
                    "UPDATE time_entries SET
                        project_id = ?1, description = ?2, started_at = ?3,
                        stopped_at = ?4, billable = ?5, rate_snapshot = ?6,
                        updated_at = ?7
                     WHERE id = ?8 AND stopped_at IS NOT NULL",
                    params![
                        project_id,
                        description.trim(),
                        started_at,
                        stopped_at,
                        billable as i64,
                        rate,
                        now_seconds(),
                        id
                    ],
                )
                .map_err(|error| error.to_string())?;
            if changed == 0 {
                return Err("completed time entry not found".into());
            }
            entry_by_id(connection, id, now_seconds())
        })
    }

    pub fn split_entry(
        database: &Database,
        id: i64,
        split_at: i64,
    ) -> Result<Vec<TimeEntry>, String> {
        database.with_connection(|connection| {
            let entry = entry_by_id(connection, id, now_seconds())?;
            let stopped_at = entry
                .stopped_at
                .ok_or_else(|| "stop a timer before splitting it".to_string())?;
            if split_at <= entry.started_at || split_at >= stopped_at {
                return Err("split time must fall inside the entry".into());
            }
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "UPDATE time_entries SET stopped_at = ?1, updated_at = ?2 WHERE id = ?3",
                    params![split_at, now_seconds(), id],
                )
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "INSERT INTO time_entries(
                        project_id, note_path, description, started_at, stopped_at,
                        source, billable, rate_snapshot, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?9)",
                    params![
                        entry.project_id,
                        entry.note_path,
                        entry.description,
                        split_at,
                        stopped_at,
                        entry.source,
                        entry.billable as i64,
                        entry.rate_snapshot,
                        now_seconds()
                    ],
                )
                .map_err(|error| error.to_string())?;
            let second_id = transaction.last_insert_rowid();
            transaction.commit().map_err(|error| error.to_string())?;
            Ok(vec![
                entry_by_id(connection, id, now_seconds())?,
                entry_by_id(connection, second_id, now_seconds())?,
            ])
        })
    }

    pub fn merge_entries(
        database: &Database,
        first_id: i64,
        second_id: i64,
    ) -> Result<TimeEntry, String> {
        if first_id == second_id {
            return Err("choose two different entries".into());
        }
        database.with_connection(|connection| {
            let first = entry_by_id(connection, first_id, now_seconds())?;
            let second = entry_by_id(connection, second_id, now_seconds())?;
            let first_stop = first
                .stopped_at
                .ok_or_else(|| "stop both timers before merging".to_string())?;
            let second_stop = second
                .stopped_at
                .ok_or_else(|| "stop both timers before merging".to_string())?;
            if first.project_id != second.project_id || first.billable != second.billable {
                return Err("entries must share a project and billing state".into());
            }
            let started_at = first.started_at.min(second.started_at);
            let stopped_at = first_stop.max(second_stop);
            let description = if first.description == second.description {
                first.description
            } else {
                format!("{} / {}", first.description, second.description)
            };
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            transaction
                .execute(
                    "UPDATE time_entries SET description = ?1, started_at = ?2,
                     stopped_at = ?3, updated_at = ?4 WHERE id = ?5",
                    params![description, started_at, stopped_at, now_seconds(), first_id],
                )
                .map_err(|error| error.to_string())?;
            transaction
                .execute("DELETE FROM time_entries WHERE id = ?1", [second_id])
                .map_err(|error| error.to_string())?;
            transaction.commit().map_err(|error| error.to_string())?;
            entry_by_id(connection, first_id, now_seconds())
        })
    }

    pub fn report(
        database: &Database,
        day_start: i64,
        day_end: i64,
    ) -> Result<TimerReport, String> {
        if day_end <= day_start {
            return Err("report end must be after its start".into());
        }
        let now = now_seconds();
        database.with_connection(|connection| {
            let entries = query_entries(
                connection,
                &format!(
                    "WHERE e.started_at < {day_end}
                     AND COALESCE(e.stopped_at, {now}) > {day_start}
                     ORDER BY e.started_at DESC"
                ),
                now,
            )?;
            let total_seconds = entries.iter().map(|entry| entry.duration_seconds).sum();
            let billable_seconds = entries
                .iter()
                .filter(|entry| entry.billable)
                .map(|entry| entry.duration_seconds)
                .sum();
            let amount = entries
                .iter()
                .filter(|entry| entry.billable)
                .map(|entry| entry.duration_seconds as f64 / 3600.0 * entry.rate_snapshot)
                .sum();
            Ok(TimerReport {
                total_seconds,
                billable_seconds,
                amount,
                entries,
            })
        })
    }

    pub fn reconcile_idle(
        database: &Database,
        idle_seconds: i64,
        decision: IdleDecision,
        target_project_id: Option<i64>,
    ) -> Result<Option<TimeEntry>, String> {
        Self::reconcile_idle_at(
            database,
            idle_seconds,
            decision,
            target_project_id,
            now_seconds(),
        )
    }

    fn reconcile_idle_at(
        database: &Database,
        idle_seconds: i64,
        decision: IdleDecision,
        target_project_id: Option<i64>,
        now: i64,
    ) -> Result<Option<TimeEntry>, String> {
        if idle_seconds <= 0 {
            return Err("idle duration must be positive".into());
        }
        if matches!(decision, IdleDecision::Keep) {
            return Self::status(database);
        }
        database.with_connection(|connection| {
            let running = query_entries(
                connection,
                "WHERE e.stopped_at IS NULL ORDER BY e.started_at DESC LIMIT 1",
                now,
            )?
            .into_iter()
            .next()
            .ok_or_else(|| "no timer is running".to_string())?;
            let idle_start = (now - idle_seconds).max(running.started_at);
            let transaction = connection
                .unchecked_transaction()
                .map_err(|error| error.to_string())?;
            if idle_start > running.started_at {
                transaction
                    .execute(
                        "UPDATE time_entries
                         SET stopped_at = ?1, updated_at = ?2 WHERE id = ?3",
                        params![idle_start, now, running.id],
                    )
                    .map_err(|error| error.to_string())?;
            } else {
                transaction
                    .execute("DELETE FROM time_entries WHERE id = ?1", [running.id])
                    .map_err(|error| error.to_string())?;
            }
            if matches!(decision, IdleDecision::Reassign) && now > idle_start {
                transaction
                    .execute(
                        "INSERT INTO time_entries(
                            project_id, description, started_at, stopped_at, source,
                            billable, rate_snapshot, created_at, updated_at
                         ) VALUES (?1, 'Idle time', ?2, ?3, 'idle', 0, 0, ?3, ?3)",
                        params![target_project_id, idle_start, now],
                    )
                    .map_err(|error| error.to_string())?;
            }
            transaction
                .execute(
                    "INSERT INTO time_entries(
                        project_id, note_path, description, started_at, stopped_at,
                        source, billable, rate_snapshot, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, NULL, 'timer', ?5, ?6, ?4, ?4)",
                    params![
                        running.project_id,
                        running.note_path,
                        running.description,
                        now,
                        running.billable as i64,
                        running.rate_snapshot
                    ],
                )
                .map_err(|error| error.to_string())?;
            let continuation_id = transaction.last_insert_rowid();
            transaction.commit().map_err(|error| error.to_string())?;
            entry_by_id(connection, continuation_id, now).map(Some)
        })
    }
}

fn entry_by_id(connection: &rusqlite::Connection, id: i64, now: i64) -> Result<TimeEntry, String> {
    query_entries(connection, &format!("WHERE e.id = {id}"), now)?
        .into_iter()
        .next()
        .ok_or_else(|| "time entry not found".to_string())
}

fn query_entries(
    connection: &rusqlite::Connection,
    filter: &str,
    now: i64,
) -> Result<Vec<TimeEntry>, String> {
    let sql = format!(
        "SELECT e.id, e.project_id, p.name, p.color, e.note_path,
                e.description, e.started_at, e.stopped_at, e.source,
                e.billable, e.rate_snapshot,
                COALESCE(e.stopped_at, {now}) - e.started_at
         FROM time_entries e
         LEFT JOIN projects p ON p.id = e.project_id
         {filter}"
    );
    let mut statement = connection
        .prepare(&sql)
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok(TimeEntry {
                id: row.get(0)?,
                project_id: row.get(1)?,
                project_name: row.get(2)?,
                project_color: row.get(3)?,
                note_path: row.get(4)?,
                description: row.get(5)?,
                started_at: row.get(6)?,
                stopped_at: row.get(7)?,
                source: row.get(8)?,
                billable: row.get::<_, i64>(9)? != 0,
                rate_snapshot: row.get(10)?,
                duration_seconds: row.get(11)?,
            })
        })
        .map_err(|error| error.to_string())?;
    rows.map(|row| row.map_err(|error| error.to_string()))
        .collect()
}

fn now_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starting_a_timer_stops_the_previous_one() {
        let database = Database::open_memory().expect("database");
        let first = TimeService::start_at(&database, Some(1), None, "First", false, 100)
            .expect("first timer");
        let second = TimeService::start_at(&database, Some(1), None, "Second", false, 160)
            .expect("second timer");
        let report = TimeService::report(&database, 0, 1_000).expect("report");
        let stopped_first = report
            .entries
            .iter()
            .find(|entry| entry.id == first.id)
            .expect("first entry");
        assert_eq!(stopped_first.duration_seconds, 60);
        assert_eq!(
            TimeService::status(&database)
                .expect("status")
                .expect("running")
                .id,
            second.id
        );
    }

    #[test]
    fn stopped_timer_uses_persisted_timestamps() {
        let database = Database::open_memory().expect("database");
        TimeService::start_at(&database, Some(1), None, "Focus", true, 1_000).expect("timer");
        let stopped = TimeService::stop_at(&database, 4_600).expect("stopped");
        assert_eq!(stopped.duration_seconds, 3_600);
        assert_eq!(
            TimeService::report(&database, 0, 5_000)
                .expect("report")
                .total_seconds,
            3_600
        );
    }

    #[test]
    fn invalid_manual_duration_is_rejected() {
        let database = Database::open_memory().expect("database");
        assert!(TimeService::add_manual(&database, None, "Bad", 100, 99, false).is_err());
    }

    #[test]
    fn idle_discard_creates_a_gap_and_continues_running() {
        let database = Database::open_memory().expect("database");
        TimeService::start_at(&database, Some(1), None, "Focus", false, 100).expect("timer");
        let continuation =
            TimeService::reconcile_idle_at(&database, 300, IdleDecision::Discard, None, 1_000)
                .expect("reconciled")
                .expect("continuation");
        assert_eq!(continuation.started_at, 1_000);
        let logged: i64 = database
            .with_connection(|connection| {
                connection
                    .query_row(
                        "SELECT stopped_at - started_at FROM time_entries
                         WHERE stopped_at IS NOT NULL AND source = 'timer'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(|error| error.to_string())
            })
            .expect("logged duration");
        assert_eq!(logged, 600);
    }

    #[test]
    fn completed_entry_can_split_and_merge() {
        let database = Database::open_memory().expect("database");
        let entry = TimeService::add_manual(&database, Some(1), "Review", 100, 400, false)
            .expect("manual entry");
        let parts = TimeService::split_entry(&database, entry.id, 250).expect("split");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0].duration_seconds, 150);
        let merged =
            TimeService::merge_entries(&database, parts[0].id, parts[1].id).expect("merge");
        assert_eq!(merged.duration_seconds, 300);
    }

    #[test]
    fn running_timer_survives_database_reopen() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("tooldeck.sqlite3");
        {
            let database = Database::open(&path).expect("database");
            TimeService::start_at(&database, Some(1), None, "Persistent", false, 100)
                .expect("timer");
        }
        let reopened = Database::open(&path).expect("reopened database");
        let recovered = TimeService::status(&reopened)
            .expect("status")
            .expect("running timer");
        assert_eq!(recovered.description, "Persistent");
        assert_eq!(recovered.started_at, 100);
    }
}
