use futures::try_join;
use leptos::prelude::*;

use super::workout_new::draft_name;
use crate::api::{self, ApiError};
use crate::models::{NamedSets, Templates, Workouts};
use crate::ui::{
    BackLink, CARD, CHIP, Header, MUTED, ROW_LINK, SECTION_TITLE, describe_set, group_consecutive,
    id_param, when_loaded,
};

/// Home: start a workout (empty or from a template), then the history.
#[component]
pub fn WorkoutList() -> impl IntoView {
    let workouts: LocalResource<Result<Vec<Workouts>, ApiError>> =
        LocalResource::new(api::get_workouts);

    // An unsaved workout replaces the start buttons: any of them would open
    // it anyway, until it's saved or discarded.
    let start_section: AnyView = match draft_name() {
        Some(name) => view! {
            <h2 class=SECTION_TITLE>"Unsaved workout"</h2>
            <a href="/workouts/new" class=CHIP>"Continue " {name}</a>
        }
        .into_any(),
        None => {
            // Only fetched when the buttons are shown.
            let templates: LocalResource<Result<Vec<Templates>, ApiError>> =
                LocalResource::new(api::get_templates);
            view! {
                <h2 class=SECTION_TITLE>"Start a workout"</h2>
                <div class="flex flex-wrap gap-2">
                    <a href="/workouts/new" class=CHIP>"Empty"</a>
                    {move || when_loaded(
                        templates.get(),
                        |list: Vec<Templates>| {
                            list.into_iter()
                                .filter_map(|template| {
                                    let id: i32 = template.id?;
                                    Some(view! {
                                        <a href=format!("/workouts/new?template={id}") class=CHIP>
                                            {template.name}
                                        </a>
                                    })
                                })
                                .collect_view()
                        },
                    )}
                </div>
            }
            .into_any()
        }
    };

    view! {
        <Header title="Workouts" />
        <section class="mb-8">{start_section}</section>
        <section>
            <h2 class=SECTION_TITLE>"History"</h2>
            {move || when_loaded(
                workouts.get(),
                |list: Vec<Workouts>| {
                    if list.is_empty() {
                        return view! { <p class=MUTED>"No workouts yet."</p> }.into_any();
                    }
                    view! {
                        <ul class=CARD role="list">
                            {list.into_iter()
                                .filter_map(|workout| {
                                    let id: i32 = workout.id?;
                                    Some(view! {
                                        <li>
                                            <a href=format!("/workouts/{id}") class=ROW_LINK>
                                                <span class="truncate font-medium">{workout.name}</span>
                                                <span class="shrink-0 text-sm text-stone-500 tabular-nums dark:text-stone-400">
                                                    {workout.date}
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
        </section>
    }
}

/// One logged workout, read-only (the API has no workout editing).
#[component]
pub fn WorkoutDetail() -> impl IntoView {
    let route_id: Memo<Option<i32>> = id_param();
    let data: LocalResource<Result<(Workouts, Vec<NamedSets>), ApiError>> =
        LocalResource::new(move || load_workout(route_id.get()));

    view! {
        <BackLink href="/" label="Workouts" />
        {move || when_loaded(
            data.get(),
            |(workout, sets): (Workouts, Vec<NamedSets>)| {
                // Consecutive sets of the same exercise become one card.
                let groups: Vec<(String, Vec<NamedSets>)> = group_consecutive(
                    sets,
                    |named: &NamedSets| named.exercise_name.clone(),
                );
                view! {
                    <Header title=workout.name />
                    <p class="-mt-4 mb-6 text-sm text-stone-500 tabular-nums dark:text-stone-400">{workout.date}</p>
                    {workout.notes.map(|notes| view! {
                        <p class="mb-6 whitespace-pre-line text-stone-600 dark:text-stone-400">{notes}</p>
                    })}
                    {groups.is_empty().then(|| view! { <p class=MUTED>"No sets."</p> })}
                    <div class="space-y-3">
                        {groups
                            .into_iter()
                            .map(|(exercise_name, sets)| view! {
                                <section class="rounded-xl border border-stone-200 bg-white px-4 py-3 dark:border-stone-800 dark:bg-stone-900">
                                    <h2 class="font-medium">{exercise_name}</h2>
                                    <ol class="mt-1 space-y-0.5 text-sm text-stone-600 tabular-nums dark:text-stone-400" role="list">
                                        {sets
                                            .into_iter()
                                            .map(|named| view! { <li>{describe_set(&named.set)}</li> })
                                            .collect_view()}
                                    </ol>
                                </section>
                            })
                            .collect_view()}
                    </div>
                }
            },
        )}
    }
}

async fn load_workout(id: Option<i32>) -> Result<(Workouts, Vec<NamedSets>), ApiError> {
    let id: i32 = id.ok_or(ApiError::Status(404))?;
    // Both requests at once; the first error wins.
    let (workout, sets): (Workouts, Vec<NamedSets>) =
        try_join!(api::get_workout(id), api::get_sets_by_workout(id))?;
    Ok((workout, sets))
}
