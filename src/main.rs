mod routes;

#[tokio::main]
async fn main() {
    let app = routes::router();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app).await.unwrap();
}
