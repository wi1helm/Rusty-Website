// routes/pages/signup.rs
use serde::Deserialize;
use axum::{extract::State, Form};
use crate::{Context, api::validation::{ValidationError}};
// This represents that the application does not deem trustworthy.
// #derive means this struct is automaticly creates from form data.
#[derive(Deserialize)]
pub struct SignupRequest {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm: String,
}
// The output, removed confirm as if valid password and confirm is the same.
// This represents data the application deems trustworthy
pub struct ValidatedSignup {
    pub name: String,
    pub email: String,
    pub password: String,
}

struct SignupValidator<'a> {
    input: &'a SignupRequest,
}

impl SignupValidator<'_> {
    fn validate(&self) -> Result<ValidatedSignup, ValidationError> {
        todo!()
    }
}
pub async fn signup(State(context): State<Context>, Form(input): Form<SignupRequest>) -> impl axum::response::IntoResponse {
    let validated = SignupValidator { input: &input }.validate();
    match validated {
        Ok(validated) => {
            // validation succeeded
            // create organization/member
            // redirect somewhere
        }
        //TODO
        Err(error) => {
            // validation failed
            // return signup page / error response
        }
    }
}