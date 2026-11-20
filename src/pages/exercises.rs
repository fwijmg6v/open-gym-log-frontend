use futures::try_join;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;

use crate::api::{self, ApiError};
use crate::models::{DatedSets, Exercises};
use crate::ui::{
    BUTTON, BUTTON_DANGER, BackLink, CARD, ErrorText, Header, INPUT, MUTED, ROW_LINK,
    SECTION_TITLE, SetLine, TITLE_INPUT, confirm, group_consecutive, id_param, non_empty,
    replace_page, when_loaded,
};

#[component]
pub fn ExerciseList() -> impl IntoView {
    let exercises: LocalResource<Result<Vec<Exercises>, ApiError>> =
        LocalResource::new(api::get_exercises);
    let name: RwSignal<String> = RwSignal::new(String::new());
    let notes: RwSignal<String> = RwSignal::new(String::new());
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let busy: RwSignal<bool> = RwSignal::new(false);

    let add = move |ev: SubmitEvent| {
        ev.prevent_default();
        let exercise: Exercises = Exercises {
            id: None,
            name: name.get_untracked().trim().to_string(),
            notes: non_empty(notes.get_untracked()),
        };
        if exercise.name.is_empty() {
            error.set(Some(String::from("Give the exercise a name.")));
            return;
        }
        error.set(None);
        busy.set(true);
        spawn_local(async move {
            match api::post_exercise(&exercise).await {
                Ok(()) => {
                    name.set(String::new());
                    notes.set(String::new());
                    exercises.refetch();
                }
                Err(ApiError::Status(409)) => error.set(Some(String::from(
                    "An exercise with that name already exists.",
                ))),
                Err(failure) => error.set(Some(failure.to_string())),
            }
            busy.set(false);
        });
    };

    view! {
        <Header title="Exercises" />
        // Any typing clears the last error.
        <form class="mb-8 space-y-2" on:submit=add on:input=move |_| error.set(None)>
            <input
                class=INPUT
                placeholder="New exercise"
                aria-label="New exercise name"
                maxlength="20"
                required=true
                bind:value=name
            />
            <input
                class=INPUT
                placeholder="Notes, e.g. seat on 4"
                aria-label="Notes"
                maxlength="200"
                bind:value=notes
            />
            <ErrorText message=error />
            <button type="submit" class=BUTTON disabled=move || busy.get()>"Add exercise"</button>
        </form>
        {move || when_loaded(
            exercises.get(),
            |list: Vec<Exercises>| {
                if list.is_empty() {
                    return view! { <p class=MUTED>"No exercises yet."</p> }.into_any();
                }
                view! {
                    <ul class=CARD role="list">
                        {list.into_iter()
                            .filter_map(|exercise| {
                                let id: i32 = exercise.id?;
                                Some(view! {
                                    <li>
                                        <a href=format!("/exercises/{id}") class=ROW_LINK>
                                            <span class="shrink-0 font-medium">{exercise.name}</span>
                                            <span class="truncate text-sm text-stone-500 dark:text-stone-400">
                                                {exercise.notes}
                                            </span>
                                        </a>
                                    </li>
                                })
                            })
                            .collect_view()}
                    </ul>
                }
                .into_any()
            },
        )}
    }
}

/// What the exercise page loads: its id, the exercise, and its logged sets.
type ExercisePage = (i32, Exercises, Vec<DatedSets>);

#[component]
pub fn ExerciseDetail() -> impl IntoView {
    let route_id: Memo<Option<i32>> = id_param();
    let data: LocalResource<Result<ExercisePage, ApiError>> =
        LocalResource::new(move || load_exercise(route_id.get()));

    view! {
        <BackLink href="/exercises" label="Exercises" />
        {move || when_loaded(
            data.get(),
            |(id, exercise, history): ExercisePage| view! {
                <ExerciseEditor id=id exercise=exercise history=history />
            },
        )}
    }
}

async fn load_exercise(id: Option<i32>) -> Result<ExercisePage, ApiError> {
    let id: i32 = id.ok_or(ApiError::Status(404))?;
    // Both requests at once; the first error wins.
    let (exercise, history): (Exercises, Vec<DatedSets>) =
        try_join!(api::get_exercise(id), api::get_sets_by_exercise(id))?;
    Ok((id, exercise, history))
}

