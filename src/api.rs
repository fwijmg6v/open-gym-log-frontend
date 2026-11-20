//! One function per backend endpoint, named after the backend handler it calls
//! (`get_exercises` here calls `get_exercises` in the backend's routes.rs).

use std::fmt;

use gloo_net::http::{Request, RequestBuilder, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use web_sys::AbortSignal;

use crate::models::{
    DatedSets, Exercises, FullTemplate, FullWorkout, NamedFullTemplate, NamedFullWorkout,
    NamedSets, Templates, Workouts,
};

/// Trunk proxies /api/* to the backend (see Trunk.toml).
const API: &str = "/api";

/// Give up on a request after 15 seconds, so a dead connection at the gym
/// shows an error instead of a button that stays disabled for a minute.
const TIMEOUT_MS: u32 = 15_000;

#[derive(Clone, Debug, PartialEq)]
pub enum ApiError {
    /// The request never got an answer (server down, offline, timed out).
    Network(String),
    /// The server answered with a non-2xx status. Pages match on the codes
    /// they expect (404, 409, ...) to show a specific message.
    Status(u16),
    /// The server answered, but not with what this app expects (models.rs
    /// is out of sync with the API).
    BadData(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Network(message) => write!(f, "Can't reach the server ({message})."),
            ApiError::Status(404) => write!(f, "Not found."),
            ApiError::Status(code) => write!(f, "The server returned an error ({code})."),
            ApiError::BadData(message) => {
                write!(f, "The server's reply didn't match this app ({message}).")
            }
        }
    }
}

impl From<gloo_net::Error> for ApiError {
    fn from(error: gloo_net::Error) -> Self {
        match error {
            gloo_net::Error::JsError(js) if js.name == "TimeoutError" => {
                ApiError::Network(String::from("timed out"))
            }
            gloo_net::Error::SerdeError(serde) => ApiError::BadData(serde.to_string()),
            other => ApiError::Network(other.to_string()),
        }
    }
}

fn url(path: &str) -> String {
    format!("{API}{path}")
}

/// Adds the timeout to a request.
fn timed(request: RequestBuilder) -> RequestBuilder {
    request.abort_signal(Some(&AbortSignal::timeout_with_u32(TIMEOUT_MS)))
}

fn check(response: Response) -> Result<Response, ApiError> {
    if response.ok() {
        Ok(response)
    } else {
        Err(ApiError::Status(response.status()))
    }
}

async fn fetch_json<T: DeserializeOwned>(path: &str) -> Result<T, ApiError> {
    let response: Response = check(timed(Request::get(&url(path))).send().await?)?;
    let body: T = response.json().await?;
    Ok(body)
}

async fn send_json<B: Serialize>(request: RequestBuilder, body: &B) -> Result<(), ApiError> {
    check(timed(request).json(body)?.send().await?)?;
    Ok(())
}

async fn send_delete(path: &str) -> Result<(), ApiError> {
    check(timed(Request::delete(&url(path))).send().await?)?;
    Ok(())
}

// Exercises

pub async fn get_exercises() -> Result<Vec<Exercises>, ApiError> {
    fetch_json("/exercises").await
}

pub async fn get_exercise(id: i32) -> Result<Exercises, ApiError> {
    fetch_json(&format!("/exercises/{id}")).await
}

pub async fn get_sets_by_exercise(id: i32) -> Result<Vec<DatedSets>, ApiError> {
    fetch_json(&format!("/exercises/{id}/sets")).await
}

pub async fn post_exercise(exercise: &Exercises) -> Result<(), ApiError> {
    send_json(Request::post(&url("/exercises")), exercise).await
}

pub async fn put_exercise(id: i32, exercise: &Exercises) -> Result<(), ApiError> {
    send_json(Request::put(&url(&format!("/exercises/{id}"))), exercise).await
}

pub async fn delete_exercise(id: i32) -> Result<(), ApiError> {
    send_delete(&format!("/exercises/{id}")).await
}

// Templates

pub async fn get_templates() -> Result<Vec<Templates>, ApiError> {
    fetch_json("/templates").await
}

pub async fn get_full_template(id: i32) -> Result<NamedFullTemplate, ApiError> {
    fetch_json(&format!("/templates/{id}")).await
}

pub async fn get_workout_from_template(id: i32) -> Result<NamedFullWorkout, ApiError> {
    fetch_json(&format!("/templates/{id}/start")).await
}

pub async fn post_template(template: &FullTemplate) -> Result<(), ApiError> {
    send_json(Request::post(&url("/templates")), template).await
}

pub async fn put_template(id: i32, template: &FullTemplate) -> Result<(), ApiError> {
    send_json(Request::put(&url(&format!("/templates/{id}"))), template).await
}

pub async fn delete_template(id: i32) -> Result<(), ApiError> {
    send_delete(&format!("/templates/{id}")).await
}

// Workouts

pub async fn get_workouts() -> Result<Vec<Workouts>, ApiError> {
    fetch_json("/workouts").await
}

pub async fn get_workout(id: i32) -> Result<Workouts, ApiError> {
    fetch_json(&format!("/workouts/{id}")).await
}

pub async fn get_sets_by_workout(id: i32) -> Result<Vec<NamedSets>, ApiError> {
    fetch_json(&format!("/workouts/{id}/sets")).await
}

pub async fn post_workout(workout: &FullWorkout) -> Result<(), ApiError> {
    send_json(Request::post(&url("/workouts")), workout).await
}
