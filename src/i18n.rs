//! Interface strings, English and Portuguese (Brazil). `t`/`tf` read the
//! active language from a process-wide atomic set at startup or from the
//! spaces-menu language row.

use std::sync::atomic::{AtomicU8, Ordering};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    En,
    PtBr,
}

/// Stored in settings key `lang`: `system` | `en` | `pt-BR`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LangPref {
    System,
    En,
    PtBr,
}

impl LangPref {
    pub fn parse(s: &str) -> Self {
        match s {
            "en" => Self::En,
            "pt-BR" | "pt" => Self::PtBr,
            _ => Self::System,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::System => "system",
            Self::En => "en",
            Self::PtBr => "pt-BR",
        }
    }

    /// Sistema → En → PtBr → Sistema (the menu row cycle).
    pub fn next(&self) -> Self {
        match self {
            Self::System => Self::En,
            Self::En => Self::PtBr,
            Self::PtBr => Self::System,
        }
    }
}

/// OS locale → `pt*` becomes Portuguese, everything else English.
pub fn detect() -> Lang {
    match sys_locale::get_locale() {
        Some(l) if l.to_lowercase().starts_with("pt") => Lang::PtBr,
        _ => Lang::En,
    }
}

static LANG: AtomicU8 = AtomicU8::new(1);

pub fn set(lang: Lang) {
    LANG.store(lang as u8, Ordering::Relaxed);
}

pub fn current() -> Lang {
    match LANG.load(Ordering::Relaxed) {
        0 => Lang::En,
        _ => Lang::PtBr,
    }
}

#[derive(Clone, Copy)]
pub enum Key {
    // Toolbar / status
    Sidebar,
    Search,
    Theme,
    ThemeSystem,
    ThemeLight,
    ThemeDark,
    DeleteNote,
    Saving,
    SaveFailed,
    Words,
    // Sidebar
    NewNote,
    NewFolder,
    SwitchSpace,
    Notes,
    Rename,
    MoveToTrash,
    // Notes / notices / prompts
    TrashFailed,
    TrashFolderPrompt,
    TrashFolderHint,
    Cancel,
    FileRemovedOutside,
    Untitled,
    NameConflict,
    RenameFailed,
    CreateNoteFailed,
    // Search palette
    SearchPlaceholder,
    NoNotesFound,
    // Wiki-links / backlinks
    ReferencedBy,
    // Editor
    EditorPlaceholder,
    EditorAria,
    // Spaces menu
    Spaces,
    OpenFolderAsSpace,
    OpenAsSpace,
    RemoveFromList,
    Language,
    LangSystem,
    // Tour
    Tour1Title,
    Tour1Body,
    Tour2Title,
    Tour2Body,
    Tour3Title,
    Tour3Body,
    Tour4Title,
    Tour4Body,
    Tour5Title,
    Tour5Body,
    Tour6Title,
    Tour6Body,
    TourSkip,
    TourBack,
    TourNext,
    TourDone,
    TourStepOf,
    TourStepAria,
}

impl Key {
    #[cfg(test)]
    pub const ALL: &'static [Key] = &[
        Key::Sidebar,
        Key::Search,
        Key::Theme,
        Key::ThemeSystem,
        Key::ThemeLight,
        Key::ThemeDark,
        Key::DeleteNote,
        Key::Saving,
        Key::SaveFailed,
        Key::Words,
        Key::NewNote,
        Key::NewFolder,
        Key::SwitchSpace,
        Key::Notes,
        Key::Rename,
        Key::MoveToTrash,
        Key::TrashFailed,
        Key::TrashFolderPrompt,
        Key::TrashFolderHint,
        Key::Cancel,
        Key::FileRemovedOutside,
        Key::Untitled,
        Key::NameConflict,
        Key::RenameFailed,
        Key::CreateNoteFailed,
        Key::SearchPlaceholder,
        Key::NoNotesFound,
        Key::ReferencedBy,
        Key::EditorPlaceholder,
        Key::EditorAria,
        Key::Spaces,
        Key::OpenFolderAsSpace,
        Key::OpenAsSpace,
        Key::RemoveFromList,
        Key::Language,
        Key::LangSystem,
        Key::Tour1Title,
        Key::Tour1Body,
        Key::Tour2Title,
        Key::Tour2Body,
        Key::Tour3Title,
        Key::Tour3Body,
        Key::Tour4Title,
        Key::Tour4Body,
        Key::Tour5Title,
        Key::Tour5Body,
        Key::Tour6Title,
        Key::Tour6Body,
        Key::TourSkip,
        Key::TourBack,
        Key::TourNext,
        Key::TourDone,
        Key::TourStepOf,
        Key::TourStepAria,
    ];
}