#[component]
fn ExerciseEditor(id: i32, exercise: Exercises, history: Vec<DatedSets>) -> impl IntoView {
    let navigate = use_navigate();
    let name: RwSignal<String> = RwSignal::new(exercise.name);
    let notes: RwSignal<String> = RwSignal::new(exercise.notes.unwrap_or_default());
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let saved: RwSignal<bool> = RwSignal::new(false);
    let busy: RwSignal<bool> = RwSignal::new(false);

    let save = move |ev: SubmitEvent| {
        ev.prevent_default();
        let exercise: Exercises = Exercises {
            id: None,
            name: name.get_untracked().trim().to_string(),
            notes: non_empty(notes.get_untracked()),
        };
        if exercise.name.is_empty() {
            error.set(Some(String::from("Give the exercise a name.")));
            return;
        }
        error.set(None);
        busy.set(true);
        spawn_local(async move {
            match api::put_exercise(id, &exercise).await {
                Ok(()) => saved.set(true),
                Err(ApiError::Status(409)) => error.set(Some(String::from(
                    "Another exercise already has that name.",
                ))),
                Err(failure) => error.set(Some(failure.to_string())),
            }
            busy.set(false);
        });
    };

    let delete = move |_| {
        if !confirm("Delete this exercise?") {
            return;
        }
        error.set(None);
        busy.set(true);
        let navigate = navigate.clone();
        spawn_local(async move {
            match api::delete_exercise(id).await {
                Ok(()) | Err(ApiError::Status(404)) => navigate("/exercises", replace_page()),
                Err(failure) => {
                    error.set(Some(match failure {
                        ApiError::Status(409) => String::from(
                            "It's used by logged sets or a template, so it can't be deleted.",
                        ),
                        other => other.to_string(),
                    }));
                    busy.set(false);
                }
            }
        });
    };

    // Newest day first, sets in order within a day (the API's order).
    let history_view: AnyView = if history.is_empty() {
        view! { <p class=MUTED>"Not logged yet."</p> }.into_any()
    } else {
        let days: Vec<(String, Vec<DatedSets>)> =
            group_consecutive(history, |dated: &DatedSets| dated.date.clone());
        view! {
            <ul class=CARD role="list">
                {days
                    .into_iter()
                    .map(|(date, sets)| view! {
                        // One set per line, like a logged workout's page.
                        <li class="px-4 py-3">
                            <p class="text-sm text-stone-500 tabular-nums dark:text-stone-400">{date}</p>
                            <ol class="mt-1 space-y-0.5 tabular-nums" role="list">
                                {sets
                                    .into_iter()
                                    .map(|dated| view! { <SetLine set=dated.set /> })
                                    .collect_view()}
                            </ol>
                        </li>
                    })
                    .collect_view()}
            </ul>
        }
        .into_any()
    };

    view! {
        // Any typing hides the old "Saved." message and the last error.
        <form
            class="space-y-3"
            on:submit=save
            on:input=move |_| {
                saved.set(false);
                error.set(None);
            }
        >
            <h1>
                <input
                    class=TITLE_INPUT
                    placeholder="Exercise name"
                    aria-label="Exercise name"
                    maxlength="20"
                    required=true
                    bind:value=name
                />
            </h1>
            <textarea
                class=INPUT
                rows="3"
                placeholder="Notes: machine setup, cues…"
                aria-label="Notes"
                maxlength="200"
                bind:value=notes
            ></textarea>
            <ErrorText message=error />
            <div class="flex items-center justify-between">
                <div class="flex items-center gap-3">
                    <button type="submit" class=BUTTON disabled=move || busy.get()>"Save"</button>
                    // Always in the page so screen readers announce "Saved.".
                    <span role="status" class=MUTED>
                        {move || saved.get().then_some("Saved.")}
                    </span>
                </div>
                <button
                    type="button"
                    class=BUTTON_DANGER
                    disabled=move || busy.get()
                    on:click=delete
                >
                    "Delete"
                </button>
            </div>
        </form>
        <section class="mt-10">
            <h2 class=SECTION_TITLE>"History"</h2>
            {history_view}
        </section>
    }
}
