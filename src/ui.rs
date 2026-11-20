//! Shared Tailwind class strings, small components, and formatting helpers.

use std::sync::atomic::{AtomicUsize, Ordering};

use leptos::prelude::*;
use leptos::wasm_bindgen::JsCast;
use leptos_router::NavigateOptions;
use leptos_router::hooks::use_params_map;
use leptos_router::params::ParamsMap;
use web_sys::{Element, HtmlElement};

use crate::api::ApiError;
use crate::models::{Exercises, Sets, WeightUnit};

// Class strings live in Rust constants. Tailwind scans `src/` (see
// style/main.css), so every class must appear here in full, never glued
// together at runtime.

// Grey text is stone-500 in light mode and stone-400 in dark mode (placeholders
// too): the lightest greys that still pass WCAG AA contrast on these backgrounds.
// Field borders are stone-500 in both modes, so fields stand out at 3:1 or more.
// `user-invalid:` turns a field red once the user has left it invalid.

pub const INPUT: &str = "w-full rounded-lg border border-stone-500 bg-white px-3 py-2 text-base placeholder:text-stone-500 focus:border-stone-900 focus:outline-hidden user-invalid:border-red-600 dark:bg-stone-900 dark:placeholder:text-stone-400 dark:focus:border-stone-100 dark:user-invalid:border-red-400";
/// Big input that doubles as the page title (put it in an `<h1>`). It's
/// underlined while focused.
pub const TITLE_INPUT: &str = "w-full border-b-2 border-transparent bg-transparent text-2xl font-semibold tracking-tight placeholder:text-stone-500 focus:border-stone-500 focus:outline-hidden user-invalid:border-red-600 dark:placeholder:text-stone-400 dark:focus:border-stone-400 dark:user-invalid:border-red-400";
/// The set row's three number inputs share its spare width (`flex-1`);
/// `min-w-0` lets them shrink below their content on narrow phones.
pub const NUMBER_INPUT: &str = "w-0 min-w-0 flex-1 rounded-lg border border-stone-500 bg-white px-2 py-1.5 text-center text-base tabular-nums placeholder:text-stone-500 focus:border-stone-900 focus:outline-hidden user-invalid:border-red-600 dark:bg-stone-900 dark:placeholder:text-stone-400 dark:focus:border-stone-100 dark:user-invalid:border-red-400";
pub const BUTTON: &str = "rounded-lg bg-stone-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-stone-700 disabled:opacity-50 dark:bg-stone-100 dark:text-stone-900 dark:hover:bg-stone-300";
pub const BUTTON_QUIET: &str = "shrink-0 rounded-lg border border-stone-300 px-4 py-2 text-sm font-medium transition-colors hover:bg-stone-100 disabled:opacity-50 dark:border-stone-700 dark:hover:bg-stone-800";
pub const BUTTON_DANGER: &str = "rounded-lg px-4 py-2 text-sm font-medium text-red-600 transition-colors hover:bg-red-50 disabled:opacity-50 dark:text-red-400 dark:hover:bg-red-950";
/// 44px square: the minimum comfortable touch target.
pub const ICON_BUTTON: &str = "grid size-11 shrink-0 place-items-center rounded-lg text-stone-500 transition-colors hover:bg-stone-100 hover:text-stone-700 dark:text-stone-400 dark:hover:bg-stone-800 dark:hover:text-stone-200";
pub const CHIP: &str = "inline-flex min-h-11 shrink-0 items-center rounded-full border border-stone-300 px-3 text-sm transition-colors hover:bg-stone-100 dark:border-stone-700 dark:hover:bg-stone-800";
/// For `<ul>`/`<ol>` lists. Give them `role="list"`: Tailwind removes the
/// bullets, and Safari then stops announcing them as lists.
pub const CARD: &str = "divide-y divide-stone-200 overflow-hidden rounded-xl border border-stone-200 bg-white dark:divide-stone-800 dark:border-stone-800 dark:bg-stone-900";
/// `-outline-offset-2` draws the focus ring inside, where CARD can't clip it.
pub const ROW_LINK: &str = "flex items-center justify-between gap-4 px-4 py-3 -outline-offset-2 transition-colors hover:bg-stone-50 dark:hover:bg-stone-800/60";
pub const SECTION_TITLE: &str =
    "mb-2 text-xs font-medium tracking-wide text-stone-500 uppercase dark:text-stone-400";
