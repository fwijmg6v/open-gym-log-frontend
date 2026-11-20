use futures::join;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::{use_navigate, use_query_map};
use leptos_router::params::ParamsMap;
use serde::{Deserialize, Serialize};
use web_sys::Storage;

use crate::api::{self, ApiError};
use crate::models::{Exercises, FullWorkout, NamedFullWorkout, Sets, WeightUnit, Workouts};
use crate::ui::{
    BUTTON, BUTTON_DANGER, BUTTON_QUIET, BackLink, CARD, ERROR, ErrorText, ExerciseSelect,
    ICON_BUTTON, INPUT, MUTED, NUMBER_INPUT, SECTION_TITLE, TITLE_INPUT, announce, confirm,
    focus_after_remove, next_key, non_empty, replace_page, today, when_loaded,
};

/// `/workouts/new` starts an empty workout. `/workouts/new?template=3` starts
/// from template 3 (one empty set per template row). An unsaved workout from
/// before wins over both: it comes back until it's saved or discarded.
#[component]
pub fn WorkoutNew() -> impl IntoView {
    let query: Memo<ParamsMap> = use_query_map();
    let data: LocalResource<Result<NewWorkout, ApiError>> = LocalResource::new(move || {
        let template_id: Option<i32> = query
            .with(|map| map.get("template"))
            .and_then(|raw| raw.parse().ok());
        load(template_id)
    });

    view! {
        <BackLink href="/" label="Workouts" />
        {move || when_loaded(
            data.get(),
            |page: NewWorkout| view! {
                <WorkoutForm draft=page.draft restored=page.restored exercises=page.exercises />
            },
        )}
    }
}

/// What the page loads: the workout to show, whether it's an unsaved one
/// coming back, and every exercise for the "add set" picker. The list can
/// fail on its own (no signal at the gym) and the workout still opens.
#[derive(Clone)]
struct NewWorkout {
    draft: Draft,
    restored: bool,
    exercises: Result<Vec<Exercises>, ApiError>,
}

async fn load(template_id: Option<i32>) -> Result<NewWorkout, ApiError> {
    if let Some(draft) = load_draft() {
        return Ok(NewWorkout {
            draft,
            restored: true,
            exercises: api::get_exercises().await,
        });
    }
    let (draft, exercises): (Draft, Result<Vec<Exercises>, ApiError>) = match template_id {
        Some(id) => {
            // Both requests at once. Only the template is needed for the form.
            let (skeleton, exercises): (
                Result<NamedFullWorkout, ApiError>,
                Result<Vec<Exercises>, ApiError>,
            ) = join!(api::get_workout_from_template(id), api::get_exercises());
            (Draft::from_template(skeleton?), exercises)
        }
        None => (Draft::empty(), api::get_exercises().await),
    };
    Ok(NewWorkout {
        draft,
        restored: false,
        exercises,
    })
}

/// The workout as typed. It's kept in localStorage after every change until
/// it's saved or discarded, so leaving the page (or iOS reloading the app)
/// doesn't lose it.
#[derive(Clone, Serialize, Deserialize)]
struct Draft {
    name: String,
    /// "YYYY-MM-DD", the format of `<input type="date">`.
    date: String,
    notes: String,
    rows: Vec<SetRow>,
}

impl Draft {
    fn empty() -> Self {
        Draft {
            name: String::from("Workout"),
            date: today(),
            notes: String::new(),
            rows: Vec::new(),
        }
    }

    /// The server stamps template workouts with its UTC date. The phone's
    /// local date is the one the user means (late-evening sessions).
    fn from_template(skeleton: NamedFullWorkout) -> Self {
        Draft {
            name: skeleton.workout.name,
            date: today(),
            notes: skeleton.workout.notes.unwrap_or_default(),
            rows: skeleton
                .sets
                .into_iter()
                .map(|named| {
                    SetRow::new(
                        named.set.e_id,
                        named.exercise_name,
                        named.set.weight.unwrap_or_default(),
                        named.set.metric.unwrap_or(WeightUnit::Kg),
                    )
                })
                .collect(),
        }
    }
}

/// Where the unsaved workout lives in localStorage.
const DRAFT_KEY: &str = "workout-draft";

/// `None` when storage is turned off; the form still works, it just isn't kept.
fn storage() -> Option<Storage> {
    web_sys::window()?.local_storage().ok()?
}

/// The unsaved workout, if there is one. One that doesn't parse (say, saved
/// by an older version of the app) counts as none.
fn load_draft() -> Option<Draft> {
    let stored: Option<String> = storage()?.get_item(DRAFT_KEY).ok().flatten();
    serde_json::from_str(&stored?).ok()
}

fn save_draft(draft: &Draft) {
    let (Some(storage), Ok(json)) = (storage(), serde_json::to_string(draft)) else {
        return;
    };
    // If storage is full or blocked, the draft just isn't kept.
    storage.set_item(DRAFT_KEY, &json).ok();
}

