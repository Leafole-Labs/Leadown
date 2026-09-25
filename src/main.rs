mod editor;
mod md;
mod spaces;
mod store;
mod theme;
mod tour;
mod vault;

use std::borrow::Cow;
use std::collections::HashSet;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

use gpui_kit::component::Root;
use gpui_kit::component::input::{self, Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use editor::{Changed, LiveEditor};
use spaces::Spaces;
use store::{Session, SessionNote, SessionWindow, Settings};
use theme::{Palette, PaletteAccess, ThemePref};
use vault::NodeKind;

// ── Palette: pure monochrome, driven by `Palette` global ──────────────────
const SANS: &str = "Noto Sans";

const SIDEBAR_W: f32 = 248.;
const SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

/// Embedded Hugeicons (stroke-rounded, MIT). Anything else falls through to
/// the component library's default icon set.
const ICONS: [(&str, &[u8]); 12] = [
    (
        "icons/add.svg",
        include_bytes!("../assets/icons/add-01.svg"),
    ),
    (
        "icons/check.svg",
        include_bytes!("../assets/icons/tick-02.svg"),
    ),
    (
        "icons/chevrons.svg",
        include_bytes!("../assets/icons/unfold-more.svg"),
    ),
    (
        "icons/close.svg",
        include_bytes!("../assets/icons/cancel-01.svg"),
    ),
    (
        "icons/delete.svg",
        include_bytes!("../assets/icons/delete-02.svg"),
    ),
    (
        "icons/folder.svg",
        include_bytes!("../assets/icons/folder-01.svg"),
    ),
    (
        "icons/folder-add.svg",
        include_bytes!("../assets/icons/folder-add.svg"),
    ),
    (
        "icons/maximize.svg",
        include_bytes!("../assets/icons/square.svg"),
    ),
    (
        "icons/minimize.svg",
        include_bytes!("../assets/icons/minus-sign.svg"),
    ),
    (
        "icons/note.svg",
        include_bytes!("../assets/icons/note-01.svg"),
    ),
    (
        "icons/restore.svg",
        include_bytes!("../assets/icons/square-arrow-shrink-02.svg"),
    ),
    (
        "icons/sidebar.svg",
        include_bytes!("../assets/icons/view-sidebar-left.svg"),
    ),
];

/// Marks the synthetic sidebar row that hosts the new-folder input. Starts
/// with a dot so a real file can never collide.
const NEW_FOLDER_ROW: &str = ".abstract-new-folder";

struct AppAssets;

impl AssetSource for AppAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        match ICONS.iter().find(|(p, _)| *p == path) {
            Some((_, bytes)) => Ok(Some(Cow::Borrowed(bytes))),
            None => gpui_kit_assets::Assets.load(path),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut out = gpui_kit_assets::Assets.list(path)?;
        out.extend(
            ICONS
                .iter()
                .filter(|(p, _)| p.starts_with(path))
                .map(|(p, _)| (*p).into()),
        );
        Ok(out)
    }
}

fn icon(name: &'static str, color: u32) -> Svg {
    svg()
        .path(name)
        .size(px(16.))
        .flex_none()
        .text_color(rgb(color))
}

/// Quart-out fade + short travel. Keyed by `id`; a new id replays it.
/// `AnimationExt` honors the platform reduced-motion setting.
pub(crate) fn rise<E: Styled + IntoElement + 'static>(
    el: E,
    id: impl Into<ElementId>,
    ms: u64,
    delay: f32,
    travel: f32,
) -> impl IntoElement {
    el.with_animation(
        id,
        Animation::new(Duration::from_millis(ms)).with_easing(move |t| {
            let t = ((t - delay) / (1.0 - delay)).clamp(0.0, 1.0);
            1.0 - (1.0 - t).powi(4)
        }),
        move |el, d| el.opacity(d).mt(px(travel * (1.0 - d))),
    )
}

actions!(
    abstract_app,
    [
        NewNote,
        NewFolder,
        DeleteNote,
        RenameNote,
        SaveNow,
        CycleTheme,
        ToggleSidebar,
        OpenSpace,
        ToggleSpaces,
        StartTour,
    ]
);

// ── Notes on disk ─────────────────────────────────────────────────────────

fn title_of(text: &str) -> SharedString {
    text.lines()
        .map(|l| l.trim().trim_start_matches('#').trim())
        .find(|l| !l.is_empty())
        .map(|l| SharedString::from(l.chars().take(80).collect::<String>()))
        .unwrap_or_else(|| SharedString::new_static("Sem título"))
}

