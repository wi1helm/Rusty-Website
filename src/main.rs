mod database;
mod routes;

#[derive(Clone)]
pub struct Context {
    pub database: database::PostgresDatabase,
}

#[tokio::main]
async fn main() {
    // Get the Database URL from environment variable set in compose.yaml
    // Panic if no url is set
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    // Open Postgres Connection using url and wait for it to connect
    // Panic if no connection is done
    let database = database::PostgresDatabase::new(&database_url)
        .await
        .expect("failed to connect to database");

    // Create the context of this application
    let context = Context { database };

    // Create the app
    let app = routes::router(context);

    // Bind the listener to port and ip
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}
