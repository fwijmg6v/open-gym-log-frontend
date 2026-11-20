//! Mirrors the backend's `src/models.rs` field for field, so the JSON lines up.
//! It's a copy rather than a shared crate because the backend structs derive
//! sqlx traits, which don't compile to WASM.
//!
//! Note the nesting: `#[sqlx(flatten)]` only affects the database side, so in
//! JSON a `NamedSets` is `{ "set": { ... }, "exercise_name": "..." }`.

use serde::{Deserialize, Serialize};

/// Sent as "Kg" / "Lb": the backend's serde derive has no rename (its
/// lowercase rename only applies to Postgres).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeightUnit {
    Kg,
    Lb,
}

impl WeightUnit {
    pub fn label(self) -> &'static str {
        match self {
            WeightUnit::Kg => "kg",
            WeightUnit::Lb => "lb",
        }
    }

    pub fn toggled(self) -> Self {
        match self {
            WeightUnit::Kg => WeightUnit::Lb,
            WeightUnit::Lb => WeightUnit::Kg,
        }
    }
}

// Database structs

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workouts {
    pub id: Option<i32>,
    pub name: String,
    /// "YYYY-MM-DD", the same format as `<input type="date">`.
    pub date: String,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Exercises {
    pub id: Option<i32>,
    pub name: String,
    pub notes: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Templates {
    pub id: Option<i32>,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sets {
    pub id: Option<i32>,
    pub w_id: Option<i32>,
    pub e_id: i32,
    /// The backend's `Decimal` travels as a string ("102.5"), which also
    /// avoids float rounding.
    pub weight: Option<String>,
    pub metric: Option<WeightUnit>,
    pub reps: Option<i16>,
    pub rir: Option<i16>,
    pub notes: Option<String>,
    pub set_order: i16,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TemplateExercises {
    pub id: Option<i32>,
    pub t_id: Option<i32>,
    pub e_id: i32,
    pub ex_order: i16,
}

// Specialized structs

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FullWorkout {
    pub workout: Workouts,
    pub sets: Vec<Sets>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DatedSets {
    pub set: Sets,
    pub date: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FullTemplate {
    pub template: Templates,
    pub exercises: Vec<TemplateExercises>,
}

// Response-only structs: rows joined with the exercise name

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedSets {
    pub set: Sets,
    pub exercise_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedTemplateExercises {
    pub template_exercise: TemplateExercises,
    pub exercise_name: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedFullTemplate {
    pub template: Templates,
    pub exercises: Vec<NamedTemplateExercises>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct NamedFullWorkout {
    pub workout: Workouts,
    pub sets: Vec<NamedSets>,
}