fn en(k: Key) -> &'static str {
    match k {
        Key::Sidebar => "Sidebar ({MOD}+\\)",
        Key::Search => "Search notes ({MOD}+P)",
        Key::Theme => "Theme: {name} ({MOD}+Shift+L)",
        Key::ThemeSystem => "System",
        Key::ThemeLight => "Light",
        Key::ThemeDark => "Dark",
        Key::DeleteNote => "Delete note ({MOD}+Shift+Backspace)",
        Key::Saving => "Saving…",
        Key::SaveFailed => "Save failed",
        Key::Words => "{n} words",
        Key::NewNote => "New note ({MOD}+N)",
        Key::NewFolder => "New folder",
        Key::SwitchSpace => "Switch space ({MOD}+O)",
        Key::Notes => "NOTES",
        Key::Rename => "Rename",
        Key::MoveToTrash => "Move to Trash",
        Key::TrashFailed => "Could not move to Trash",
        Key::TrashFolderPrompt => "Move the folder “{name}” to Trash?",
        Key::TrashFolderHint => "Notes inside it go too.",
        Key::Cancel => "Cancel",
        Key::FileRemovedOutside => "File removed outside the app",
        Key::Untitled => "Untitled",
        Key::NameConflict => "An item with this name already exists",
        Key::RenameFailed => "Could not rename",
        Key::CreateNoteFailed => "Could not create the note",
        Key::SearchPlaceholder => "Search notes…",
        Key::NoNotesFound => "No notes found",
        Key::ReferencedBy => "Referenced by",
        Key::EditorPlaceholder => "Start writing…",
        Key::EditorAria => "Markdown editor",
        Key::Spaces => "SPACES",
        Key::OpenFolderAsSpace => "Open folder as space…",
        Key::OpenAsSpace => "Open as space",
        Key::RemoveFromList => "Remove from list (files stay)",
        Key::Language => "Language",
        Key::LangSystem => "System",
        Key::Tour1Title => "Spaces are real folders",
        Key::Tour1Body => {
            "Each space is a folder on your disk. Switch spaces or open another folder here (Ctrl+O)."
        }
        Key::Tour2Title => "Notes and folders",
        Key::Tour2Body => {
            "Create notes with Ctrl+N and organize them in folders. Everything becomes a plain .md file."
        }
        Key::Tour3Title => "Write in Markdown",
        Key::Tour3Body => "The first line becomes the title — and the file name.",
        Key::Tour4Title => "Auto-save",
        Key::Tour4Body => {
            "Everything saves by itself. Ctrl+S saves right away; deleting sends to Trash."
        }
        Key::Tour5Title => "Light, dark or system",
        Key::Tour5Body => "Switch the theme with Ctrl+Shift+L.",
        Key::Tour6Title => "Focus mode",
        Key::Tour6Body => "Hide the sidebar with Ctrl+\\. F1 reopens this tour.",
        Key::TourSkip => "Skip",
        Key::TourBack => "Back",
        Key::TourNext => "Next",
        Key::TourDone => "Done",
        Key::TourStepOf => "{step} of {n}",
        Key::TourStepAria => "Step {step} of {n}: {title}",
    }
}

