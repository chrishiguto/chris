use leptos::prelude::*;

/// External contact link: no house underline — the arrow nudges outward on
/// hover instead, and parks under reduced motion. Wrapped by [`Contacts`].
#[component]
fn ContactLink(href: &'static str, label: &'static str) -> impl IntoView {
    view! {
        <a
            href=href
            class="group inline-flex items-baseline gap-1.5 bg-none text-sm font-medium text-ink-2"
        >
            {label}
            <span
                class="inline-block transition-transform duration-200 ease-out-expo motion-safe:group-hover:translate-x-[2px] motion-safe:group-hover:-translate-y-[2px]"
                aria-hidden="true"
            >
                "↗"
            </span>
        </a>
    }
}

/// The external contact cluster shared by the home masthead and the about
/// page: the email line over the github + linkedin links. Hrefs are
/// well-formed mocks until real handles exist — kept here so they live in one
/// place. `lead` spaces the email line for its context.
#[component]
pub(crate) fn Contacts(lead: &'static str) -> impl IntoView {
    view! {
        <p class=lead>
            <a href="mailto:hi@chris.dev" class="text-sm font-medium">
                "hi@chris.dev"
            </a>
        </p>
        <div class="mt-3 flex gap-6">
            <ContactLink href="https://github.com/chris" label="github" />
            <ContactLink href="https://www.linkedin.com/in/chris" label="linkedin" />
        </div>
    }
}
