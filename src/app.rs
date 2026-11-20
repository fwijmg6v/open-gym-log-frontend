use leptos::html;
use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::path;

use crate::pages::{
    ExerciseDetail, ExerciseList, Home, TemplateEdit, TemplateList, TemplateNew, WorkoutDetail,
    WorkoutList, WorkoutNew,
};
use crate::ui::MUTED;

#[component]
pub fn App() -> impl IntoView {
    let main: NodeRef<html::Main> = NodeRef::new();

    view! {
        <Router>
            // tabindex="-1": focusable from code only (see FocusOnNavigate).
            <main
                node_ref=main
                tabindex="-1"
                class="mx-auto min-h-dvh max-w-lg px-4 pt-8 pb-28 focus:outline-hidden"
            >
                // Static segments ("new") go before ":id" so they win.
                <Routes fallback=|| view! { <p class=MUTED>"Page not found."</p> }>
                    <Route path=path!("/") view=Home />
                    <Route path=path!("/workouts") view=WorkoutList />
                    <Route path=path!("/workouts/new") view=WorkoutNew />
                    <Route path=path!("/workouts/:id") view=WorkoutDetail />
                    <Route path=path!("/templates") view=TemplateList />
                    <Route path=path!("/templates/new") view=TemplateNew />
                    <Route path=path!("/templates/:id") view=TemplateEdit />
                    <Route path=path!("/exercises") view=ExerciseList />
                    <Route path=path!("/exercises/:id") view=ExerciseDetail />
                </Routes>
            </main>
            <Nav />
            <FocusOnNavigate main=main />
        </Router>
    }
}

/// After an in-app page change, focus moves to the new page's content, as it
/// would on a full page load. Otherwise it stays on a link that's gone, and
/// screen readers lose their place. The first load is left alone.
#[component]
fn FocusOnNavigate(main: NodeRef<html::Main>) -> impl IntoView {
    let pathname: Memo<String> = use_location().pathname;
    Effect::new(move |previous: Option<String>| {
        let path: String = pathname.get();
        // `previous` is None on the first run, which skips the first load.
        if let Some(main) = previous.and(main.get()) {
            main.focus().ok();
        }
        path
    });
}

const TAB: &str = "py-3 text-center text-sm font-medium text-stone-500 transition-colors hover:text-stone-700 aria-[current=page]:font-semibold aria-[current=page]:text-stone-900 dark:text-stone-400 dark:hover:text-stone-200 dark:aria-[current=page]:text-stone-100";

/// Bottom tab bar, thumb-reachable on a phone. The active tab gets
/// `aria-current="page"`, which the `aria-[current=page]:` classes style
/// (bolder as well as darker, so it's not marked by colour alone).
#[component]
fn Nav() -> impl IntoView {
    let pathname: Memo<String> = use_location().pathname;
    let tab = move |href: &'static str, is_current: fn(&str) -> bool, label: &'static str| {
        let current = move || pathname.with(|path| is_current(path)).then_some("page");
        view! { <a href=href aria-current=current class=TAB>{label}</a> }
    };

    view! {
        <nav class="fixed inset-x-0 bottom-0 border-t border-stone-200 bg-stone-50/90 backdrop-blur dark:border-stone-800 dark:bg-stone-950/90">
            <div class="mx-auto grid max-w-lg grid-cols-4 pb-[env(safe-area-inset-bottom)]">
                // The workout being logged belongs to Home, where it was started.
                {tab("/", |path| path == "/" || path == "/workouts/new", "Home")}
                {tab("/workouts", |path| path.starts_with("/workouts") && path != "/workouts/new", "Workouts")}
                {tab("/templates", |path| path.starts_with("/templates"), "Templates")}
                {tab("/exercises", |path| path.starts_with("/exercises"), "Exercises")}
            </div>
        </nav>
    }
}