fn clear_draft() {
    if let Some(storage) = storage() {
        storage.remove_item(DRAFT_KEY).ok();
    }
}

/// The unsaved workout's name, for the home page's "Continue" button.
pub fn draft_name() -> Option<String> {
    load_draft().map(|draft| draft.name)
}

/// One set being logged, as typed. Plain data: the whole list lives in one
/// signal, and each input writes its own field back by `key`.
#[derive(Clone, Serialize, Deserialize)]
struct SetRow {
    /// Only means something during one page load, so it isn't stored:
    /// restored rows get fresh keys.
    #[serde(skip, default = "next_key")]
    key: usize,
    e_id: i32,
    exercise_name: String,
    weight: String,
    metric: WeightUnit,
    reps: String,
    rir: String,
}

impl SetRow {
    fn new(e_id: i32, exercise_name: String, weight: String, metric: WeightUnit) -> Self {
        SetRow {
            key: next_key(),
            e_id,
            exercise_name,
            weight,
            metric,
            reps: String::new(),
            rir: String::new(),
        }
    }

    /// Same exercise, weight, and unit, with a new key. Reps and RIR start empty.
    fn copy(&self) -> Self {
        SetRow::new(
            self.e_id,
            self.exercise_name.clone(),
            self.weight.clone(),
            self.metric,
        )
    }
}

/// Changes one field of the row with this key.
fn edit_row(rows: RwSignal<Vec<SetRow>>, key: usize, change: impl FnOnce(&mut SetRow)) {
    rows.update(|list| {
        if let Some(row) = list.iter_mut().find(|row| row.key == key) {
            change(row);
        }
    });
}

