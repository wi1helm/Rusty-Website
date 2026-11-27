use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Deserialize;
use serde_json::Value;
use uuid::Uuid;

use crate::api::error::api_error;
use crate::auth::CurrentUser;
use crate::storage::database::{NewPosition, NewPositionConnection, NewPositionStatus, PositionRepository, PositionStatus};

const MAX_POSITIONS: usize = 500;
const MAX_CONNECTIONS: usize = 5_000;
const MAX_STATUS_INFORMATION_SIZE: usize = 10_000;

#[derive(Debug, Deserialize)]
pub(super) struct GraphUpdateRequest {
    positions: Vec<PositionInput>,
    connections: Vec<ConnectionInput>,
}

#[derive(Debug, Deserialize)]
struct PositionInput {
    id: Option<Uuid>,
    client_id: String,
    title: String,
    description: String,
    current_status: String,
    statuses: Vec<PositionStatusInput>,
}

#[derive(Debug, Deserialize)]
struct PositionStatusInput {
    status: String,
    information: Value,
}

#[derive(Debug, Deserialize)]
struct ConnectionInput {
    from: String,
    to: String,
}

impl GraphUpdateRequest {
    fn validate(self) -> Result<(Vec<NewPosition>, Vec<NewPositionConnection>), ()> {
        if self.positions.len() > MAX_POSITIONS || self.connections.len() > MAX_CONNECTIONS {
            return Err(());
        }

        let mut client_ids = HashSet::with_capacity(self.positions.len());
        let mut database_ids = HashSet::with_capacity(self.positions.len());
        let mut positions = Vec::with_capacity(self.positions.len());

        for position in self.positions {
            if position.client_id.trim().is_empty()
                || position.client_id.len() > 128
                || !client_ids.insert(position.client_id.clone())
                || position.title.trim().is_empty()
                || position.title.chars().count() > 120
                || position.title.contains('\0')
                || position.description.chars().count() > 5_000
                || position.description.contains('\0')
                || position.statuses.is_empty()
                || position.statuses.len() > 4
                || position.id.is_some_and(|id| !database_ids.insert(id))
            {
                return Err(());
            }

            let current_status = PositionStatus::from_str(&position.current_status).ok_or(())?;
            let mut statuses = Vec::with_capacity(position.statuses.len());
            let mut status_names = HashSet::with_capacity(position.statuses.len());

            for status in position.statuses {
                let status_name = PositionStatus::from_str(&status.status).ok_or(())?;
                if !status_names.insert(status_name) || !status.information.is_object() || serde_json::to_vec(&status.information).map_err(|_| ())?.len() > MAX_STATUS_INFORMATION_SIZE {
                    return Err(());
                }

                statuses.push(NewPositionStatus { status: status_name, information: status.information });
            }

            if !status_names.contains(&current_status) {
                return Err(());
            }

            positions.push(NewPosition { id: position.id, client_id: position.client_id, title: position.title, description: position.description, current_status, statuses });
        }

        let mut indegree = client_ids.iter().map(|client_id| (client_id.clone(), 0usize)).collect::<HashMap<_, _>>();
        let mut children = HashMap::<String, Vec<String>>::new();
        let mut undirected_connections = HashSet::with_capacity(self.connections.len());
        let mut connections = Vec::with_capacity(self.connections.len());

        for connection in self.connections {
            if connection.from == connection.to || !client_ids.contains(&connection.from) || !client_ids.contains(&connection.to) {
                return Err(());
            }

            let pair = if connection.from < connection.to { (connection.from.clone(), connection.to.clone()) } else { (connection.to.clone(), connection.from.clone()) };
            if !undirected_connections.insert(pair) {
                return Err(());
            }

            *indegree.get_mut(&connection.to).ok_or(())? += 1;
            children.entry(connection.from.clone()).or_default().push(connection.to.clone());
            connections.push(NewPositionConnection { from: connection.from, to: connection.to });
        }

        let mut ready = indegree.iter().filter_map(|(client_id, count)| (*count == 0).then_some(client_id.clone())).collect::<VecDeque<_>>();
        let mut visited = 0;
        while let Some(client_id) = ready.pop_front() {
            visited += 1;
            for child in children.get(&client_id).into_iter().flatten() {
                let child_indegree = indegree.get_mut(child).ok_or(())?;
                *child_indegree -= 1;
                if *child_indegree == 0 {
                    ready.push_back(child.clone());
                }
            }
        }

        if visited != client_ids.len() {
            return Err(());
        }

        Ok((positions, connections))
    }
}

pub async fn update(State(positions): State<Arc<dyn PositionRepository>>, CurrentUser(user): CurrentUser, Json(input): Json<GraphUpdateRequest>) -> impl IntoResponse {
    let (positions_input, connections) = match input.validate() {
        Ok(graph) => graph,
        Err(()) => {
            return api_error(StatusCode::BAD_REQUEST, "invalid-graph", "The organization graph contains invalid or unsupported data.");
        }
    };
    //TODO no content reponse isnt tis a ok one? wierd
    match positions.replace_graph_by_organization_id(user.organization.id, positions_input, connections).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => {
            tracing::error!(?error, organization_id = user.organization.id, "failed to update organization graph");
            api_error(StatusCode::INTERNAL_SERVER_ERROR, "server-error", "The organization graph could not be saved. Please try again.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn position(client_id: &str, status: &str) -> PositionInput {
        PositionInput { id: None, client_id: client_id.to_owned(), title: format!("{client_id} title"), description: String::new(), current_status: status.to_owned(), statuses: vec![PositionStatusInput { status: status.to_owned(), information: json!({"information": ""}) }] }
    }

    fn request(positions: Vec<PositionInput>, connections: Vec<ConnectionInput>) -> GraphUpdateRequest {
        GraphUpdateRequest { positions, connections }
    }

    #[test]
    fn accepts_a_valid_graph_with_connections() {
        let result = request(vec![position("manager", "open"), position("report", "FILLED")], vec![ConnectionInput { from: "manager".to_owned(), to: "report".to_owned() }]).validate();

        let (positions, connections) = result.expect("graph should be valid");
        assert_eq!(positions.len(), 2);
        assert_eq!(connections.len(), 1);
    }

    #[test]
    fn rejects_connections_to_unknown_positions() {
        let result = request(vec![position("manager", "open")], vec![ConnectionInput { from: "manager".to_owned(), to: "missing".to_owned() }]);

        assert!(result.validate().is_err());
    }

    #[test]
    fn rejects_cycles_and_duplicate_reverse_connections() {
        let cycle = request(
            vec![position("one", "open"), position("two", "open"), position("three", "open")],
            vec![ConnectionInput { from: "one".to_owned(), to: "two".to_owned() }, ConnectionInput { from: "two".to_owned(), to: "three".to_owned() }, ConnectionInput { from: "three".to_owned(), to: "one".to_owned() }],
        );
        assert!(cycle.validate().is_err());

        let reverse = request(vec![position("one", "open"), position("two", "open")], vec![ConnectionInput { from: "one".to_owned(), to: "two".to_owned() }, ConnectionInput { from: "two".to_owned(), to: "one".to_owned() }]);
        assert!(reverse.validate().is_err());
    }

    #[test]
    fn rejects_graphs_without_the_current_status_record() {
        let mut input = position("manager", "open");
        input.current_status = "closed".to_owned();

        assert!(request(vec![input], Vec::new()).validate().is_err());
    }
}
