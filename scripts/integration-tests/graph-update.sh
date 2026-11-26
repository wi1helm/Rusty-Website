#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

signup_account graph-update

http_request "${TEST_ID}-unauthenticated-graph-update" \
  --request POST \
  --header "Origin: $BASE_URL" \
  --header 'Content-Type: application/json' \
  --data '{"positions":[],"connections":[]}' \
  "$BASE_URL/api/profile/update-graph"
assert_status 401 "$HTTP_STATUS" "Unauthenticated graph update"
assert_body_contains "$RESPONSE_BODY" '"code":"authentication-required"'

INITIAL_PAYLOAD="$(python3 -c '
import json,sys
test_id=sys.argv[1]
print(json.dumps({
    "positions": [
        {
            "client_id": "director",
            "title": f"Director {test_id}",
            "description": "Leads the organization",
            "current_status": "open",
            "statuses": [
                {"status": "open", "information": {"team": "leadership"}},
                {"status": "filled", "information": {"since": 2020}}
            ]
        },
        {
            "client_id": "engineer",
            "title": f"Engineer {test_id}",
            "description": "Builds the product",
            "current_status": "filled",
            "statuses": [
                {"status": "open", "information": {"team": "product"}},
                {"status": "filled", "information": {"level": "senior"}}
            ]
        }
    ],
    "connections": [{"from": "director", "to": "engineer"}]
}))
' "$TEST_ID")"

post_graph_update() {
  local name="$1"
  local payload="$2"

  http_request "$name" \
    --request POST \
    --header "Origin: $BASE_URL" \
    --header 'Content-Type: application/json' \
    --cookie "$COOKIE_JAR" \
    --data "$payload" \
    "$BASE_URL/api/profile/update-graph"
}

fetch_public_graph() {
  local name="$1"

  http_request "$name" "$BASE_URL/$ORGANIZATION_SLUG"
  assert_status 200 "$HTTP_STATUS" "Public profile graph"
  GRAPH_DATA="$(python3 -c '
import json,sys
from html.parser import HTMLParser

class GraphDataParser(HTMLParser):
    def __init__(self):
        super().__init__()
        self.in_graph_data = False
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag == "script" and dict(attrs).get("id") == "organization-graph-data":
            self.in_graph_data = True

    def handle_endtag(self, tag):
        if tag == "script" and self.in_graph_data:
            self.in_graph_data = False

    def handle_data(self, data):
        if self.in_graph_data:
            self.parts.append(data)

parser = GraphDataParser()
with open(sys.argv[1], encoding="utf-8") as page:
    parser.feed(page.read())
if not parser.parts:
    raise SystemExit("Public profile did not include organization graph JSON.")
print(json.dumps(json.loads("".join(parser.parts))))
' "$RESPONSE_BODY")"
}

assert_public_graph() {
  local expected_from_title="$1"
  local expected_to_title="$2"
  local expected_status="$3"

  python3 -c '
import json,sys
graph=json.loads(sys.argv[1])
expected_from_title,expected_to_title,expected_status=sys.argv[2:]
positions={position["title"]:position for position in graph["positions"]}
assert set(positions) == {expected_from_title,expected_to_title}, positions
assert len(graph["connections"]) == 1, graph["connections"]
connection=graph["connections"][0]
assert connection == {"from": positions[expected_from_title]["id"], "to": positions[expected_to_title]["id"]}, connection
for title,position in positions.items():
    is_director=title.startswith(("Director ","Updated Director "))
    expected_information={
        "open": {"team": "leadership" if is_director else "product"},
        "filled": {"since": 2020} if is_director else {"level": "senior"}
    }
    assert {status["status"] for status in position["statuses"]} == {"open", "filled"}, position["statuses"]
    for status in position["statuses"]:
        assert status["information"] == expected_information[status["status"]], status
    current=next(status for status in position["statuses"] if status["id"] == position["current_status_id"])
    if title == expected_from_title:
        assert current["status"] == expected_status, current
        assert len(position["statuses"]) == 2, position["statuses"]
' "$GRAPH_DATA" "$expected_from_title" "$expected_to_title" "$expected_status"
  pass "Public graph contains the saved positions, statuses, and connection"
}

post_graph_update "${TEST_ID}-initial-graph-update" "$INITIAL_PAYLOAD"
assert_status 204 "$HTTP_STATUS" "Initial graph update"
fetch_public_graph "${TEST_ID}-initial-public-graph"
DIRECTOR_TITLE="Director ${TEST_ID}"
ENGINEER_TITLE="Engineer ${TEST_ID}"
assert_public_graph "$DIRECTOR_TITLE" "$ENGINEER_TITLE" "open"

INVALID_PAYLOAD="$(python3 -c '
import json,sys
test_id=sys.argv[1]
positions=[
    {"client_id":"director","title":f"Director {test_id}","description":"","current_status":"open","statuses":[{"status":"open","information":{}}]},
    {"client_id":"engineer","title":f"Engineer {test_id}","description":"","current_status":"filled","statuses":[{"status":"filled","information":{}}]}
]
print(json.dumps({"positions":positions,"connections":[{"from":"director","to":"engineer"},{"from":"engineer","to":"director"}]}))
' "$TEST_ID")"
post_graph_update "${TEST_ID}-invalid-graph-update" "$INVALID_PAYLOAD"
assert_status 400 "$HTTP_STATUS" "Graph update with duplicate reverse connection"
fetch_public_graph "${TEST_ID}-graph-after-invalid-update"
assert_public_graph "$DIRECTOR_TITLE" "$ENGINEER_TITLE" "open"

UPDATED_PAYLOAD="$(python3 -c '
import json,sys
graph=json.loads(sys.argv[1])
test_id=sys.argv[2]
positions=[]
for position in graph["positions"]:
    is_director=position["title"].startswith("Director ")
    client_id="director" if is_director else "engineer"
    role="Director" if is_director else "Engineer"
    title=f"Updated {role} {test_id}"
    current=next(status for status in position["statuses"] if status["id"] == position["current_status_id"])
    positions.append({
        "id": position["id"],
        "client_id": client_id,
        "title": title,
        "description": f"Updated description for {client_id}",
        "current_status": current["status"],
        "statuses": [{"status": status["status"],"information": status["information"]} for status in position["statuses"]]
    })
print(json.dumps({"positions":positions,"connections":[{"from":"engineer","to":"director"}]}))
' "$GRAPH_DATA" "$TEST_ID")"
post_graph_update "${TEST_ID}-edited-graph-update" "$UPDATED_PAYLOAD"
assert_status 204 "$HTTP_STATUS" "Graph update using saved position IDs"
fetch_public_graph "${TEST_ID}-updated-public-graph"
UPDATED_DIRECTOR_TITLE="Updated Director ${TEST_ID}"
UPDATED_ENGINEER_TITLE="Updated Engineer ${TEST_ID}"
assert_public_graph "$UPDATED_ENGINEER_TITLE" "$UPDATED_DIRECTOR_TITLE" "filled"

pass "Authenticated graph save, validation, update, and public persistence checks"