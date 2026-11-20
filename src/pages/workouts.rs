use std::collections::HashMap;

use futures::try_join;
use leptos::prelude::*;

use super::workout_new::draft_name;
use crate::api::{self, ApiError};
use crate::models::{NamedSets, Templates, Workouts};
use crate::ui::{
    BackLink, CARD, CHIP, Header, MUTED, ROW_LINK, SECTION_TITLE, SetLine, format_date,
    group_consecutive, id_param, when_loaded,
};

/// Home: start a workout, empty or from a template.
#[component]
pub fn Home() -> impl IntoView {
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
        <Header title="Start" />
        <section>{start_section}</section>
    }
}

/// Past workouts: the calendar, then the list (newest first).
#[component]
pub fn WorkoutList() -> impl IntoView {
    let workouts: LocalResource<Result<Vec<Workouts>, ApiError>> =
        LocalResource::new(api::get_workouts);

    view! {
        <Header title="Workouts" />
        {move || when_loaded(
            workouts.get(),
            |list: Vec<Workouts>| {
                if list.is_empty() {
                    return view! { <p class=MUTED>"No workouts yet."</p> }.into_any();
                }
                view! {
                    <Heatmap workouts=list.clone() />
                    <section>
                        <h2 class=SECTION_TITLE>"History"</h2>
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
                    </section>
                }
                .into_any()
            },
        )}
    }
}

/// How many weeks the calendar shows, this one included.
const WEEKS: u32 = 26;

/// A GitHub-style calendar: one square per day, a column per week (Monday on
/// top), filled on days with a workout. A filled square opens that workout.
#[component]
fn Heatmap(workouts: Vec<Workouts>) -> impl IntoView {
    // date -> (id, name)
    let by_date: HashMap<String, (i32, String)> = workouts
        .into_iter()
        .filter_map(|workout| Some((workout.date, (workout.id?, workout.name))))
        .collect();

    // Local dates, built from year/month/day so DST changes can't skip or
    // repeat a day (JS rolls day 0, 32 and so on into the right month).
    let now: js_sys::Date = js_sys::Date::new_0();
    let weekday: u32 = (now.get_day() + 6) % 7; // Monday = 0
    let first_day: i32 = now.get_date() as i32 - (weekday + (WEEKS - 1) * 7) as i32;
    let days: Vec<String> = (0..=(WEEKS - 1) * 7 + weekday)
        .map(|offset| {
            format_date(&js_sys::Date::new_with_year_month_day(
                now.get_full_year(),
                now.get_month() as i32,
                first_day + offset as i32,
            ))
        })
        .collect();

    let count: usize = days.iter().filter(|day| by_date.contains_key(*day)).count();

    view! {
        <section class="mb-8">
            <h2 class=SECTION_TITLE>
                "Last " {WEEKS} " weeks"
                <span class="sr-only">", " {count} " workouts"</span>
            </h2>
            // Screen readers only hear the filled days, as links; the empty
            // squares are hidden from them.
            <div class="grid auto-cols-fr grid-flow-col grid-rows-7 gap-0.5">
                {days
                    .into_iter()
                    .map(|day| match by_date.get(&day) {
                        Some((id, name)) => view! {
                            <a
                                href=format!("/workouts/{id}")
                                title=format!("{day} {name}")
                                aria-label=format!("{name}, {day}")
                                class="aspect-square rounded-sm bg-stone-900 outline-offset-1 transition-colors hover:bg-stone-600 dark:bg-stone-100 dark:hover:bg-stone-400"
                            ></a>
                        }
                        .into_any(),
                        None => view! {
                            <span
                                aria-hidden="true"
                                class="aspect-square rounded-sm bg-stone-200 dark:bg-stone-800"
                            ></span>
                        }
                        .into_any(),
                    })
                    .collect_view()}
            </div>
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
        <BackLink href="/workouts" label="Workouts" />
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
                                            .map(|named| view! { <SetLine set=named.set /> })
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
