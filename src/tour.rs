//! First-run coach marks: a bubble anchored to each highlighted element.
//! Steps advance with buttons or Enter/→, go back with ←, and skip with Esc.

use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use crate::app::AbstractApp;
use crate::assets::rise;
use crate::i18n::{Key, t, tf};
use crate::theme::PaletteAccess;

actions!(abstract_tour, [TourNext, TourBack, TourSkip]);

pub fn bind_keys(cx: &mut App) {
    let c = Some("Tour");
    cx.bind_keys([
        KeyBinding::new("enter", TourNext, c),
        KeyBinding::new("right", TourNext, c),
        KeyBinding::new("left", TourBack, c),
        KeyBinding::new("escape", TourSkip, c),
    ]);
}

pub struct Step {
    pub title: Key,
    pub body: Key,
}

pub const STEPS: &[Step] = &[
    Step {
        title: Key::Tour1Title,
        body: Key::Tour1Body,
    },
    Step {
        title: Key::Tour2Title,
        body: Key::Tour2Body,
    },
    Step {
        title: Key::Tour3Title,
        body: Key::Tour3Body,
    },
    Step {
        title: Key::Tour4Title,
        body: Key::Tour4Body,
    },
    Step {
        title: Key::Tour5Title,
        body: Key::Tour5Body,
    },
    Step {
        title: Key::Tour6Title,
        body: Key::Tour6Body,
    },
];

fn btn(id: &'static str, label: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .h(px(26.))
        .px(px(10.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .text_size(px(12.))
}

/// The bubble for `step`, styled from the active palette. Rendered inside a
/// `deferred`/`anchored` wrapper by the anchor element.
pub fn bubble(step: usize, focus: FocusHandle, cx: &mut Context<AbstractApp>) -> impl IntoElement {
    let p = cx.palette();
    let s = &STEPS[step];
    let n = STEPS.len();
    let last = step + 1 == n;
    div()
        .id("tour-bubble")
        .key_context("Tour")
        .track_focus(&focus)
        .role(Role::Dialog)
        .aria_label(tf(
            Key::TourStepAria,
            &[
                ("step", &(step + 1).to_string()),
                ("n", &n.to_string()),
                ("title", t(s.title)),
            ],
        ))
        .w(px(280.))
        .bg(rgb(p.menu_bg))
        .border_1()
        .border_color(rgb(p.menu_border))
        .rounded(px(10.))
        .shadow_lg()
        .occlude()
        .on_action(cx.listener(|this, _: &TourNext, window, cx| this.tour_next(window, cx)))
        .on_action(cx.listener(|this, _: &TourBack, window, cx| this.tour_back(window, cx)))
        .on_action(cx.listener(|this, _: &TourSkip, window, cx| this.tour_skip(window, cx)))
        .p(px(14.))
        .flex()
        .flex_col()
        .gap(px(8.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(p.fg))
                .child(t(s.title)),
        )
        .child(
            div()
                .text_size(px(12.5))
                .line_height(px(18.))
                .text_color(rgb(p.dim))
                .child(t(s.body)),
        )
        .child(
            div()
                .flex()
                .items_center()
                .gap(px(4.))
                .pt(px(4.))
                .child(div().text_size(px(11.)).text_color(rgb(p.faint)).child(tf(
                    Key::TourStepOf,
                    &[("step", &(step + 1).to_string()), ("n", &n.to_string())],
                )))
                .child(div().flex_1())
                .child(
                    btn("tour-skip", t(Key::TourSkip))
                        .text_color(rgb(p.dim))
                        .hover(|s| s.bg(rgb(p.hover)))
                        .on_click(cx.listener(|this, _, window, cx| this.tour_skip(window, cx))),
                )
                .when(step > 0, |row| {
                    row.child(
                        btn("tour-back", t(Key::TourBack))
                            .text_color(rgb(p.dim))
                            .hover(|s| s.bg(rgb(p.hover)))
                            .on_click(
                                cx.listener(|this, _, window, cx| this.tour_back(window, cx)),
                            ),
                    )
                })
                .child(
                    btn(
                        "tour-next",
                        if last {
                            t(Key::TourDone)
                        } else {
                            t(Key::TourNext)
                        },
                    )
                    .bg(rgb(p.fg))
                    .text_color(rgb(p.bg))
                    .font_weight(FontWeight::MEDIUM)
                    .hover(|s| s.opacity(0.85))
                    .on_click(cx.listener(|this, _, window, cx| this.tour_next(window, cx))),
                ),
        )
}

/// Wrapped + animated bubble ready to hang off an anchor element.
pub fn mark(
    step: usize,
    anchor: Anchor,
    offset: Point<Pixels>,
    focus: FocusHandle,
    cx: &mut Context<AbstractApp>,
) -> impl IntoElement {
    deferred(
        anchored()
            .anchor(anchor)
            .offset(offset)
            .snap_to_window_with_margin(px(8.))
            .child(rise(
                div().child(bubble(step, focus, cx)),
                ("tour-step", step),
                220,
                0.,
                6.,
            )),
    )
    .with_priority(1)
}