#[component]
fn WorkoutForm(
    draft: Draft,
    restored: bool,
    exercises: Result<Vec<Exercises>, ApiError>,
) -> impl IntoView {
    let navigate = use_navigate();
    let name: RwSignal<String> = RwSignal::new(draft.name);
    let date: RwSignal<String> = RwSignal::new(draft.date);
    let notes: RwSignal<String> = RwSignal::new(draft.notes);
    let rows: RwSignal<Vec<SetRow>> = RwSignal::new(draft.rows);
    // Without the list, the picker shows why instead; the rest still works.
    let picker_error: Option<String> = exercises.as_ref().err().map(ApiError::to_string);
    let exercises: StoredValue<Vec<Exercises>> = StoredValue::new(exercises.unwrap_or_default());
    let selected: RwSignal<String> = RwSignal::new(String::new());
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let saving: RwSignal<bool> = RwSignal::new(false);
    // Read out by screen readers after a set is removed.
    let announcement: RwSignal<String> = RwSignal::new(String::new());

    // Keep the draft after every change. The first run only starts tracking
    // (`has_run` is None), so opening a template and leaving keeps nothing.
    Effect::new(move |has_run: Option<()>| {
        let draft: Draft = Draft {
            name: name.get(),
            date: date.get(),
            notes: notes.get(),
            rows: rows.get(),
        };
        if has_run.is_some() {
            save_draft(&draft);
        }
    });

    let discard = {
        let navigate = navigate.clone();
        move |_| {
            if confirm("Discard this workout? It hasn't been saved.") {
                clear_draft();
                navigate("/", replace_page());
            }
        }
    };

    let add_set = move |_| {
        let e_id: Option<i32> = selected.get_untracked().parse().ok();
        let Some(e_id) = e_id else {
            return;
        };
        let exercise_name: Option<String> = exercises.with_value(|list| {
            list.iter()
                .find(|exercise| exercise.id == Some(e_id))
                .map(|exercise| exercise.name.clone())
        });
        let Some(exercise_name) = exercise_name else {
            return;
        };
        rows.update(|list| {
            // Start from the last set of this exercise, so a repeat set only
            // needs reps. A new exercise gets the unit of the last set.
            let row: SetRow = match list.iter().rev().find(|row| row.e_id == e_id) {
                Some(previous) => previous.copy(),
                None => SetRow::new(
                    e_id,
                    exercise_name,
                    String::new(),
                    list.last().map(|row| row.metric).unwrap_or(WeightUnit::Kg),
                ),
            };
            list.push(row);
        });
    };

    let save = move |ev: SubmitEvent| {
        ev.prevent_default();
        let workout_name: String = name.get_untracked();
        let workout_date: String = date.get_untracked();
        let workout_notes: String = notes.get_untracked();
        let workout: FullWorkout = match rows
            .with_untracked(|list| build_workout(workout_name, workout_date, workout_notes, list))
        {
            Ok(workout) => workout,
            Err(message) => {
                error.set(Some(message));
                return;
            }
        };
        error.set(None);
        saving.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            match api::post_workout(&workout).await {
                Ok(()) => {
                    clear_draft();
                    navigate("/", replace_page());
                }
                Err(failure) => {
                    error.set(Some(match failure {
                        ApiError::Status(409) => {
                            String::from("There's already a workout on this date.")
                        }
                        ApiError::Status(400) => {
                            String::from("One of these exercises no longer exists.")
                        }
                        other => other.to_string(),
                    }));
                    saving.set(false);
                }
            }
        });
    };

    view! {
        // Any typing clears the last error.
        <form class="space-y-8" on:submit=save on:input=move |_| error.set(None)>
            <div class="space-y-3">
                // Also shown when a template's Start link led here: that
                // template can only start once this one is saved or discarded.
                {restored
                    .then(|| {
                        view! {
                            <p class=MUTED>
                                "Restored your unsaved workout. Save or discard it to start another."
                            </p>
                        }
                    })}
                <h1>
                    <input
                        class=TITLE_INPUT
                        placeholder="Workout name"
                        aria-label="Workout name"
                        maxlength="20"
                        required=true
                        bind:value=name
                    />
                </h1>
                <input type="date" class=INPUT aria-label="Date" required=true bind:value=date />
                <textarea
                    class=INPUT
                    rows="2"
                    placeholder="Notes"
                    aria-label="Notes"
                    maxlength="200"
                    bind:value=notes
                ></textarea>
            </div>

            <section>
                <h2 class=SECTION_TITLE>"Sets"</h2>
                <Show
                    when=move || rows.with(|list| !list.is_empty())
                    fallback=|| view! { <p class=MUTED>"No sets yet. Add one below."</p> }
                >
                    <ol class=CARD role="list">
                        <For
                            each=move || rows.get()
                            key=|row: &SetRow| row.key
                            children=move |row: SetRow| {
                                view! { <SetRowView row=row rows=rows announcement=announcement /> }
                            }
                        />
                    </ol>
                </Show>
                <p role="status" class="sr-only">{move || announcement.get()}</p>
                <div class="mt-3">
                    {match picker_error {
                        Some(message) => view! { <p class=ERROR>{message}</p> }.into_any(),
                        None => view! {
                            <div class="flex gap-2">
                                <ExerciseSelect exercises=exercises.get_value() selected=selected />
                                <button
                                    type="button"
                                    class=BUTTON_QUIET
                                    disabled=move || selected.with(|id| id.is_empty())
                                    on:click=add_set
                                >
                                    "Add set"
                                </button>
                            </div>
                        }
                        .into_any(),
                    }}
                </div>
            </section>

            <div class="space-y-3">
                <ErrorText message=error />
                <div class="flex items-center justify-between">
                    <button type="submit" class=BUTTON disabled=move || saving.get()>
                        {move || if saving.get() { "Saving…" } else { "Save workout" }}
                    </button>
                    <button
                        type="button"
                        class=BUTTON_DANGER
                        disabled=move || saving.get()
                        on:click=discard
                    >
                        "Discard"
                    </button>
                </div>
            </div>
        </form>
    }
}