fn pt(k: Key) -> &'static str {
    match k {
        Key::Sidebar => "Barra lateral ({MOD}+\\)",
        Key::Search => "Buscar notas ({MOD}+P)",
        Key::Theme => "Tema: {name} ({MOD}+Shift+L)",
        Key::ThemeSystem => "Sistema",
        Key::ThemeLight => "Claro",
        Key::ThemeDark => "Escuro",
        Key::DeleteNote => "Apagar nota ({MOD}+Shift+Backspace)",
        Key::Saving => "Salvando…",
        Key::SaveFailed => "Erro ao salvar",
        Key::Words => "{n} palavras",
        Key::NewNote => "Nova nota ({MOD}+N)",
        Key::NewFolder => "Nova pasta",
        Key::SwitchSpace => "Trocar de espaço ({MOD}+O)",
        Key::Notes => "NOTAS",
        Key::Rename => "Renomear",
        Key::MoveToTrash => "Mover para a Lixeira",
        Key::TrashFailed => "Não foi possível mover para a Lixeira",
        Key::TrashFolderPrompt => "Mover a pasta “{name}” para a Lixeira?",
        Key::TrashFolderHint => "As notas dentro dela também vão.",
        Key::Cancel => "Cancelar",
        Key::FileRemovedOutside => "Arquivo removido fora do app",
        Key::Untitled => "Sem título",
        Key::NameConflict => "Já existe um item com esse nome",
        Key::RenameFailed => "Não foi possível renomear",
        Key::CreateNoteFailed => "Não foi possível criar a nota",
        Key::SearchPlaceholder => "Buscar notas…",
        Key::NoNotesFound => "Nenhuma nota encontrada",
        Key::ReferencedBy => "Referenciada por",
        Key::EditorPlaceholder => "Comece a escrever…",
        Key::EditorAria => "Editor Markdown",
        Key::Spaces => "ESPAÇOS",
        Key::OpenFolderAsSpace => "Abrir pasta como espaço…",
        Key::OpenAsSpace => "Abrir como espaço",
        Key::RemoveFromList => "Remover da lista (os arquivos ficam)",
        Key::Language => "Idioma",
        Key::LangSystem => "Sistema",
        Key::Tour1Title => "Espaços são pastas reais",
        Key::Tour1Body => {
            "Cada espaço é uma pasta no seu disco. Troque de espaço ou abra outra pasta aqui (Ctrl+O)."
        }
        Key::Tour2Title => "Notas e pastas",
        Key::Tour2Body => {
            "Crie notas com Ctrl+N e organize em pastas. Tudo vira arquivo .md comum."
        }
        Key::Tour3Title => "Escreva em Markdown",
        Key::Tour3Body => "A primeira linha vira o título — e o nome do arquivo.",
        Key::Tour4Title => "Salvamento automático",
        Key::Tour4Body => {
            "Tudo é salvo sozinho. Ctrl+S salva na hora; apagar envia para a Lixeira."
        }
        Key::Tour5Title => "Claro, escuro ou sistema",
        Key::Tour5Body => "Alterne o tema com Ctrl+Shift+L.",
        Key::Tour6Title => "Modo foco",
        Key::Tour6Body => "Esconda a barra lateral com Ctrl+\\. F1 reabre este tour.",
        Key::TourSkip => "Pular",
        Key::TourBack => "Voltar",
        Key::TourNext => "Próximo",
        Key::TourDone => "Concluir",
        Key::TourStepOf => "{step} de {n}",
        Key::TourStepAria => "Passo {step} de {n}: {title}",
    }
}

pub fn t(k: Key) -> &'static str {
    match current() {
        Lang::En => en(k),
        Lang::PtBr => pt(k),
    }
}

/// `t` with `{name}` placeholders replaced from `args`. `{MOD}` expands to
/// `Cmd` on macOS, `Ctrl` elsewhere — same convention as the keymap.
pub fn tf(k: Key, args: &[(&str, &str)]) -> String {
    let mut s = t(k).to_string();
    s = s.replace("{MOD}", crate::keymap::MOD);
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_keys_filled_in_both_languages() {
        for &k in Key::ALL {
            assert!(!en(k).is_empty());
            assert!(!pt(k).is_empty());
            // Placeholder sets must match across languages.
            let ph = |s: &str| {
                let mut v: Vec<String> = s
                    .split('{')
                    .skip(1)
                    .filter_map(|x| x.split('}').next().map(String::from))
                    .collect();
                v.sort();
                v
            };
            assert_eq!(ph(en(k)), ph(pt(k)), "placeholder mismatch: {:?}", en(k));
        }
    }

    #[test]
    fn tf_replaces_placeholders() {
        set(Lang::En);
        assert_eq!(tf(Key::Words, &[("n", "5")]), "5 words");
        set(Lang::PtBr);
        assert_eq!(tf(Key::Words, &[("n", "5")]), "5 palavras");
        assert!(tf(Key::Sidebar, &[]).contains("Cmd+\\"));
    }
}