fn stem_of(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// What the open note is on disk; shared with in-flight writes, which may
/// rename it.
struct NoteFile {
    path: PathBuf,
    /// Mtime after the last read/write we did; detects external edits.
    mtime: Option<SystemTime>,
    /// Trashed: in-flight writes must not recreate the file.
    deleted: bool,
}

struct CurrentNote {
    file: Arc<Mutex<NoteFile>>,
    /// Renames to the title's stem on each save (placeholder or title-derived
    /// name); a manual rename to anything else turns this off.
    synced: bool,
}

/// Inline rename / new-folder input in a sidebar row.
struct RenameEdit {
    state: Entity<InputState>,
    /// Path being renamed, or the parent dir when `create`.
    target: PathBuf,
    kind: NodeKind,
    create: bool,
    _sub: Subscription,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SaveState {
    Saved,
    Pending,
    Failed,
}

/// One serialized on-disk update: rename to the title stem when synced, then
/// write. Returns whether the visible tree changed (created or renamed).
/// Blocking; runs on the background executor under the app's write lock.
fn write_note(
    lock: &Arc<Mutex<()>>,
    file: &Arc<Mutex<NoteFile>>,
    synced: bool,
    text: &str,
) -> std::io::Result<bool> {
    let _write = lock.lock().unwrap();
    let mut f = file.lock().unwrap();
    if f.deleted {
        return Ok(false);
    }
    let from = f.path.clone();
    let mut target = from.clone();
    if synced {
        let dir = from
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        target = vault::unique_path(&dir, &vault::stem_for_title(&title_of(text)), Some(&from));
    }
    let existed = from.exists();
    if target != from {
        if !existed || std::fs::rename(&from, &target).is_ok() {
            f.path = target.clone();
        } else {
            // Rename failed: still save under the old name.
            target = from.clone();
        }
    }
    store::write_atomic(&target, text.as_bytes())?;
    f.mtime = std::fs::metadata(&target).and_then(|m| m.modified()).ok();
    Ok(target != from || !existed)
}

struct AbstractApp {
    spaces: Spaces,
    spaces_open: bool,
    dir: PathBuf,
    tree: Vec<vault::Node>,
    expanded: HashSet<PathBuf>,
    current: Option<CurrentNote>,
    /// Planned path of a note not yet written (first keystroke creates it).
    pending_new: Option<PathBuf>,
    /// Last clicked folder; target dir for new notes and folders.
    target_folder: Option<PathBuf>,
    editing: Option<RenameEdit>,
    /// Transient status message, cleared on the next successful save/action.
    notice: Option<SharedString>,
    theme_pref: ThemePref,
    settings: Settings,
    session: Session,
    session_notes: Vec<SessionNote>,
    window_state: Option<SessionWindow>,
    tour_step: Option<usize>,
    tour_focus: FocusHandle,
    tour_shown: bool,
    loading: bool,
    editor: Entity<LiveEditor>,
    sidebar_open: bool,
    /// Bumped per toggle; keys the sidebar slide so it replays.
    sidebar_gen: usize,
    /// Bumped per opened note; keys the editor fade-in.
    open_gen: usize,
    save: SaveState,
    words: usize,
    /// Serializes on-disk ops on the open note (rename + write + trash mark).
    write_lock: Arc<Mutex<()>>,
    _save_task: Option<Task<()>>,
    _io_task: Option<Task<()>>,
    _bounds_task: Option<Task<()>>,
    _subs: Vec<Subscription>,
}

impl AbstractApp {
    fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        settings: Settings,
        session: Session,
    ) -> Self {
        let editor = cx.new(LiveEditor::new);
        let on_change = cx.subscribe(&editor, |this: &mut Self, editor, _: &Changed, cx| {
            let text = editor.read(cx).text();
            this.words = text.split_whitespace().count();
            this.schedule_save(cx);
            cx.notify();
        });
        let on_quit = cx.on_app_quit(|this, cx| {
            this.flush_blocking(cx);
            async {}
        });
        let on_bounds = cx.observe_window_bounds(window, |this, window, cx| {
            this.bounds_changed(window, cx);
        });
        let on_activation = cx.observe_window_activation(window, |this, window, cx| {
            this.activation_changed(window, cx);
        });
        let on_appearance = cx.observe_window_appearance(window, |this, window, cx| {
            this.appearance_changed(window, cx);
        });

        let mut app = Self {
            spaces: Spaces {
                paths: vec![PathBuf::new()],
                active: 0,
            },
            spaces_open: false,
            dir: PathBuf::new(),
            tree: Vec::new(),
            expanded: HashSet::new(),
            current: None,
            pending_new: None,
            target_folder: None,
            editing: None,
            notice: None,
            theme_pref: settings.theme(),
            settings,
            session_notes: session.notes(),
            session,
            window_state: None,
            tour_step: None,
            tour_focus: cx.focus_handle(),
            tour_shown: false,
            loading: true,
            editor,
            sidebar_open: true,
            sidebar_gen: 0,
            open_gen: 0,
            save: SaveState::Saved,
            words: 0,
            write_lock: Arc::new(Mutex::new(())),
            _save_task: None,
            _io_task: None,
            _bounds_task: None,
            _subs: vec![on_change, on_quit, on_bounds, on_activation, on_appearance],
        };
        app.sidebar_open = app.session.sidebar_open().unwrap_or(true);
        app._io_task = Some(cx.spawn_in(window, async move |this, cx| {
            let spaces = cx
                .background_executor()
                .spawn(async { spaces::load() })
                .await;
            this.update_in(cx, |this, window, cx| this.enter_space(spaces, window, cx))
                .ok();
        }));
        app
    }

    fn current_text(&self, cx: &App) -> String {
        self.editor.read(cx).text().to_string()
    }

    /// Debounced background write of the open note.
    fn schedule_save(&mut self, cx: &mut Context<Self>) {
        let Some(cur) = &self.current else { return };
        let file = cur.file.clone();
        let synced = cur.synced;
        let lock = self.write_lock.clone();
        self.save = SaveState::Pending;
        self._save_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SAVE_DEBOUNCE).await;
            // Path and text are captured now, not when the timer was set.
            let Ok(text) = this.update(cx, |this, cx| this.current_text(cx)) else {
                return;
            };
            let result = cx
                .background_executor()
                .spawn(async move { write_note(&lock, &file, synced, &text) })
                .await;
            this.update(cx, |this, cx| this.finish_save(result, cx))
                .ok();
        }));
    }

    /// Ctrl+S: write now instead of waiting for the debounce.
    fn save_now(&mut self, cx: &mut Context<Self>) {
        let Some(cur) = &self.current else { return };
        self._save_task = None;
        let file = cur.file.clone();
        let synced = cur.synced;
        let lock = self.write_lock.clone();
        let text = self.current_text(cx);
        self.save = SaveState::Pending;
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_executor()
                .spawn(async move { write_note(&lock, &file, synced, &text) })
                .await;
            this.update(cx, |this, cx| this.finish_save(result, cx))
                .ok();
        })
        .detach();
    }

    fn finish_save(&mut self, result: std::io::Result<bool>, cx: &mut Context<Self>) {
        match result {
            Ok(changed) => {
                self.save = SaveState::Saved;
                self.notice = None;
                if changed {
                    self.rescan_tree(cx);
                }
            }
            Err(err) => {
                eprintln!("abstract: failed to save note: {err}");
                self.save = SaveState::Failed;
            }
        }
        cx.notify();
    }

    /// Hand a pending save to the background before the buffer is replaced.
    fn flush(&mut self, cx: &mut Context<Self>) {
        self._save_task = None;
        if self.save != SaveState::Pending {
            return;
        }
        self.save = SaveState::Saved;
        if let Some(cur) = &self.current {
            let file = cur.file.clone();
            let synced = cur.synced;
            let lock = self.write_lock.clone();
            let text = self.current_text(cx);
            let write = cx.background_spawn(async move { write_note(&lock, &file, synced, &text) });
            cx.spawn(async move |this, cx| {
                let result = write.await;
                this.update(cx, |this, cx| this.finish_save(result, cx))
                    .ok();
            })
            .detach();
        }
    }

    /// On quit: write the pending save inline (the write lock waits for any
    /// in-flight op), then the session — note entry included.
    fn flush_blocking(&mut self, cx: &mut Context<Self>) {
        self._save_task = None;
        if self.save == SaveState::Pending
            && let Some(cur) = &self.current
        {
            let lock = self.write_lock.clone();
            let file = cur.file.clone();
            let _ = write_note(&lock, &file, cur.synced, &self.current_text(cx));
            self.save = SaveState::Saved;
        }
        self.record_session_note(cx);
        self.session.set_sidebar(self.sidebar_open);
        if let Some(w) = self.window_state {
            self.session.set_window(&w);
        }
        self.session.set_notes(&self.session_notes);
        self.session.save();
    }

    /// Remember where the open note was left (space + rel path + view).
    fn record_session_note(&mut self, cx: &App) {
        let Some(cur) = &self.current else { return };
        let path = cur.file.lock().unwrap().path.clone();
        let Ok(rel) = path.strip_prefix(&self.dir) else {
            return;
        };
        if rel.as_os_str().is_empty() {
            return;
        }
        let (cursor, scroll) = self.editor.read(cx).view_state();
        self.session_notes
            .retain(|n| !(n.space == self.dir && n.rel == rel));
        self.session_notes.push(SessionNote {
            space: self.dir.clone(),
            rel: rel.to_path_buf(),
            cursor,
            scroll,
        });
    }

    fn save_session(&self, cx: &mut Context<Self>) {
        let mut session = self.session.clone();
        session.set_sidebar(self.sidebar_open);
        if let Some(w) = self.window_state {
            session.set_window(&w);
        }
        session.set_notes(&self.session_notes);
        cx.background_spawn(async move { session.save() }).detach();
    }

    /// Debounced (500ms) bounds persistence; the final write is on quit.
    fn bounds_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.window_state = Some(session_window(window));
        self._bounds_task = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(500))
                .await;
            this.update(cx, |this, cx| this.save_session(cx)).ok();
        }));
    }

    /// Returning to the window: rescan the tree and stat the open file.
    fn activation_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !window.is_window_active() {
            return;
        }
        self.rescan_tree(cx);
        let Some(cur) = &self.current else { return };
        let file = cur.file.clone();
        cx.spawn(async move |this, cx| {
            let (path, recorded, deleted) = {
                let f = file.lock().unwrap();
                (f.path.clone(), f.mtime, f.deleted)
            };
            if deleted {
                return;
            }
            let mtime = cx
                .background_executor()
                .spawn({
                    let p = path.clone();
                    async move { std::fs::metadata(&p).and_then(|m| m.modified()).ok() }
                })
                .await;
            this.update(cx, |this, cx| {
                if this
                    .current
                    .as_ref()
                    .is_none_or(|c| !Arc::ptr_eq(&c.file, &file))
                {
                    return;
                }
                if this.current.as_ref().unwrap().file.lock().unwrap().path != path {
                    return;
                }
                match mtime {
                    // Vanished: keep the buffer; the next edit recreates it.
                    None if recorded.is_some() => {
                        file.lock().unwrap().mtime = None;
                        this.notice = Some("Arquivo removido fora do app".into());
                        cx.notify();
                    }
                    Some(m) if recorded != Some(m) => {
                        // A pending save wins; it will record the new mtime.
                        if this.save == SaveState::Pending {
                            return;
                        }
                        this.reload_file(file.clone(), path.clone(), cx);
                    }
                    _ => {}
                }
            })
            .ok();
        })
        .detach();
    }

    /// The file changed on disk and nothing is pending: reload, keeping the
    /// cursor (clamped) and scroll.
    fn reload_file(&mut self, file: Arc<Mutex<NoteFile>>, path: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let read = cx
                .background_executor()
                .spawn({
                    let p = path.clone();
                    async move {
                        (
                            std::fs::read_to_string(&p).unwrap_or_default(),
                            std::fs::metadata(&p).and_then(|m| m.modified()).ok(),
                        )
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if this
                    .current
                    .as_ref()
                    .is_none_or(|c| !Arc::ptr_eq(&c.file, &file))
                {
                    return;
                }
                {
                    let mut f = file.lock().unwrap();
                    if f.path != path || f.deleted {
                        return;
                    }
                    f.mtime = read.1;
                }
                let (cursor, scroll) = this.editor.read(cx).view_state();
                let synced = vault::synced_stem(&stem_of(&path), &title_of(&read.0));
                if let Some(cur) = &mut this.current {
                    cur.synced = synced;
                }
                this.words = read.0.split_whitespace().count();
                this.editor.update(cx, |ed, cx| {
                    ed.set_text(read.0, cx);
                    ed.restore_view(cursor, scroll, cx);
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn appearance_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.theme_pref == ThemePref::System {
            theme::apply(ThemePref::System, window.appearance(), cx);
            cx.notify();
        }
    }

    /// Re-read the whole folder tree off-thread.
    fn rescan_tree(&mut self, cx: &mut Context<Self>) {
        let dir = self.dir.clone();
        cx.spawn(async move |this, cx| {
            let scanned = cx
                .background_executor()
                .spawn(async move { vault::scan(&dir) })
                .await;
            this.update(cx, |this, cx| {
                match scanned {
                    Ok(tree) => this.tree = tree,
                    Err(err) => eprintln!("abstract: cannot read space: {err}"),
                }
                if this.pending_new.as_ref().is_some_and(|p| p.exists()) {
                    this.pending_new = None;
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Expand every folder between the space root and `path`'s parent.
    fn expand_to(&mut self, path: &Path) {
        let mut dir = path.parent();
        while let Some(d) = dir {
            if d == self.dir || !d.starts_with(&self.dir) {
                break;
            }
            self.expanded.insert(d.to_path_buf());
            dir = d.parent();
        }
    }

    fn load_buffer(
        &mut self,
        file: NoteFile,
        synced: bool,
        text: String,
        restore: Option<(usize, f32)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.current = Some(CurrentNote {
            file: Arc::new(Mutex::new(file)),
            synced,
        });
        self.open_gen += 1;
        self.save = SaveState::Saved;
        self.words = text.split_whitespace().count();
        // `set_text` emits no `Changed`, so nothing is re-saved.
        self.editor.update(cx, |ed, cx| {
            ed.set_text(text, cx);
            if let Some((cursor, scroll)) = restore {
                ed.restore_view(cursor, scroll, cx);
            }
            ed.focus(window, cx);
        });
        cx.notify();
    }

    fn open_path(
        &mut self,
        path: PathBuf,
        restore: Option<(usize, f32)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .current
            .as_ref()
            .is_some_and(|c| c.file.lock().unwrap().path == path)
        {
            return;
        }
        self.record_session_note(cx);
        self.save_session(cx);
        self.flush(cx);
        self.pending_new = None;
        self.editing = None;
        self.target_folder = None;
        self.expand_to(&path);
        self._io_task = Some(cx.spawn_in(window, async move |this, cx| {
            let p = path.clone();
            let read = cx
                .background_executor()
                .spawn(async move {
                    (
                        std::fs::read_to_string(&p).unwrap_or_default(),
                        std::fs::metadata(&p).and_then(|m| m.modified()).ok(),
                    )
                })
                .await;
            this.update_in(cx, |this, window, cx| {
                let synced = vault::synced_stem(&stem_of(&path), &title_of(&read.0));
                this.load_buffer(
                    NoteFile {
                        path,
                        mtime: read.1,
                        deleted: false,
                    },
                    synced,
                    read.0,
                    restore,
                    window,
                    cx,
                );
            })
            .ok();
        }));
    }

    /// The folder new notes/folders go into: last clicked folder, else the
    /// open note's parent, else the space root.
    fn target_dir(&self) -> PathBuf {
        self.target_folder
            .clone()
            .or_else(|| {
                self.current
                    .as_ref()
                    .and_then(|c| c.file.lock().unwrap().path.parent().map(Path::to_path_buf))
            })
            .unwrap_or_else(|| self.dir.clone())
    }

    /// The file is created on the first keystroke, so untouched notes leave no trace.
    fn new_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.record_session_note(cx);
        self.save_session(cx);
        self.flush(cx);
        self._io_task = None;
        let dir = self.target_dir();
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_or(0, |d| d.as_millis());
        let path = vault::unique_path(&dir, &format!("nota-{stamp}"), None);
        self.expand_to(&path);
        self.pending_new = Some(path.clone());
        self.target_folder = None;
        self.load_buffer(
            NoteFile {
                path,
                mtime: None,
                deleted: false,
            },
            true,
            String::new(),
            None,
            window,
            cx,
        );
    }

    /// Send a path to the trash off-thread; failure only shows a notice.
    fn trash_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let ok = cx
                .background_executor()
                .spawn({
                    let p = path.clone();
                    async move {
                        trash::delete(&p)
                            .map_err(|e| eprintln!("abstract: cannot trash {}: {e}", p.display()))
                            .is_ok()
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if !ok {
                    this.notice = Some("Não foi possível mover para a Lixeira".into());
                }
                this.rescan_tree(cx);
            })
            .ok();
        })
        .detach();
    }

    fn delete_note(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cur) = self.current.take() else {
            return;
        };
        let path = cur.file.lock().unwrap().path.clone();
        self._save_task = None;
        self._io_task = None;
        self.pending_new = None;
        let pending = self.save == SaveState::Pending;
        self.save = SaveState::Saved;
        let file = cur.file.clone();
        let synced = cur.synced;
        let lock = self.write_lock.clone();
        let text = self.current_text(cx);
        cx.spawn(async move |this, cx| {
            let gone = cx
                .background_executor()
                .spawn(async move {
                    if pending {
                        let _ = write_note(&lock, &file, synced, &text);
                    }
                    let mut f = file.lock().unwrap();
                    f.deleted = true;
                    let p = f.path.clone();
                    p.exists().then_some(p)
                })
                .await;
            this.update(cx, |this, cx| match gone {
                Some(p) => this.trash_path(p, cx),
                None => this.rescan_tree(cx),
            })
            .ok();
        })
        .detach();
        // Open the next newest note, or a fresh one.
        match vault::newest_note(&self.tree).filter(|p| *p != path) {
            Some(next) => self.open_path(next, None, window, cx),
            None => self.new_note(window, cx),
        }
        cx.notify();
    }

    /// Row delete: notes go straight to the trash, folders ask first.
    fn delete_row(
        &mut self,
        path: PathBuf,
        kind: NodeKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match kind {
            NodeKind::Note => {
                let is_current = self
                    .current
                    .as_ref()
                    .is_some_and(|c| c.file.lock().unwrap().path == path);
                if is_current {
                    self.delete_note(window, cx);
                } else {
                    self.trash_path(path, cx);
                }
            }
            NodeKind::Folder => self.delete_folder(path, window, cx),
        }
    }

    fn delete_folder(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let prompt = window.prompt(
            PromptLevel::Warning,
            &format!("Mover a pasta “{name}” para a Lixeira?"),
            Some("As notas dentro dela também vão."),
            &["Mover para a Lixeira", "Cancelar"],
            cx,
        );
        cx.spawn_in(window, async move |this, cx| {
            if prompt.await != Ok(0) {
                return;
            }
            this.update_in(cx, |this, _, cx| {
                this.expanded.retain(|p| !p.starts_with(&path));
                if this
                    .target_folder
                    .as_ref()
                    .is_some_and(|t| t.starts_with(&path))
                {
                    this.target_folder = None;
                }
                // The open note inside it keeps its buffer as "removed".
                if let Some(cur) = &this.current
                    && cur.file.lock().unwrap().path.starts_with(&path)
                {
                    cur.file.lock().unwrap().mtime = None;
                    this.notice = Some("Arquivo removido fora do app".into());
                }
                this.trash_path(path.clone(), cx);
            })
            .ok();
        })
        .detach();
    }

    // ── Inline rename / create ────────────────────────────────────────────

    fn start_edit(
        &mut self,
        target: PathBuf,
        kind: NodeKind,
        create: bool,
        value: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
        let sub = cx.subscribe(&state, |this: &mut Self, _, ev: &InputEvent, cx| match ev {
            InputEvent::PressEnter { .. } => this.commit_edit(cx),
            InputEvent::Blur => this.cancel_edit(cx),
            _ => {}
        });
        self.editing = Some(RenameEdit {
            state: state.clone(),
            target,
            kind,
            create,
            _sub: sub,
        });
        state.update(cx, |s, cx| {
            s.focus(window, cx);
            s.select_all(window, cx);
        });
        cx.notify();
    }

    fn start_rename(
        &mut self,
        path: PathBuf,
        kind: NodeKind,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.editing.is_some() {
            return;
        }
        let name = match kind {
            NodeKind::Folder => path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            NodeKind::Note => stem_of(&path),
        };
        self.start_edit(path, kind, false, name, window, cx);
    }

    /// F2 on the open note.
    fn rename_current(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(cur) = &self.current else { return };
        let path = cur.file.lock().unwrap().path.clone();
        self.start_rename(path, NodeKind::Note, window, cx);
    }

    fn new_folder(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.editing.is_some() {
            return;
        }
        let dir = self.target_dir();
        self.expand_to(&dir.join("x"));
        if dir != self.dir {
            self.expanded.insert(dir.clone());
        }
        self.start_edit(
            dir,
            NodeKind::Folder,
            true,
            "Nova pasta".to_string(),
            window,
            cx,
        );
    }

    fn focus_edit(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.editing.as_ref().map(|e| e.state.clone()) else {
            return;
        };
        if let Some(w) = cx.windows().first().copied() {
            w.update(cx, |_, window, cx| {
                state.update(cx, |s, cx| {
                    s.focus(window, cx);
                    s.select_all(window, cx);
                });
            })
            .ok();
        }
    }

    fn focus_editor(&mut self, cx: &mut Context<Self>) {
        let editor = self.editor.clone();
        if let Some(w) = cx.windows().first().copied() {
            w.update(cx, |_, window, cx| {
                editor.update(cx, |ed, cx| ed.focus(window, cx))
            })
            .ok();
        }
    }

    /// Keep the input open and complain.
    fn edit_conflict(&mut self, ed: RenameEdit, cx: &mut Context<Self>) {
        self.notice = Some("Já existe um item com esse nome".into());
        self.editing = Some(ed);
        self.focus_edit(cx);
        cx.notify();
    }

    fn commit_edit(&mut self, cx: &mut Context<Self>) {
        let Some(ed) = self.editing.take() else {
            return;
        };
        let raw = ed.state.read(cx).value().to_string();
        if raw.trim().is_empty() {
            self.focus_editor(cx);
            cx.notify();
            return;
        }
        let name = vault::stem_for_title(raw.trim());
        if ed.create {
            let target = ed.target.join(&name);
            if target.exists() {
                self.edit_conflict(ed, cx);
                return;
            }
            self.expanded.insert(ed.target.clone());
            cx.spawn(async move |this, cx| {
                let result = cx
                    .background_executor()
                    .spawn(async move { std::fs::create_dir(&target) })
                    .await;
                this.update(cx, |this, cx| {
                    if let Err(err) = result {
                        eprintln!("abstract: failed to create folder: {err}");
                    }
                    this.rescan_tree(cx);
                })
                .ok();
            })
            .detach();
            self.focus_editor(cx);
            cx.notify();
            return;
        }

        let Some(parent) = ed.target.parent().map(Path::to_path_buf) else {
            return;
        };
        let newp = match ed.kind {
            NodeKind::Folder => parent.join(&name),
            NodeKind::Note => parent.join(format!("{name}.md")),
        };
        if newp == ed.target {
            self.focus_editor(cx);
            cx.notify();
            return;
        }
        if newp.exists() {
            self.edit_conflict(ed, cx);
            return;
        }
        match ed.kind {
            NodeKind::Folder => self.rename_folder(ed.target.clone(), newp, cx),
            NodeKind::Note => self.rename_note_file(ed.target.clone(), newp, cx),
        }
        self.focus_editor(cx);
        cx.notify();
    }

    fn cancel_edit(&mut self, cx: &mut Context<Self>) {
        if self.editing.take().is_some() {
            self.focus_editor(cx);
            cx.notify();
        }
    }

    /// Update every path the app tracks after `old` moves to `new`.
    fn remap_prefix(&mut self, old: &Path, new: &Path, cx: &mut Context<Self>) {
        let remap = |p: &Path| -> PathBuf {
            p.strip_prefix(old)
                .map_or_else(|_| p.to_path_buf(), |rest| new.join(rest))
        };
        self.expanded = self.expanded.iter().map(|p| remap(p)).collect();
        if let Some(t) = &self.target_folder {
            self.target_folder = Some(remap(t));
        }
        if let Some(p) = &self.pending_new {
            self.pending_new = Some(remap(p));
        }
        for n in &mut self.session_notes {
            if n.space == self.dir
                && let Ok(rest) = n
                    .rel
                    .strip_prefix(old.strip_prefix(&self.dir).unwrap_or(old))
            {
                n.rel = new.strip_prefix(&self.dir).unwrap_or(new).join(rest);
            }
        }
        self.save_session(cx);
    }

    fn rename_folder(&mut self, old: PathBuf, new: PathBuf, cx: &mut Context<Self>) {
        let lock = self.write_lock.clone();
        // If the open note lives inside, its path moves with the folder.
        let inside = self
            .current
            .as_ref()
            .filter(|c| c.file.lock().unwrap().path.starts_with(&old))
            .map(|c| c.file.clone());
        cx.spawn(async move |this, cx| {
            let ok = cx
                .background_executor()
                .spawn({
                    let old = old.clone();
                    let new = new.clone();
                    async move {
                        let _w = lock.lock().unwrap();
                        match std::fs::rename(&old, &new) {
                            Ok(()) => {
                                if let Some(f) = inside {
                                    let mut f = f.lock().unwrap();
                                    if let Ok(rest) = f.path.strip_prefix(&old) {
                                        f.path = new.join(rest);
                                    }
                                }
                                true
                            }
                            Err(err) => {
                                eprintln!("abstract: failed to rename folder: {err}");
                                false
                            }
                        }
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if ok {
                    this.remap_prefix(&old, &new, cx);
                } else {
                    this.notice = Some("Não foi possível renomear".into());
                }
                this.rescan_tree(cx);
            })
            .ok();
        })
        .detach();
    }

    fn rename_note_file(&mut self, old: PathBuf, new: PathBuf, cx: &mut Context<Self>) {
        let lock = self.write_lock.clone();
        let current = self
            .current
            .as_ref()
            .filter(|c| c.file.lock().unwrap().path == old)
            .map(|c| c.file.clone());
        cx.spawn(async move |this, cx| {
            let ok = cx
                .background_executor()
                .spawn({
                    let old = old.clone();
                    let new = new.clone();
                    let current = current.clone();
                    async move {
                        let _w = lock.lock().unwrap();
                        if let Some(f) = &current {
                            // Pending new note: the file does not exist yet, just
                            // update the planned path.
                            let mut f = f.lock().unwrap();
                            if !old.exists() || std::fs::rename(&old, &new).is_ok() {
                                f.path = new.clone();
                                true
                            } else {
                                false
                            }
                        } else {
                            std::fs::rename(&old, &new).is_ok()
                        }
                    }
                })
                .await;
            this.update(cx, |this, cx| {
                if ok {
                    // Manual rename recomputes synced against the buffer.
                    if let (Some(cur), Some(f)) = (this.current.as_mut(), current)
                        && Arc::ptr_eq(&cur.file, &f)
                    {
                        let title = title_of(this.editor.read(cx).text());
                        cur.synced = vault::synced_stem(&stem_of(&new), &title);
                    }
                    this.remap_prefix(&old, &new, cx);
                } else {
                    eprintln!("abstract: failed to rename {}", old.display());
                    this.notice = Some("Não foi possível renomear".into());
                }
                this.rescan_tree(cx);
            })
            .ok();
        })
        .detach();
    }

    // ── Tour ──────────────────────────────────────────────────────────────

    fn start_tour(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.tour_step.is_some() {
            return;
        }
        self.tour_step = Some(0);
        self.tour_shown = true;
        if !self.sidebar_open {
            self.sidebar_open = true;
            self.sidebar_gen += 1;
        }
        self.tour_focus.focus(window, cx);
        cx.notify();
    }

    fn tour_next(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        match self.tour_step {
            Some(step) if step + 1 < tour::STEPS.len() => {
                self.tour_step = Some(step + 1);
                self.tour_focus.focus(window, cx);
            }
            _ => self.finish_tour(window, cx),
        }
        cx.notify();
    }

    fn tour_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(step) = self.tour_step {
            self.tour_step = Some(step.saturating_sub(1));
            self.tour_focus.focus(window, cx);
            cx.notify();
        }
    }

    fn tour_skip(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.finish_tour(window, cx);
    }

    fn finish_tour(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.tour_step = None;
        self.settings.set_tour_done();
        let settings = self.settings.clone();
        cx.background_spawn(async move { settings.save() }).detach();
        self.editor.update(cx, |ed, cx| ed.focus(window, cx));
        cx.notify();
    }

    /// Coach mark + highlight ring for `step`, to hang on an anchor element.
    fn mark(
        &self,
        step: usize,
        anchor: Anchor,
        offset: Point<Pixels>,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        (self.tour_step == Some(step)).then(|| {
            tour::mark(step, anchor, offset, self.tour_focus.clone(), cx).into_any_element()
        })
    }

    fn ring(&self, step: usize, el: Stateful<Div>, pal: &Palette) -> Stateful<Div> {
        el.when(self.tour_step == Some(step), |el| {
            el.shadow(vec![gpui_base::box_shadow(
                px(0.),
                px(0.),
                px(0.),
                px(2.),
                rgb(pal.fg).into(),
            )])
        })
    }

    // ── Spaces & window chrome ────────────────────────────────────────────

    fn toggle_sidebar(&mut self, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        self.sidebar_gen += 1;
        self.save_session(cx);
        cx.notify();
    }

    fn cycle_theme(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.theme_pref = self.theme_pref.next();
        theme::apply(self.theme_pref, window.appearance(), cx);
        self.settings.set_theme(self.theme_pref);
        let settings = self.settings.clone();
        cx.background_spawn(async move { settings.save() }).detach();
        cx.notify();
    }

    /// Switch to `spaces.current()`: flush the open note, persist the list,
    /// scan the folder off-thread, then reopen where the session left off.
    fn enter_space(&mut self, spaces: Spaces, window: &mut Window, cx: &mut Context<Self>) {
        self.record_session_note(cx);
        self.save_session(cx);
        self.flush(cx);
        self._save_task = None;
        self.editing = None;
        self.pending_new = None;
        self.target_folder = None;
        self.spaces = spaces;
        self.spaces_open = false;
        self.dir = self.spaces.current().to_path_buf();
        self.tree.clear();
        self.expanded.clear();
        self.current = None;
        self.loading = true;
        let dir = self.dir.clone();
        let snapshot = self.spaces.clone();
        self._io_task = Some(cx.spawn_in(window, async move |this, cx| {
            let scanned = cx
                .background_executor()
                .spawn(async move {
                    spaces::save(&snapshot);
                    vault::scan(&dir)
                })
                .await;
            this.update_in(cx, |this, window, cx| {
                this.loading = false;
                match scanned {
                    Ok(tree) => this.tree = tree,
                    Err(err) => eprintln!("abstract: cannot read space: {err}"),
                }
                // Session's last note for this space, else the newest.
                let restore = this
                    .session_notes
                    .iter()
                    .find(|n| n.space == this.dir)
                    .map(|n| (this.dir.join(&n.rel), n.cursor, n.scroll));
                match restore {
                    Some((p, cursor, scroll)) if vault::contains(&this.tree, &p) => {
                        this.open_path(p, Some((cursor, scroll)), window, cx)
                    }
                    _ => match vault::newest_note(&this.tree) {
                        Some(p) => this.open_path(p, None, window, cx),
                        None => this.new_note(window, cx),
                    },
                }
                if !this.tour_shown && !this.settings.tour_done() {
                    this.start_tour(window, cx);
                }
            })
            .ok();
        }));
        cx.notify();
    }

    fn switch_space(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        if ix == self.spaces.active {
            self.spaces_open = false;
            cx.notify();
            return;
        }
        let mut spaces = self.spaces.clone();
        spaces.active = ix;
        self.enter_space(spaces, window, cx);
    }

    fn remove_space(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) {
        let mut spaces = self.spaces.clone();
        let was_active = ix == spaces.active;
        if !spaces.remove(ix) {
            return;
        }
        if was_active {
            self.enter_space(spaces, window, cx);
        } else {
            self.spaces = spaces;
            let snapshot = self.spaces.clone();
            cx.background_spawn(async move { spaces::save(&snapshot) })
                .detach();
            cx.notify();
        }
    }

    /// Native folder picker; the chosen folder becomes (or reactivates) a space.
    fn open_space(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.spaces_open = false;
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Abrir como espaço".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            this.update_in(cx, |this, window, cx| {
                let mut spaces = this.spaces.clone();
                spaces.activate_path(path);
                this.enter_space(spaces, window, cx);
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    fn toggle_spaces(&mut self, cx: &mut Context<Self>) {
        self.spaces_open = !self.spaces_open;
        cx.notify();
    }

    fn toggle_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path.clone());
        }
        self.target_folder = Some(path);
        cx.notify();
    }

    /// Flattened visible rows plus the synthetic pending-note/new-folder rows.
    fn flat_rows(&self) -> Vec<vault::Row> {
        let mut rows = vault::flatten(&self.tree, &self.expanded);
        let insert = |rows: &mut Vec<vault::Row>, parent: &Path| -> usize {
            let (ix, depth) = match rows.iter().position(|r| r.path == parent) {
                Some(i) => (i + 1, rows[i].depth + 1),
                None => (0, 0),
            };
            rows.insert(
                ix,
                vault::Row {
                    path: PathBuf::new(),
                    name: String::new(),
                    kind: NodeKind::Note,
                    depth,
                    expanded: false,
                },
            );
            ix
        };
        if let Some(ed) = &self.editing
            && ed.create
        {
            let ix = insert(&mut rows, &ed.target);
            rows[ix].path = ed.target.join(NEW_FOLDER_ROW);
            rows[ix].kind = NodeKind::Folder;
        }
        if let Some(p) = &self.pending_new
            && !rows.iter().any(|r| r.path == *p)
            && let Some(parent) = p.parent()
        {
            let ix = insert(&mut rows, parent);
            rows[ix].path = p.clone();
            rows[ix].name = "Sem título".into();
        }
        rows
    }

    fn note_count(nodes: &[vault::Node]) -> usize {
        nodes
            .iter()
            .map(|n| {
                if n.is_folder() {
                    Self::note_count(&n.children)
                } else {
                    1
                }
            })
            .sum()
    }

    fn render_row(
        &self,
        ix: usize,
        row: &vault::Row,
        current: Option<&Path>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let pal = cx.palette();
        let active = current == Some(row.path.as_path());
        let folder = row.kind == NodeKind::Folder;
        let editing = self.editing.as_ref().is_some_and(|e| {
            (!e.create && e.target == row.path) || (e.create && row.path.ends_with(NEW_FOLDER_ROW))
        });
        let path = row.path.clone();
        let kind = row.kind;

        let mut pill = div()
            .id(("row", ix))
            .group("note-row")
            .role(Role::Button)
            .aria_label(row.name.clone())
            .h(px(30.))
            .pl(px(10. + row.depth as f32 * 14.))
            .pr(px(4.))
            .flex()
            .items_center()
            .gap(px(6.))
            .rounded(px(6.))
            .cursor_pointer()
            .text_size(px(13.))
            .line_height(px(18.))
            .text_color(rgb(if active { pal.fg } else { pal.dim }))
            .when(active, |s| s.bg(rgb(pal.active)))
            .when(!active, |s| {
                s.hover(|s| s.bg(rgb(pal.hover)).text_color(rgb(pal.body)))
            })
            .active(|s| s.bg(rgb(pal.active)));
        if !editing {
            pill = pill.on_click(cx.listener({
                let path = path.clone();
                move |this, _, window, cx| match kind {
                    NodeKind::Folder => this.toggle_folder(path.clone(), cx),
                    NodeKind::Note => this.open_path(path.clone(), None, window, cx),
                }
            }));
        }
        if folder {
            pill = pill
                .child(
                    icon(
                        if row.expanded {
                            "icons/chevron-down.svg"
                        } else {
                            "icons/chevron-right.svg"
                        },
                        pal.faint,
                    )
                    .size(px(12.)),
                )
                .child(icon("icons/folder.svg", pal.faint).size(px(15.)));
        } else {
            pill = pill.child(
                icon("icons/note.svg", if active { pal.fg } else { pal.faint })
                    .size(px(15.))
                    .ml(px(18.)),
            );
        }
        if editing && let Some(ed) = &self.editing {
            let state = ed.state.clone();
            pill = pill.child(
                div()
                    .flex_1()
                    .min_w_0()
                    .on_action(cx.listener(|this, _: &input::Escape, _, cx| this.cancel_edit(cx)))
                    .child(
                        Input::new(&state)
                            .appearance(false)
                            .bordered(false)
                            .h(px(24.))
                            .w_full()
                            .text_size(px(13.))
                            .text_color(rgb(pal.fg)),
                    ),
            );
        } else {
            pill = pill.child(div().flex_1().min_w_0().truncate().child(row.name.clone()));
            // Hover actions: rename, delete.
            pill = pill
                .child(
                    div()
                        .id(("row-rename", ix))
                        .role(Role::Button)
                        .aria_label("Renomear")
                        .size(px(20.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .invisible()
                        .group_hover("note-row", |s| s.visible())
                        .hover(|s| s.bg(rgb(pal.active)))
                        .on_click(cx.listener({
                            let path = path.clone();
                            move |this, _, window, cx| {
                                cx.stop_propagation();
                                this.start_rename(path.clone(), kind, window, cx);
                            }
                        }))
                        .child(icon("icons/pencil.svg", pal.dim).size(px(12.))),
                )
                .child(
                    div()
                        .id(("row-delete", ix))
                        .role(Role::Button)
                        .aria_label("Mover para a Lixeira")
                        .size(px(20.))
                        .flex_none()
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded(px(4.))
                        .invisible()
                        .group_hover("note-row", |s| s.visible())
                        .hover(|s| s.bg(rgb(pal.active)))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.delete_row(path.clone(), kind, window, cx);
                        }))
                        .child(icon("icons/delete.svg", pal.dim).size(px(12.))),
                );
        }
        pill
    }

    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self.flat_rows();
        let count = rows.len();
        let list = uniform_list(
            "notes",
            count,
            cx.processor(move |this, range: Range<usize>, _window, cx| {
                let rows = this.flat_rows();
                let current = this
                    .current
                    .as_ref()
                    .map(|c| c.file.lock().unwrap().path.clone());
                range
                    .map(|ix| {
                        let row = &rows[ix.min(rows.len().saturating_sub(1))];
                        let pill = this.render_row(ix, row, current.as_deref(), cx);
                        div().h(px(32.)).px(px(8.)).pb(px(2.)).child(rise(
                            pill,
                            ("note-in", ix),
                            360,
                            (ix.min(12) as f32) * 0.05,
                            4.,
                        ))
                    })
                    .collect()
            }),
        )
        .flex_1()
        .min_h_0();

        let (from, to) = if self.sidebar_open {
            (0., SIDEBAR_W)
        } else {
            (SIDEBAR_W, 0.)
        };
        let pal = cx.palette();
        let space_name = SharedString::from(spaces::name_of(&self.dir));
        let notes_n = Self::note_count(&self.tree);
        let panel = div()
            .id("sidebar")
            .flex_none()
            .h_full()
            .overflow_hidden()
            .bg(rgb(pal.panel))
            .border_r_1()
            .border_color(rgb(pal.line))
            .child(
                div()
                    .relative()
                    .w(px(SIDEBAR_W))
                    .h_full()
                    .flex()
                    .flex_col()
                    // 48px header = toolbar height; empty area drags the window.
                    .child(
                        titlebar_drag(div().id("sidebar-head"))
                            .h(px(48.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .gap(px(4.))
                            .pl(px(9.))
                            .pr(px(9.))
                            .child(
                                self.ring(
                                    0,
                                    div()
                                        .id("space-switcher")
                                        .role(Role::Button)
                                        .aria_label("Trocar de espaço (Ctrl+O)")
                                        .flex_1()
                                        .min_w_0()
                                        .h(px(30.))
                                        .px(px(8.))
                                        .flex()
                                        .items_center()
                                        .gap(px(8.))
                                        .rounded(px(6.))
                                        .cursor_pointer()
                                        .when(self.spaces_open, |s| s.bg(rgb(pal.active)))
                                        .hover(|s| s.bg(rgb(pal.hover)))
                                        .active(|s| s.bg(rgb(pal.active)))
                                        .on_mouse_down(MouseButton::Left, |_, _, cx| {
                                            cx.stop_propagation()
                                        })
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.toggle_spaces(cx)),
                                        )
                                        .child(icon("icons/folder.svg", pal.fg).size(px(15.)))
                                        .child(
                                            div()
                                                .flex_1()
                                                .min_w_0()
                                                .truncate()
                                                .text_size(px(13.))
                                                .font_weight(FontWeight::SEMIBOLD)
                                                .text_color(rgb(pal.fg))
                                                .child(space_name),
                                        )
                                        .child(icon("icons/chevrons.svg", pal.faint).size(px(14.)))
                                        .when_some(
                                            self.mark(
                                                0,
                                                Anchor::TopLeft,
                                                point(px(0.), px(38.)),
                                                cx,
                                            ),
                                            |s, m| s.child(m),
                                        ),
                                    &pal,
                                ),
                            )
                            .child(
                                self.ring(
                                    1,
                                    icon_btn(
                                        "new-folder",
                                        "icons/folder-add.svg",
                                        "Nova pasta".into(),
                                        false,
                                    )
                                    .on_click(cx.listener(
                                        |this, _, window, cx| this.new_folder(window, cx),
                                    )),
                                    &pal,
                                )
                                .when_some(
                                    self.mark(1, Anchor::TopRight, point(px(30.), px(38.)), cx),
                                    |s, m| s.child(m),
                                ),
                            )
                            .child(
                                icon_btn(
                                    "new",
                                    "icons/add.svg",
                                    "Nova nota (Ctrl+N)".into(),
                                    false,
                                )
                                .on_click(
                                    cx.listener(|this, _, window, cx| this.new_note(window, cx)),
                                ),
                            ),
                    )
                    .child(
                        div()
                            .h(px(28.))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px(px(18.))
                            .text_size(px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgb(pal.faint))
                            .child("NOTAS")
                            .child(notes_n.to_string()),
                    )
                    .child(list)
                    .when(self.spaces_open, |col| {
                        col.child(self.render_spaces_menu(cx))
                    }),
            );
        if self.sidebar_gen == 0 {
            return panel.w(px(to)).into_any_element();
        }
        panel
            .with_animation(
                ("sidebar-slide", self.sidebar_gen),
                Animation::new(Duration::from_millis(280)).with_easing(ease_out_quint),
                move |el, d| el.w(px(from + (to - from) * d)),
            )
            .into_any_element()
    }

    fn render_spaces_menu(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = cx.palette();
        let removable = self.spaces.paths.len() > 1;
        let mut rows = div().flex().flex_col().gap(px(2.)).p(px(4.));
        for (ix, path) in self.spaces.paths.iter().enumerate() {
            let active = ix == self.spaces.active;
            rows = rows.child(
                div()
                    .id(("space", ix))
                    .group("space-row")
                    .role(Role::MenuItem)
                    .aria_label(SharedString::from(spaces::name_of(path)))
                    .h(px(40.))
                    .px(px(8.))
                    .flex()
                    .items_center()
                    .gap(px(8.))
                    .rounded(px(6.))
                    .cursor_pointer()
                    .hover(|s| s.bg(rgb(pal.hover)))
                    .active(|s| s.bg(rgb(pal.active)))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.switch_space(ix, window, cx)),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .flex()
                            .flex_col()
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(13.))
                                    .line_height(px(17.))
                                    .text_color(rgb(if active { pal.fg } else { pal.body }))
                                    .child(SharedString::from(spaces::name_of(path))),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(px(11.))
                                    .line_height(px(15.))
                                    .text_color(rgb(pal.faint))
                                    .child(SharedString::from(path.display().to_string())),
                            ),
                    )
                    .when(active, |row| {
                        row.child(icon("icons/check.svg", pal.fg).size(px(14.)))
                    })
                    .when(!active && removable, |row| {
                        row.child(
                            div()
                                .id(("space-remove", ix))
                                .role(Role::Button)
                                .aria_label("Remover da lista (os arquivos ficam)")
                                .size(px(22.))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(px(4.))
                                .invisible()
                                .group_hover("space-row", |s| s.visible())
                                .hover(|s| s.bg(rgb(pal.active)))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    cx.stop_propagation();
                                    this.remove_space(ix, window, cx);
                                }))
                                .child(icon("icons/close.svg", pal.dim).size(px(12.))),
                        )
                    }),
            );
        }
        let menu = div()
            .id("spaces-menu")
            .role(Role::Menu)
            .absolute()
            .top(px(44.))
            .left(px(8.))
            .w(px(SIDEBAR_W - 16.))
            .bg(rgb(pal.menu_bg))
            .border_1()
            .border_color(rgb(pal.menu_border))
            .rounded(px(8.))
            .shadow_lg()
            .occlude()
            .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                this.spaces_open = false;
                cx.notify();
            }))
            .child(
                div()
                    .px(px(12.))
                    .pt(px(10.))
                    .pb(px(2.))
                    .text_size(px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgb(pal.faint))
                    .child("ESPAÇOS"),
            )
            .child(rows)
            .child(div().h(px(1.)).bg(rgb(pal.line)))
            .child(
                div().p(px(4.)).child(
                    div()
                        .id("open-space")
                        .role(Role::MenuItem)
                        .h(px(32.))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .gap(px(8.))
                        .rounded(px(6.))
                        .cursor_pointer()
                        .text_size(px(13.))
                        .text_color(rgb(pal.body))
                        .hover(|s| s.bg(rgb(pal.hover)))
                        .active(|s| s.bg(rgb(pal.active)))
                        .on_click(cx.listener(|this, _, window, cx| this.open_space(window, cx)))
                        .child(icon("icons/folder-add.svg", pal.dim).size(px(15.)))
                        .child("Abrir pasta como espaço…"),
                ),
            );
        menu.with_animation(
            "spaces-menu-in",
            Animation::new(Duration::from_millis(180)).with_easing(ease_out_quint),
            |el, d| el.opacity(d).top(px(38. + 6. * d)),
        )
    }

    fn render_main(&self, window: &Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = cx.palette();
        let has_note = self.current.is_some();
        let status: SharedString = self.notice.clone().unwrap_or_else(|| match self.save {
            SaveState::Pending => "Salvando…".into(),
            SaveState::Failed => "Erro ao salvar".into(),
            SaveState::Saved => format!("{} palavras", self.words).into(),
        });
        let theme_tip =
            SharedString::from(format!("Tema: {} (Ctrl+Shift+L)", self.theme_pref.label()));

        let toolbar = titlebar_drag(div().id("toolbar"))
            .h(px(48.))
            .flex_none()
            .flex()
            .items_center()
            .gap(px(2.))
            .px(px(9.))
            .child(
                self.ring(
                    5,
                    icon_btn(
                        "toggle-sidebar",
                        "icons/sidebar.svg",
                        "Barra lateral (Ctrl+\\)".into(),
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
                        "Apagar nota (Ctrl+Shift+Backspace)".into(),
                        false,
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.delete_note(window, cx))),
                )
            })
            .child(window_controls(window, &pal));

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
                        .h_full()
                        .child(rise(
                            div().size_full().child(self.editor.clone()),
                            ("editor-in", self.open_gen),
                            420,
                            0.,
                            10.,
                        ))
                        .when_some(
                            self.mark(2, Anchor::TopLeft, point(px(70.), px(70.)), cx),
                            |s, m| s.child(m),
                        ),
                )
                .into_any_element()
        };

        div()
            .flex_1()
            .min_w_0()
            .h_full()
            .flex()
            .flex_col()
            .child(toolbar)
            .child(body)
    }
}