/// One row of inputs. It's rendered once per key; the inputs start from the
/// row's values and write changes back with `edit_row`.
#[component]
fn SetRowView(
    row: SetRow,
    rows: RwSignal<Vec<SetRow>>,
    announcement: RwSignal<String>,
) -> impl IntoView {
    let key: usize = row.key;
    // Buttons say which exercise they act on.
    let add_label: String = format!("Add another {} set", row.exercise_name);
    let remove_label: String = format!("Remove {} set", row.exercise_name);
    let removed: String = format!("{} set removed", row.exercise_name);

    // Inserts a copy right below this row.
    let duplicate = move |_| {
        rows.update(|list| {
            if let Some(index) = list.iter().position(|other| other.key == key) {
                let copy: SetRow = list[index].copy();
                list.insert(index + 1, copy);
            }
        });
    };
    let toggle_unit = move |_| edit_row(rows, key, |row| row.metric = row.metric.toggled());
    let unit_label = move || {
        rows.with(|list| {
            list.iter()
                .find(|other| other.key == key)
                .map(|other| other.metric.label())
        })
    };

    view! {
        <li class="px-4 py-3">
            <div class="mb-2 flex items-center justify-between gap-2">
                <span class="truncate text-sm font-medium">{row.exercise_name}</span>
                <div class="flex gap-2">
                    <button
                        type="button"
                        class=ICON_BUTTON
                        aria-label=add_label
                        on:click=duplicate
                    >
                        "+"
                    </button>
                    <button
                        type="button"
                        class=ICON_BUTTON
                        aria-label=remove_label
                        on:click:target=move |ev| {
                            // Focus moves to a neighbouring row first, so it isn't dropped.
                            focus_after_remove(&ev.target());
                            rows.update(|list| list.retain(|other| other.key != key));
                            announce(announcement, removed.clone());
                        }
                    >
                        "✕"
                    </button>
                </div>
            </div>
            // The browser checks `required` and `pattern` on submit and focuses the
            // first bad field; build_workout checks the same rules again.
            <div class="flex items-center gap-2">
                <input
                    class=NUMBER_INPUT
                    inputmode="decimal"
                    placeholder="Weight"
                    aria-label="Weight"
                    required=true
                    pattern="[0-9]{1,3}([.,][0-9])?"
                    value=row.weight
                    on:input:target=move |ev| edit_row(rows, key, |row| row.weight = ev.target().value())
                />
                // The visible "kg"/"lb" stays in the button's name for screen readers.
                <button
                    type="button"
                    class="w-12 shrink-0 rounded-lg border border-stone-300 py-1.5 text-sm font-medium transition-colors hover:bg-stone-100 dark:border-stone-700 dark:hover:bg-stone-800"
                    on:click=toggle_unit
                >
                    {unit_label}
                    <span class="sr-only">", switch unit"</span>
                </button>
                <span class="text-stone-500 dark:text-stone-400">"×"</span>
                <input
                    class=NUMBER_INPUT
                    inputmode="numeric"
                    placeholder="Reps"
                    aria-label="Reps"
                    required=true
                    pattern="[0-9]{1,3}"
                    value=row.reps
                    on:input:target=move |ev| edit_row(rows, key, |row| row.reps = ev.target().value())
                />
                <input
                    class=NUMBER_INPUT
                    inputmode="numeric"
                    placeholder="RIR"
                    aria-label="Reps in reserve (optional)"
                    pattern="[0-9]{1,2}"
                    value=row.rir
                    on:input:target=move |ev| edit_row(rows, key, |row| row.rir = ev.target().value())
                />
            </div>
        </li>
    }
}

/// Checks the form and builds the POST body. The API's sets columns are
/// NOT NULL, so every set needs a weight and reps. `set_order` is renumbered
/// 1, 2, 3... from the on-screen order.
fn build_workout(
    name: String,
    date: String,
    notes: String,
    rows: &[SetRow],
) -> Result<FullWorkout, String> {
    let name: String = name.trim().to_string();
    if name.is_empty() {
        return Err(String::from("Give the workout a name."));
    }
    if date.is_empty() {
        return Err(String::from("Pick a date."));
    }
    if rows.is_empty() {
        return Err(String::from("Add at least one set."));
    }

    let mut sets: Vec<Sets> = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        let number: usize = index + 1;
        let label: String = format!("Set {number} ({})", row.exercise_name);

        let weight: String = parse_weight(&row.weight)
            .ok_or_else(|| format!("{label}: enter a weight like 102.5 (max 999.9)."))?;
        let reps: i16 =
            parse_count(&row.reps).ok_or_else(|| format!("{label}: enter the reps."))?;
        let rir: Option<i16> = if row.rir.trim().is_empty() {
            None
        } else {
            let value: i16 = parse_count(&row.rir)
                .ok_or_else(|| format!("{label}: RIR must be a whole number."))?;
            Some(value)
        };

        sets.push(Sets {
            id: None,
            w_id: None,
            e_id: row.e_id,
            weight: Some(weight),
            metric: Some(row.metric),
            reps: Some(reps),
            rir,
            notes: None,
            set_order: number as i16,
        });
    }

    Ok(FullWorkout {
        workout: Workouts {
            id: None,
            name,
            date,
            notes: non_empty(notes),
        },
        sets,
    })
}

/// Checks the weight against the `NUMERIC(4,1)` column: up to 3 digits, then
/// optionally one decimal ("102.5"). Accepts a comma too (iPhone keyboards in
/// comma locales) and returns the dot form. A looser check would let "999.95"
/// through, which Postgres rounds to 1000.0 and rejects.
fn parse_weight(input: &str) -> Option<String> {
    let normalized: String = input.trim().replace(',', ".");
    let (whole, fraction): (&str, &str) = normalized
        .split_once('.')
        .unwrap_or((normalized.as_str(), ""));
    let whole_ok: bool =
        (1..=3).contains(&whole.len()) && whole.chars().all(|c| c.is_ascii_digit());
    let fraction_ok: bool = fraction.len() <= 1 && fraction.chars().all(|c| c.is_ascii_digit());
    if !(whole_ok && fraction_ok) {
        return None;
    }
    if fraction.is_empty() {
        Some(whole.to_string())
    } else {
        Some(format!("{whole}.{fraction}"))
    }
}

/// A whole number from 0 up, for reps and RIR.
fn parse_count(input: &str) -> Option<i16> {
    let value: i16 = input.trim().parse().ok()?;
    (value >= 0).then_some(value)
}
