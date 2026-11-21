use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;
use crate::storage::database::data::{DatabasePosition, DatabasePositionStatus, PositionStatus};
use crate::storage::database::DatabaseError;

pub struct PositionRow {
    // Position
    pub id: Uuid,
    pub organization_id: i64,
    pub title: String,
    pub description: String,
    pub current_status_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    // Status
    pub status_id: Uuid,
    pub status_position_id: Uuid,
    pub status: String,
    pub information: Value,
    pub status_created_at: DateTime<Utc>,
}
pub fn map_positions(rows: Vec<PositionRow>) -> Result<Vec<DatabasePosition>, DatabaseError> {

    let mut positions: Vec<DatabasePosition> = Vec::new();

    for row in rows {
        let status = PositionStatus::from_str(&row.status)
            .ok_or_else(|| {
                DatabaseError::InvalidPositionStatus(row.status.clone())
            })?;

        let status = DatabasePositionStatus {
            id: row.status_id,
            position_id: row.status_position_id,
            status,
            information: row.information,
            created_at: row.status_created_at,
        };

        // Check whether we have already created this position.
        // TODO: This only checks last_mut(), so it relies on rows for the same
        // position being consecutive in the input. It works because the SQL query
        // orders by created_at, but this invariant is fragile — if the query ever
        // changes, the mapping will silently produce incorrect results.
        //
        // Fix: Use a HashMap<Uuid, DatabasePosition> to group statuses by position
        // ID, then collect into a Vec. This is O(n) instead of O(n²) and doesn't
        // depend on row ordering. Example approach:
        //   let mut map = HashMap::<Uuid, DatabasePosition>::new();
        //   for row in rows { ... map.entry(row.id).or_insert(...).statuses.push(status); }
        //   let positions = map.into_values().collect();
        //
        // Search terms: "HashMap grouping Rust", "Iterator::fold grouping",
        // "itertools group_by", "avoid ordering dependency Rust".
        if let Some(position) = positions.last_mut()
            .filter(|position| position.id == row.id)
        {
            // Add this status to the existing position.
            position.statuses.push(status);
        } else {
            // Create a new position.
            positions.push(DatabasePosition {
                id: row.id,
                organization_id: row.organization_id,
                title: row.title,
                description: row.description,
                current_status_id: row.current_status_id,
                statuses: vec![status],
                created_at: row.created_at,
                updated_at: row.updated_at,
            });
        }
    }

    Ok(positions)
}