use futures::try_join;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;
use leptos_router::hooks::use_navigate;
use web_sys::HtmlElement;

use crate::api::{self, ApiError};
use crate::models::{Exercises, FullTemplate, NamedFullTemplate, TemplateExercises, Templates};
use crate::ui::{
    BUTTON, BUTTON_DANGER, BUTTON_QUIET, BackLink, CARD, CHIP, ErrorText, ExerciseSelect, Header,
    ICON_BUTTON, MUTED, SECTION_TITLE, TITLE_INPUT, announce, confirm, focus_after_remove,
    id_param, next_key, replace_page, when_loaded,
};

#[component]
pub fn TemplateList() -> impl IntoView {
    let templates: LocalResource<Result<Vec<Templates>, ApiError>> =
        LocalResource::new(api::get_templates);

    view! {
        <Header title="Templates">
            <a href="/templates/new" class=BUTTON>"New"</a>
        </Header>
        {move || when_loaded(
            templates.get(),
            |list: Vec<Templates>| {
                if list.is_empty() {
                    return view! { <p class=MUTED>"No templates yet."</p> }.into_any();
                }
                view! {
                    <ul class=CARD role="list">
                        {list.into_iter()
                            .filter_map(|template| {
                                let id: i32 = template.id?;
                                // Every row has a "Start"; say which one.
                                let start_label: String = format!("Start {}", template.name);
                                Some(view! {
                                    <li class="flex items-center pr-3">
                                        <a
                                            href=format!("/templates/{id}")
                                            class="min-w-0 flex-1 truncate px-4 py-3 font-medium -outline-offset-2 transition-colors hover:text-stone-500"
                                        >
                                            {template.name}
                                        </a>
                                        <a
                                            href=format!("/workouts/new?template={id}")
                                            class=CHIP
                                            aria-label=start_label
                                        >
                                            "Start"
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

#[component]
pub fn TemplateNew() -> impl IntoView {
    let exercises: LocalResource<Result<Vec<Exercises>, ApiError>> =
        LocalResource::new(api::get_exercises);

    view! {
        <BackLink href="/templates" label="Templates" />
        {move || when_loaded(
            exercises.get(),
            |exercises: Vec<Exercises>| view! {
                <TemplateEditor id=None name=String::new() rows=Vec::new() exercises=exercises />
            },
        )}
    }
}

/// What the template page loads: its id, the template, and every exercise
/// (for the "add set" picker).
type TemplatePage = (i32, NamedFullTemplate, Vec<Exercises>);

#[component]
pub fn TemplateEdit() -> impl IntoView {
    let route_id: Memo<Option<i32>> = id_param();
    let data: LocalResource<Result<TemplatePage, ApiError>> =
        LocalResource::new(move || load_template(route_id.get()));

    view! {
        <BackLink href="/templates" label="Templates" />
        {move || when_loaded(
            data.get(),
            |(id, template, exercises): TemplatePage| {
                let rows: Vec<TemplateRow> = template
                    .exercises
                    .into_iter()
                    .map(|named| TemplateRow::new(named.template_exercise.e_id, named.exercise_name))
                    .collect();
                view! {
                    <TemplateEditor
                        id=Some(id)
                        name=template.template.name
                        rows=rows
                        exercises=exercises
                    />
                }
            },
        )}
    }
}

async fn load_template(id: Option<i32>) -> Result<TemplatePage, ApiError> {
    let id: i32 = id.ok_or(ApiError::Status(404))?;
    // Both requests at once; the first error wins.
    let (template, exercises): (NamedFullTemplate, Vec<Exercises>) =
        try_join!(api::get_full_template(id), api::get_exercises())?;
    Ok((id, template, exercises))
}

/// One template row = one set when the workout starts. Plain data (no
/// signals): rows are only added, removed, and reordered, never typed into.
#[derive(Clone)]
struct TemplateRow {
    key: usize,
    e_id: i32,
    exercise_name: String,
}

impl TemplateRow {
    fn new(e_id: i32, exercise_name: String) -> Self {
        TemplateRow {
            key: next_key(),
            e_id,
            exercise_name,
        }
    }
}

/// Create (`id` is `None`, POST) or edit (`id` is `Some`, PUT, which replaces
/// the whole template).
#[component]
fn TemplateEditor(
    id: Option<i32>,
    name: String,
    rows: Vec<TemplateRow>,
    exercises: Vec<Exercises>,
) -> impl IntoView {
    let navigate = use_navigate();
    let name: RwSignal<String> = RwSignal::new(name);
    let rows: RwSignal<Vec<TemplateRow>> = RwSignal::new(rows);
    let exercises: StoredValue<Vec<Exercises>> = StoredValue::new(exercises);
    let selected: RwSignal<String> = RwSignal::new(String::new());
    let error: RwSignal<Option<String>> = RwSignal::new(None);
    let busy: RwSignal<bool> = RwSignal::new(false);
    // Read out by screen readers after a row moves or goes.
    let announcement: RwSignal<String> = RwSignal::new(String::new());

    let add = move |_| {
        let e_id: Option<i32> = selected.get_untracked().parse().ok();
        let row: Option<TemplateRow> = e_id.and_then(|e_id| {
            exercises.with_value(|list| {
                list.iter()
                    .find(|exercise| exercise.id == Some(e_id))
                    .map(|exercise| TemplateRow::new(e_id, exercise.name.clone()))
            })
        });
        if let Some(row) = row {
            rows.update(|list| list.push(row));
        }
    };

    let save = {
        let navigate = navigate.clone();
        move |ev: SubmitEvent| {
            ev.prevent_default();
            let trimmed: String = name.get_untracked().trim().to_string();
            if trimmed.is_empty() {
                error.set(Some(String::from("Give the template a name.")));
                return;
            }
            let template: FullTemplate = FullTemplate {
                template: Templates {
                    id: None,
                    name: trimmed,
                },
                exercises: rows.with_untracked(|list| {
                    list.iter()
                        .enumerate()
                        .map(|(index, row)| TemplateExercises {
                            id: None,
                            t_id: None,
                            e_id: row.e_id,
                            ex_order: (index + 1) as i16,
                        })
                        .collect()
                }),
            };
            error.set(None);
            busy.set(true);
            let navigate = navigate.clone();
            spawn_local(async move {
                let result: Result<(), ApiError> = match id {
                    Some(id) => api::put_template(id, &template).await,
                    None => api::post_template(&template).await,
                };
                match result {
                    Ok(()) => navigate("/templates", replace_page()),
                    Err(failure) => {
                        error.set(Some(match failure {
                            ApiError::Status(400) => {
                                String::from("One of these exercises no longer exists.")
                            }
                            ApiError::Status(404) => String::from("This template was deleted."),
                            other => other.to_string(),
                        }));
                        busy.set(false);
                    }
                }
            });
        }
    };

    // Start and Delete only make sense for a saved template.
    let saved_actions = id.map(|id| {
        let delete = move |_| {
            if !confirm("Delete this template?") {
                return;
            }
            busy.set(true);
            let navigate = navigate.clone();
            spawn_local(async move {
                match api::delete_template(id).await {
                    Ok(()) | Err(ApiError::Status(404)) => navigate("/templates", replace_page()),
                    Err(failure) => {
                        error.set(Some(failure.to_string()));
                        busy.set(false);
                    }
                }
            });
        };
        view! {
            <div class="flex items-center justify-between border-t border-stone-200 pt-6 dark:border-stone-800">
                <a href=format!("/workouts/new?template={id}") class=BUTTON_QUIET>"Start workout"</a>
                <button
                    type="button"
                    class=BUTTON_DANGER
                    disabled=move || busy.get()
                    on:click=delete
                >
                    "Delete"
                </button>
            </div>
        }
    });

    view! {
        // Any typing clears the last error.
        <form class="space-y-8" on:submit=save on:input=move |_| error.set(None)>
            <h1>
                <input
                    class=TITLE_INPUT
                    placeholder="Template name"
                    aria-label="Template name"
                    maxlength="20"
                    required=true
                    bind:value=name
                />
            </h1>

            <section>
                <h2 class=SECTION_TITLE>"Sets, in order"</h2>
                <Show
                    when=move || rows.with(|list| !list.is_empty())
                    fallback=|| view! { <p class=MUTED>"No sets yet. Add one row per set."</p> }
                >
                    <ol class=CARD role="list">
                        <For
                            each=move || rows.get()
                            key=|row: &TemplateRow| row.key
                            children=move |row: TemplateRow| {
                                let key: usize = row.key;
                                // Buttons say which exercise they act on.
                                let name: String = row.exercise_name;
                                let up_label: String = format!("Move {name} up");
                                let down_label: String = format!("Move {name} down");
                                let remove_label: String = format!("Remove {name}");
                                let removed: String = format!("{name} removed");
                                view! {
                                    <li class="flex items-center gap-1 py-1.5 pr-2 pl-4">
                                        <span class="w-6 shrink-0 text-sm text-stone-500 tabular-nums dark:text-stone-400">
                                            {move || rows.with(|list| {
                                                list.iter().position(|other| other.key == key).map(|index| index + 1)
                                            })}
                                        </span>
                                        <span class="min-w-0 flex-1 truncate">{name}</span>
                                        <button
                                            type="button"
                                            class=ICON_BUTTON
                                            aria-label=up_label
                                            on:click:target=move |ev| {
                                                move_row(rows, announcement, key, Direction::Up, ev.target().into())
                                            }
                                        >
                                            "↑"
                                        </button>
                                        <button
                                            type="button"
                                            class=ICON_BUTTON
                                            aria-label=down_label
                                            on:click:target=move |ev| {
                                                move_row(rows, announcement, key, Direction::Down, ev.target().into())
                                            }
                                        >
                                            "↓"
                                        </button>
                                        <button
                                            type="button"
                                            class=ICON_BUTTON
                                            aria-label=remove_label
                                            on:click:target=move |ev| {
                                                focus_after_remove(&ev.target());
                                                rows.update(|list| list.retain(|other| other.key != key));
                                                announce(announcement, removed.clone());
                                            }
                                        >
                                            "✕"
                                        </button>
                                    </li>
                                }
                            }
                        />
                    </ol>
                </Show>
                <p role="status" class="sr-only">{move || announcement.get()}</p>
                <div class="mt-3 flex gap-2">
                    <ExerciseSelect exercises=exercises.get_value() selected=selected />
                    <button
                        type="button"
                        class=BUTTON_QUIET
                        disabled=move || selected.with(|id| id.is_empty())
                        on:click=add
                    >
                        "Add set"
                    </button>
                </div>
            </section>

            <div class="space-y-3">
                <ErrorText message=error />
                <button type="submit" class=BUTTON disabled=move || busy.get()>
                    "Save template"
                </button>
            </div>

            {saved_actions}
        </form>
    }
}

enum Direction {
    Up,
    Down,
}

/// Swaps the row with its neighbour and returns its new position, counting
/// from 1. Does nothing at either end.
fn shift(rows: RwSignal<Vec<TemplateRow>>, key: usize, direction: Direction) -> Option<usize> {
    rows.try_update(|list| {
        let index: usize = list.iter().position(|row| row.key == key)?;
        let target: usize = match direction {
            Direction::Up => index.checked_sub(1)?,
            Direction::Down => Some(index + 1).filter(|next| *next < list.len())?,
        };
        list.swap(index, target);
        Some(target + 1)
    })
    .flatten()
}

/// A ↑/↓ tap. Moving re-inserts the row's element, which drops focus, so
/// focus goes back to the tapped button, and the new position is announced.
fn move_row(
    rows: RwSignal<Vec<TemplateRow>>,
    announcement: RwSignal<String>,
    key: usize,
    direction: Direction,
    button: HtmlElement,
) {
    let Some(position) = shift(rows, key, direction) else {
        return;
    };
    let (name, total): (String, usize) = rows.with_untracked(|list| {
        let name: String = list
            .iter()
            .find(|row| row.key == key)
            .map(|row| row.exercise_name.clone())
            .unwrap_or_default();
        (name, list.len())
    });
    request_animation_frame(move || {
        button.focus().ok();
    });
    announce(
        announcement,
        format!("{name} moved to {position} of {total}"),
    );
}
