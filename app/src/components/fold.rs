use leptos::prelude::*;

/// One-way progressive disclosure, shared by the home's career fold and the
/// post `Hidden` component. The folded content ships in full and is visible
/// until [`FOLD_SCRIPT`] marks the fold ready (`is-ready`): only then does
/// CSS clip it like `sr-only` — still in the accessibility tree, out of
/// sight — and unhide the button that opens it (`is-open`). No JavaScript,
/// no fold: the reader simply sees everything. `label` is the button's
/// spoken name.
#[component]
pub fn Fold(label: &'static str, children: Children) -> impl IntoView {
    view! {
        <div class="fold">
            <button class="fold-button" type="button" aria-expanded="false" hidden>
                <span aria-hidden="true">"(…)"</span>
                <span class="sr-only">{label}</span>
            </button>
            <div class="fold-content">{children()}</div>
        </div>
    }
}

/// The one fold script, rendered once per page by the app shell: readies
/// every fold (taking its clipped content out of the tab order), opens a
/// fold on its button, and keeps keyboard focus inside the revealed content
/// — the first focusable there, else the content itself. Not an island: the
/// fold is progressive disclosure over server HTML.
pub const FOLD_SCRIPT: &str = r#"
const foldFocusables = (fold) => fold.querySelectorAll('.fold-content a[href], .fold-content button, .fold-content [tabindex]');
document.querySelectorAll('.fold').forEach((fold) => {
  fold.classList.add('is-ready');
  fold.querySelector('.fold-button').hidden = false;
  foldFocusables(fold).forEach((el) => {
    el.dataset.foldTabindex = el.getAttribute('tabindex') ?? '';
    el.tabIndex = -1;
  });
});
document.addEventListener('click', (event) => {
  const button = event.target.closest('.fold-button');
  if (!button) return;
  const fold = button.closest('.fold');
  fold.classList.add('is-open');
  button.setAttribute('aria-expanded', 'true');
  const els = foldFocusables(fold);
  els.forEach((el) => {
    if (el.dataset.foldTabindex === '') el.removeAttribute('tabindex');
    else el.setAttribute('tabindex', el.dataset.foldTabindex);
    delete el.dataset.foldTabindex;
  });
  let target = els[0];
  if (!target) {
    target = fold.querySelector('.fold-content');
    target.tabIndex = -1;
  }
  target.focus();
});
"#;
