use super::*;

impl AbstractApp {
    /// Ctrl/`Cmd`-click on `[[target]]`: open the note it names, creating it
    /// with a `# target` heading when nothing resolves.
    pub(crate) fn open_link(&mut self, target: &str, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = crate::links::resolve(&self.tree, target) {
            self.open_path(path, None, window, cx);
            return;
        }
        let stem = vault::stem_for_title(target);
        let path = vault::unique_path(&self.dir, &stem, None);
        if let Err(err) = store::write_atomic(&path, format!("# {target}\n\n").as_bytes()) {
            eprintln!("abstract: cannot create linked note: {err}");
            self.notice = Some("Não foi possível criar a nota".into());
            cx.notify();
            return;
        }
        self.expand_to(&path);
        self.rescan_tree(cx);
        self.open_path(path, None, window, cx);
    }
}