fn session_window(window: &Window) -> SessionWindow {
    let wb = window.window_bounds();
    let b = wb.get_bounds();
    SessionWindow {
        maximized: matches!(wb, WindowBounds::Maximized(_) | WindowBounds::Fullscreen(_)),
        x: b.origin.x.into(),
        y: b.origin.y.into(),
        w: b.size.width.into(),
        h: b.size.height.into(),
    }
}

fn ease_out_quint(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(5)
}

fn icon_btn(id: &'static str, path: &'static str, label: SharedString, on: bool) -> Stateful<Div> {
    let tip = label.clone();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .tooltip(move |window, cx| {
            gpui_kit::component::tooltip::Tooltip::new(tip.clone()).build(window, cx)
        })
        .size(px(30.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .map(|b| {
            let pal = theme::DARK;
            let _ = pal;
            b
        })
        .when(on, |s| s.bg(rgb(0)))
        .child(icon(path, 0))
}

/// Minimize / maximize-restore / close. Shown in both decoration modes: the
/// app requests no compositor titlebar, so these are the only controls.
fn window_controls(window: &Window, pal: &Palette) -> impl IntoElement {
    let caps = window.window_controls();
    let maximized = window.is_maximized();
    let line = pal.line;
    let hover = pal.hover;
    let dim = pal.dim;
    let fg = pal.fg;
    div().map(|row| {
        row.flex()
            .items_center()
            .gap(px(2.))
            .ml(px(6.))
            .pl(px(8.))
            .border_l_1()
            .border_color(rgb(line))
            .when(caps.minimize, |r| {
                r.child(
                    win_btn(
                        "win-min",
                        "icons/minimize.svg",
                        "Minimizar",
                        false,
                        dim,
                        hover,
                    )
                    .on_click(|_, window, _| window.minimize_window()),
                )
            })
            .when(caps.maximize, |r| {
                let (path, label) = if maximized {
                    ("icons/restore.svg", "Restaurar")
                } else {
                    ("icons/maximize.svg", "Maximizar")
                };
                r.child(
                    win_btn("win-max", path, label, false, dim, hover)
                        .on_click(|_, window, _| window.zoom_window()),
                )
            })
            .child(
                win_btn("win-close", "icons/close.svg", "Fechar", true, dim, hover)
                    .on_click(|_, window, _| window.remove_window())
                    .text_color(rgb(fg)),
            )
    })
}

/// A window-chrome button: icon centered in a small square, red hover when
/// `danger` (the close button).
fn win_btn(
    id: &'static str,
    path: &'static str,
    label: &'static str,
    danger: bool,
    dim: u32,
    hover: u32,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .size(px(28.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(6.))
        .cursor_pointer()
        .hover(move |s| {
            if danger {
                s.bg(rgb(0xd92d20)).text_color(rgb(0xffffff))
            } else {
                s.bg(rgb(hover))
            }
        })
        .child(icon(path, dim).size(px(14.)))
}

/// Marks `el` as a window-drag region: primary-button drags move the window.
fn titlebar_drag(el: Stateful<Div>) -> Stateful<Div> {
    el.on_mouse_down(MouseButton::Left, |_, window, _| window.start_window_move())
}

fn bind_keys(cx: &mut App) {
    let c = Some("AbstractApp");
    cx.bind_keys([
        KeyBinding::new("ctrl-n", NewNote, c),
        KeyBinding::new("cmd-n", NewNote, c),
        KeyBinding::new("ctrl-shift-n", NewFolder, c),
        KeyBinding::new("ctrl-o", OpenSpace, c),
        KeyBinding::new("cmd-o", OpenSpace, c),
        KeyBinding::new("ctrl-s", SaveNow, c),
        KeyBinding::new("cmd-s", SaveNow, c),
        KeyBinding::new("ctrl-shift-l", CycleTheme, c),
        KeyBinding::new("cmd-shift-l", CycleTheme, c),
        KeyBinding::new("ctrl-\\", ToggleSidebar, c),
        KeyBinding::new("cmd-\\", ToggleSidebar, c),
        KeyBinding::new("ctrl-shift-backspace", DeleteNote, c),
        KeyBinding::new("f2", RenameNote, c),
        KeyBinding::new("f1", StartTour, c),
    ]);
}

impl Render for AbstractApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pal = cx.palette();
        div()
            .id("abstract-root")
            .key_context("AbstractApp")
            .size_full()
            .flex()
            .font_family(SANS)
            .bg(rgb(pal.bg))
            .text_color(rgb(pal.body))
            .border_1()
            .border_color(rgb(pal.frame_border))
            .on_action(cx.listener(|this, _: &NewNote, window, cx| this.new_note(window, cx)))
            .on_action(cx.listener(|this, _: &NewFolder, window, cx| this.new_folder(window, cx)))
            .on_action(cx.listener(|this, _: &DeleteNote, window, cx| this.delete_note(window, cx)))
            .on_action(
                cx.listener(|this, _: &RenameNote, window, cx| this.rename_current(window, cx)),
            )
            .on_action(cx.listener(|this, _: &SaveNow, _, cx| this.save_now(cx)))
            .on_action(cx.listener(|this, _: &CycleTheme, window, cx| this.cycle_theme(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleSidebar, _, cx| this.toggle_sidebar(cx)))
            .on_action(cx.listener(|this, _: &OpenSpace, window, cx| this.open_space(window, cx)))
            .on_action(cx.listener(|this, _: &ToggleSpaces, _, cx| this.toggle_spaces(cx)))
            .on_action(cx.listener(|this, _: &StartTour, window, cx| this.start_tour(window, cx)))
            .child(self.render_sidebar(cx))
            .child(self.render_main(window, cx))
    }
}

fn main() {
    gpui_kit::application()
        .with_assets(AppAssets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            editor::bind_keys(cx);
            tour::bind_keys(cx);
            bind_keys(cx);

            let settings = Settings::load();
            let session = Session::load();
            let window_bounds = session.window().map(|w| {
                let b = Bounds {
                    origin: point(px(w.x), px(w.y)),
                    size: size(px(w.w), px(w.h)),
                };
                if w.maximized {
                    WindowBounds::Maximized(b)
                } else {
                    WindowBounds::Windowed(b)
                }
            });
            cx.spawn(async move |cx| {
                cx.open_window(
                    WindowOptions {
                        window_bounds,
                        titlebar: Some(TitlebarOptions {
                            title: Some("abstract".into()),
                            appears_transparent: true,
                            traffic_light_position: None,
                        }),
                        window_decorations: Some(WindowDecorations::Client),
                        app_owns_titlebar_drag: true,
                        kind: WindowKind::Normal,
                        is_movable: true,
                        is_resizable: true,
                        is_minimizable: true,
                        focus: true,
                        show: true,
                        app_id: Some("abstract".into()),
                        window_min_size: Some(size(px(560.), px(360.))),
                        ..Default::default()
                    },
                    |window, cx| {
                        theme::apply(settings.theme(), window.appearance(), cx);
                        let view = cx.new(|cx| {
                            AbstractApp::new(window, cx, settings.clone(), session.clone())
                        });
                        cx.new(|cx| Root::new(view, window, cx))
                    },
                )
                .expect("failed to open abstract window");
            })
            .detach();
        });
}
