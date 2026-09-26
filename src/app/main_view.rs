use super::*;

impl AbstractApp {
    pub(crate) fn render_main(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = cx.palette();
        let has_note = self.current.is_some();
        let status: SharedString = self.notice.clone().unwrap_or_else(|| match self.save {
            SaveState::Pending => t(Key::Saving).into(),
            SaveState::Failed => t(Key::SaveFailed).into(),
            SaveState::Saved => tf(Key::Words, &[("n", &self.words.to_string())]).into(),
        });
        let theme_tip = SharedString::from(tf(
            Key::Theme,
            &[(
                "name",
                match self.theme_pref {
                    ThemePref::System => t(Key::ThemeSystem),
                    ThemePref::Light => t(Key::ThemeLight),
                    ThemePref::Dark => t(Key::ThemeDark),
                },
            )],
        ));

        let toolbar = titlebar_drag(div().id("toolbar"))
            .h(px(48.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .pl(px(chrome_left_pad(!self.sidebar_open)))
            .pr(px(9.))
            .child(
                self.ring(
                    5,
                    icon_btn(
                        "toggle-sidebar",
                        "icons/sidebar.svg",
                        tf(Key::Sidebar, &[]).into(),
                        !self.sidebar_open,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_sidebar(cx))),
                    &pal,
                )
                .when_some(
                    self.mark(5, Anchor::TopLeft, point(px(0.), px(38.)), cx),
                    |s, m| s.child(m),
                ),
            )
            .child(
                icon_btn(
                    "search",
                    "icons/search.svg",
                    tf(Key::Search, &[]).into(),
                    false,
                )
                .on_click(cx.listener(|this, _, window, cx| this.open_search(window, cx))),
            )
            .child(div().flex_1())
            .child(rise(
                self.ring(
                    3,
                    div()
                        .id("status")
                        .w(px(140.))
                        .text_right()
                        .px(px(8.))
                        .text_size(px(12.))
                        .text_color(rgb(if self.save == SaveState::Failed {
                            pal.fg
                        } else {
                            pal.faint
                        }))
                        .child(status),
                    &pal,
                )
                .when_some(
                    self.mark(3, Anchor::TopRight, point(px(140.), px(34.)), cx),
                    |s, m| s.child(m),
                ),
                ("status", self.save as usize),
                220,
                0.,
                2.,
            ))
            .child(
                self.ring(
                    4,
                    icon_btn("theme", self.theme_pref.icon(), theme_tip, false)
                        .on_click(cx.listener(|this, _, window, cx| this.cycle_theme(window, cx))),
                    &pal,
                )
                .when_some(
                    self.mark(4, Anchor::TopRight, point(px(30.), px(38.)), cx),
                    |s, m| s.child(m),
                ),
            )
            .when(has_note, |bar| {
                bar.child(
                    icon_btn(
                        "delete",
                        "icons/delete.svg",
                        tf(Key::DeleteNote, &[]).into(),
                        false,
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.delete_note(window, cx))),
                )
            })
            .when(cfg!(not(target_os = "macos")), |t| {
                t.child(window_controls(window, &pal))
            });

        let body = if self.loading {
            div().flex_1().into_any_element()
        } else {
            div()
                .flex_1()
                .min_h_0()
                .flex()
                .child(
                    div()
                        .id("editor-col")
                        .relative()
                        .flex_1()
                        .min_w_0()
                        .min_h_0()
                        .h_full()
                        .flex()
                        .flex_col()
                        .child(div().flex_1().min_h_0().w_full().child(rise(
                            div().size_full().child(self.editor.clone()),
                            ("editor-in", self.open_gen),
                            420,
                            0.,
                            10.,
                        )))
                        .when_some(self.render_backlinks(cx), |s, m| s.child(m))
                        .when_some(
                            self.mark(2, Anchor::TopLeft, point(px(70.), px(70.)), cx),
                            |s, m| s.child(m),
                        )
                        .when_some(self.render_completion(cx), |s, m| s.child(m)),
                )
                .into_any_element()
        };

        div()
            .relative()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(toolbar)
            .child(body)
            .when_some(self.search.as_ref(), |el, _| {
                el.child(self.render_search(cx))
            })
    }
}