pub const MUTED: &str = "text-sm text-stone-500 dark:text-stone-400";
/// MUTED, but only shown after 300 ms (`appear` is in main.css), so fast
/// loads don't flash it.
const LOADING: &str = "text-sm text-stone-500 dark:text-stone-400 animate-[appear_0s_300ms_both]";
pub const ERROR: &str = "text-sm text-red-600 dark:text-red-400";

#[component]
pub fn Header(
    #[prop(into)] title: String,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <header class="mb-6 flex items-center justify-between gap-4">
            <h1 class="truncate text-2xl font-semibold tracking-tight">{title}</h1>
            {children.map(|children| children())}
        </header>
    }
}

#[component]
pub fn BackLink(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        // py-3 makes a 44px tap target; -mt-3 and mb-1 keep the layout unchanged.
        <a
            href=href
            class="-mt-3 mb-1 inline-block py-3 text-sm text-stone-500 transition-colors hover:text-stone-900 dark:text-stone-400 dark:hover:text-stone-100"
        >
            // The arrow is decoration; screen readers just say the label.
            <span aria-hidden="true">"← "</span>
            {label}
        </a>
    }
}

/// Always in the page, empty when there's no error, so screen readers
/// announce a message the moment it appears.
#[component]
pub fn ErrorText(message: RwSignal<Option<String>>) -> impl IntoView {
    view! { <p role="alert" class=ERROR>{move || message.get()}</p> }
}

/// A `<select>` of exercises that writes the chosen id (as a string) into
/// `selected`. An empty string means nothing is chosen.
#[component]
pub fn ExerciseSelect(exercises: Vec<Exercises>, selected: RwSignal<String>) -> impl IntoView {
    if exercises.is_empty() {
        return view! {
            <p class=MUTED>
                "No exercises yet. "
                <a href="/exercises" class="underline">"Add some first."</a>
            </p>
        }
        .into_any();
    }

    view! {
        <select
            class=INPUT
            aria-label="Exercise"
            prop:value=move || selected.get()
            on:change:target=move |ev| selected.set(ev.target().value())
        >
            <option value="" disabled=true selected=true>"Choose an exercise…"</option>
            {exercises
                .into_iter()
                .filter_map(|exercise| {
                    let id: i32 = exercise.id?;
                    Some(view! { <option value=id.to_string()>{exercise.name}</option> })
                })
                .collect_view()}
        </select>
    }
    .into_any()
}

/// Renders a resource: "Loading…" until it arrives, the error if it failed,
/// otherwise whatever `render` builds from the value.
pub fn when_loaded<T, V>(data: Option<Result<T, ApiError>>, render: impl FnOnce(T) -> V) -> AnyView
where
    V: IntoView + 'static,
{
    match data {
        None => view! { <p class=LOADING>"Loading…"</p> }.into_any(),
        Some(Err(error)) => view! { <p class=ERROR>{error.to_string()}</p> }.into_any(),
        Some(Ok(value)) => render(value).into_any(),
    }
}

/// The `:id` route segment as a number (`None` if it isn't one).
pub fn id_param() -> Memo<Option<i32>> {
    let params: Memo<ParamsMap> = use_params_map();
    Memo::new(move |_| {
        params
            .with(|map| map.get("id"))
            .and_then(|raw| raw.parse().ok())
    })
}

/// Unique keys for rows in editable lists, so `<For>` can track them while
/// they're added, removed, and reordered.
pub fn next_key() -> usize {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    NEXT.fetch_add(1, Ordering::Relaxed)
}

