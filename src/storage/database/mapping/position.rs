use chrono::{DateTime, Utc};
use uuid::Uuid;
use crate::storage::database::data::{DatabasePosition, DatabasePositionStatus, PositionStatus};
use crate::storage::database::DatabaseError;

pub struct PositionRow {
    pub id: Uuid,
    pub organization_id: i64,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,

    pub status_id: Uuid,
    pub status_position_id: Uuid,
    pub status: String,
    pub information: String,
    pub status_created_at: DateTime<Utc>,
}

pub fn map_position(row: PositionRow, ) -> Result<DatabasePosition, DatabaseError> {

    let status = PositionStatus::from_str(&row.status)
        .ok_or_else(|| DatabaseError::InvalidPositionStatus(row.status))?;

    Ok(DatabasePosition {
        id: row.id,
        organization_id: row.organization_id,
        title: row.title,
        description: row.description,
        status: DatabasePositionStatus {
            id: row.status_id,
            position_id: row.status_position_id,
            status,
            information: row.information,
            created_at: row.status_created_at,
        },
        created_at: row.created_at,
        updated_at: row.updated_at,
    })
}