/// Groups neighbouring items that share a key: [a, a, b, a] -> [(a, 2), (b, 1), (a, 1)].
/// The API already returns rows in order, so neighbours are all that matter.
pub fn group_consecutive<T, K: PartialEq>(
    items: Vec<T>,
    key: impl Fn(&T) -> K,
) -> Vec<(K, Vec<T>)> {
    let mut groups: Vec<(K, Vec<T>)> = Vec::new();
    for item in items {
        let item_key: K = key(&item);
        match groups.last_mut() {
            Some((last_key, group)) if *last_key == item_key => group.push(item),
            _ => groups.push((item_key, vec![item])),
        }
    }
    groups
}

/// A logged set as a list item: "102.5 kg × 5 @ RIR 2", with its note
/// underneath if it has one.
#[component]
pub fn SetLine(set: Sets) -> impl IntoView {
    let line: String = describe_set(&set);
    view! {
        <li>
            {line}
            {set.notes.map(|notes| view! {
                <p class="text-xs whitespace-pre-line text-stone-500 dark:text-stone-400">{notes}</p>
            })}
        </li>
    }
}

/// "102.5 kg × 5 @ RIR 2"
pub fn describe_set(set: &Sets) -> String {
    let weight: &str = set.weight.as_deref().map(trim_weight).unwrap_or("?");
    let unit: &str = set.metric.map(WeightUnit::label).unwrap_or("");
    let reps: String = set
        .reps
        .map(|reps| reps.to_string())
        .unwrap_or_else(|| String::from("?"));
    match set.rir {
        Some(rir) => format!("{weight} {unit} × {reps} @ RIR {rir}"),
        None => format!("{weight} {unit} × {reps}"),
    }
}

/// "100.0" -> "100", "102.5" stays.
fn trim_weight(weight: &str) -> &str {
    weight.strip_suffix(".0").unwrap_or(weight)
}

/// Trimmed text, or `None` if blank (for optional notes fields).
pub fn non_empty(text: String) -> Option<String> {
    let trimmed: &str = text.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

/// Today on the phone's clock, as "YYYY-MM-DD".
pub fn today() -> String {
    format_date(&js_sys::Date::new_0())
}

/// A date in the phone's time zone as "YYYY-MM-DD", the API's format.
pub fn format_date(date: &js_sys::Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.get_full_year(),
        date.get_month() + 1,
        date.get_date()
    )
}

/// The browser's OK/Cancel dialog.
pub fn confirm(message: &str) -> bool {
    web_sys::window()
        .and_then(|window| window.confirm_with_message(message).ok())
        .unwrap_or(false)
}

/// For `navigate` after a save or delete: the finished form leaves the
/// history, so Back doesn't return to it (or to something that's gone).
pub fn replace_page() -> NavigateOptions {
    NavigateOptions {
        replace: true,
        ..Default::default()
    }
}

/// Puts `text` in a `role="status"` region on the next frame, once focus has
/// settled. Clearing it first means the same text twice is still read out.
pub fn announce(region: RwSignal<String>, text: String) {
    region.set(String::new());
    request_animation_frame(move || region.set(text));
}

/// Call just before removing the list row that holds `button` (a "Remove …"
/// button). Focus moves to the next row's Remove button, or the previous
/// row's, or the section's exercise picker, instead of dropping to the page.
pub fn focus_after_remove(button: &Element) {
    let row: Option<Element> = button.closest("li").ok().flatten();
    let neighbour: Option<Element> = row.and_then(|row| {
        row.next_element_sibling()
            .or(row.previous_element_sibling())
    });
    let target: Option<Element> = match neighbour {
        Some(other) => other
            .query_selector("button[aria-label^=Remove]")
            .ok()
            .flatten(),
        None => button
            .closest("section")
            .ok()
            .flatten()
            .and_then(|section| section.query_selector("select").ok().flatten()),
    };
    if let Some(element) = target.and_then(|target| target.dyn_into::<HtmlElement>().ok()) {
        element.focus().ok();
    }
}
