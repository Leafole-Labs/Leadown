// Leadown frontend — communicates with the Rust backend via Tauri invoke

const { invoke } = window.__TAURI__.core;

// ── State ──────────────────────────────────────────────────────────────

let state = {
    spaces: [],
    activeSpace: 0,
    tree: [],
    expanded: new Set(),
    currentNote: null,
    notePath: null,
    noteContent: '',
    settings: { theme: 'system', lang: 'system', tourDone: false },
    saveTimeout: null,
    sidebarOpen: true,
    searchOpen: false,
    tourStep: -1,
    editing: false,
    words: 0,
};

// ── i18n ──────────────────────────────────────────────────────────────

const I18N = {
    en: {
        notes: 'NOTES',
        spaces: 'SPACES',
        newNote: 'New Note',
        newFolder: 'New Folder',
        searchPlaceholder: 'Search notes...',
        noNotesFound: 'No notes found',
        startWriting: 'Start writing...',
        untitled: 'Untitled',
        words: '{n} words',
        saving: 'Saving…',
        saveFailed: 'Save failed',
        referencedBy: 'Referenced by',
        fileRemovedOutside: 'File removed outside the app',
        moveToTrash: 'Move to Trash',
        couldNotMoveToTrash: 'Could not move to Trash',
        moveFolderPrompt: 'Move the folder "{name}" to Trash?',
        moveFolderHint: 'Notes inside it go too.',
        cancel: 'Cancel',
        rename: 'Rename',
        nameConflict: 'An item with this name already exists',
        couldNotRename: 'Could not rename',
        couldNotCreateNote: 'Could not create the note',
        openFolderAsSpace: 'Open folder as space...',
        openAsSpace: 'Open as space',
        removeFromList: 'Remove from list (files stay)',
        language: 'Language',
        system: 'System',
        light: 'Light',
        dark: 'Dark',
        theme: 'Theme: {name}',
        sidebar: 'Sidebar',
        search: 'Search notes',
        deleteNote: 'Delete note',
        tour1Title: 'Spaces are real folders',
        tour1Body: 'Each space is a folder on your disk. Switch spaces or open another folder here (Ctrl+O).',
        tour2Title: 'Notes and folders',
        tour2Body: 'Create notes with Ctrl+N and organize them in folders. Everything becomes a plain .md file.',
        tour3Title: 'Write in Markdown',
        tour3Body: 'The first line becomes the title — and the file name.',
        tour4Title: 'Auto-save',
        tour4Body: 'Everything saves by itself. Ctrl+S saves right away; deleting sends to Trash.',
        tour5Title: 'Light, dark or system',
        tour5Body: 'Switch the theme with Ctrl+Shift+L.',
        tour6Title: 'Focus mode',
        tour6Body: 'Hide the sidebar with Ctrl+\\. F1 reopens this tour.',
        skip: 'Skip',
        back: 'Back',
        next: 'Next',
        done: 'Done',
        stepOf: '{step} of {n}',
    },
    pt: {
        notes: 'NOTAS',
        spaces: 'ESPAÇOS',
        newNote: 'Nova Nota',
        newFolder: 'Nova Pasta',
        searchPlaceholder: 'Buscar notas...',
        noNotesFound: 'Nenhuma nota encontrada',
        startWriting: 'Comece a escrever...',
        untitled: 'Sem título',
        words: '{n} palavras',
        saving: 'Salvando…',
        saveFailed: 'Erro ao salvar',
        referencedBy: 'Referenciada por',
        fileRemovedOutside: 'Arquivo removido fora do app',
        moveToTrash: 'Mover para a Lixeira',
        couldNotMoveToTrash: 'Não foi possível mover para a Lixeira',
        moveFolderPrompt: 'Mover a pasta "{name}" para a Lixeira?',
        moveFolderHint: 'As notas dentro dela também vão.',
        cancel: 'Cancelar',
        rename: 'Renomear',
        nameConflict: 'Já existe um item com esse nome',
        couldNotRename: 'Não foi possível renomear',
        couldNotCreateNote: 'Não foi possível criar a nota',
        openFolderAsSpace: 'Abrir pasta como espaço...',
        openAsSpace: 'Abrir como espaço',
        removeFromList: 'Remover da lista (os arquivos ficam)',
        language: 'Idioma',
        system: 'Sistema',
        light: 'Claro',
        dark: 'Escuro',
        theme: 'Tema: {name}',
        sidebar: 'Barra lateral',
        search: 'Buscar notas',
        deleteNote: 'Apagar nota',
        tour1Title: 'Espaços são pastas reais',
        tour1Body: 'Cada espaço é uma pasta no seu disco. Troque de espaço ou abra outra pasta aqui (Ctrl+O).',
        tour2Title: 'Notas e pastas',
        tour2Body: 'Crie notas com Ctrl+N e organize em pastas. Tudo vira arquivo .md comum.',
        tour3Title: 'Escreva em Markdown',
        tour3Body: 'A primeira linha vira o título — e o nome do arquivo.',
        tour4Title: 'Salvamento automático',
        tour4Body: 'Tudo é salvo sozinho. Ctrl+S salva na hora; apagar envia para a Lixeira.',
        tour5Title: 'Claro, escuro ou sistema',
        tour5Body: 'Alterne o tema com Ctrl+Shift+L.',
        tour6Title: 'Modo foco',
        tour6Body: 'Esconda a barra lateral com Ctrl+\\. F1 reabre este tour.',
        skip: 'Pular',
        back: 'Voltar',
        next: 'Próximo',
        done: 'Concluir',
        stepOf: '{step} de {n}',
    },
};

let lang = 'en';

function detectLang() {
    const l = (navigator.language || 'en').toLowerCase();
    return l.startsWith('pt') ? 'pt' : 'en';
}

function t(key, args) {
    let s = (I18N[lang] && I18N[lang][key]) || I18N.en[key] || key;
    if (args) {
        for (const [k, v] of Object.entries(args)) {
            s = s.replace(`{${k}}`, v);
        }
    }
    return s;
}

function tf(key, args) {
    return t(key, args).replace('{MOD}', isMac() ? 'Cmd' : 'Ctrl');
}

function isMac() {
    return navigator.platform.toLowerCase().includes('mac');
}

// ── Theme ──────────────────────────────────────────────────────────────

function applyTheme(theme) {
    if (theme === 'dark') {
        document.documentElement.removeAttribute('data-theme');
    } else if (theme === 'light') {
        document.documentElement.setAttribute('data-theme', 'light');
    } else {
        // system
        const prefersDark = window.matchMedia('(prefers-color-scheme: dark)').matches;
        if (prefersDark) {
            document.documentElement.removeAttribute('data-theme');
        } else {
            document.documentElement.setAttribute('data-theme', 'light');
        }
    }
}

// ── DOM refs ───────────────────────────────────────────────────────────

const $ = (id) => document.getElementById(id);

const els = {};

function cacheEls() {
    els.sidebar = $('sidebar');
    els.noteTree = $('note-tree');
    els.editor = $('editor');
    els.preview = $('markdown-preview');
    els.emptyState = $('empty-state');
    els.btnEmptyNew = $('btn-empty-new');
    els.wordCount = $('word-count');
    els.saveStatus = $('save-status');
    els.searchPalette = $('search-palette');
    els.searchInput = $('search-input');
    els.searchResults = $('search-results');
    els.tourOverlay = $('tour-overlay');
    els.tourTitle = $('tour-title');
    els.tourBody = $('tour-body');
    els.tourStep = $('tour-step');
    els.tourBack = $('tour-back');
    els.tourNext = $('tour-next');
    els.tourSkip = $('tour-skip');
    els.backlinks = $('backlinks');
    els.backlinkList = $('backlink-list');
    els.btnDelete = $('btn-delete');
    els.titlebar = $('titlebar');
    els.titlebarTitle = $('titlebar-title');
    els.windowControls = $('window-controls');
}

// ── Note operations ────────────────────────────────────────────────────

async function loadNote(path) {
    try {
        const content = await invoke('load_note', { path });
        state.notePath = path;
        state.noteContent = content;
        state.editing = false;
        els.editor.value = content;
        els.editor.classList.remove('visible');
        els.preview.classList.remove('hidden');
        els.emptyState.classList.add('hidden');
        updatePreview();
        updateWordCount();
        updateTitle();
        els.btnDelete.classList.remove('hidden');
        hideBacklinks();
    } catch (e) {
        console.error('Failed to load note:', e);
    }
}

async function saveNote() {
    if (!state.notePath) return;
    els.saveStatus.textContent = t('saving');
    try {
        await invoke('save_note', { path: state.notePath, content: state.noteContent });
        els.saveStatus.textContent = '';
    } catch (e) {
        els.saveStatus.textContent = t('saveFailed');
        console.error('Failed to save:', e);
    }
}

function scheduleSave() {
    els.saveStatus.textContent = t('saving');
    clearTimeout(state.saveTimeout);
    state.saveTimeout = setTimeout(() => {
        saveNote();
    }, 400);
}

let previewTimeout;
function schedulePreview() {
    clearTimeout(previewTimeout);
    previewTimeout = setTimeout(() => {
        updatePreview();
    }, 300);
}

function updatePreview() {
    if (!state.notePath) {
        els.preview.innerHTML = '';
        return;
    }
    const md = state.noteContent;
    invoke('markdown_to_html', { text: md }).then((html) => {
        els.preview.innerHTML = html;
        attachPreviewHandlers();
    }).catch(() => {});
}

function attachPreviewHandlers() {
    // Wiki-links
    els.preview.querySelectorAll('a[data-wiki]').forEach((a) => {
        a.addEventListener('click', (e) => {
            e.preventDefault();
            const target = a.getAttribute('data-wiki');
            openWikiLink(target);
        });
    });

    // Task checkboxes
    els.preview.querySelectorAll('input[type="checkbox"]').forEach((cb) => {
        cb.addEventListener('change', () => {
            toggleTask(cb);
        });
    });
}

function toggleTask(cb) {
    const li = cb.closest('li');
    if (!li) return;
    const text = li.textContent;
    const checked = cb.checked;
    const marker = checked ? '[x]' : '[ ]';
    const newText = text.replace(/^\[([ xX])\]/, marker);
    // Find and replace in the source
    const lines = state.noteContent.split('\n');
    for (let i = 0; i < lines.length; i++) {
        if (lines[i].includes(text.trim())) {
            lines[i] = lines[i].replace(/^\[([ xX])\]/, marker);
            break;
        }
    }
    state.noteContent = lines.join('\n');
    updatePreview();
    scheduleSave();
}

function updateWordCount() {
    state.words = state.noteContent.split(/\s+/).filter(Boolean).length;
    els.wordCount.textContent = t('words', { n: state.words });
}

function updateTitle() {
    const lines = state.noteContent.split('\n');
    for (const line of lines) {
        const trimmed = line.trim().replace(/^#+\s*/, '').trim();
        if (trimmed) {
            const title = trimmed.slice(0, 80);
            els.titlebarTitle.textContent = title;
            document.title = `${title} — Leadown`;
            return;
        }
    }
    els.titlebarTitle.textContent = 'Leadown';
    document.title = 'Leadown';
}

// ── Tree ───────────────────────────────────────────────────────────────

async function refreshTree() {
    if (state.spaces.length === 0) return;
    const root = state.spaces[state.activeSpace];
    try {
        state.tree = await invoke('list_tree', { root });
        renderTree();
    } catch (e) {
        console.error('Failed to list tree:', e);
    }
}

function renderTree() {
    els.noteTree.innerHTML = '';
    const rows = flattenTree(state.tree, 0);
    for (const row of rows) {
        els.noteTree.appendChild(renderTreeRow(row));
    }
}

function flattenTree(nodes, depth) {
    const rows = [];
    for (const node of nodes) {
        rows.push({ node, depth });
        if (node.kind === 'folder' && state.expanded.has(node.path)) {
            rows.push(...flattenTree(node.children, depth + 1));
        }
    }
    return rows;
}

function renderTreeRow({ node, depth }) {
    const div = document.createElement('div');
    div.className = `tree-item ${node.kind === 'folder' ? 'folder' : ''}`;
    div.style.paddingLeft = `${8 + depth * 12}px`;

    if (node.kind === 'folder') {
        const expanded = state.expanded.has(node.path);
        div.innerHTML = `
            <svg class="chevron ${expanded ? 'expanded' : ''}" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="9,18 15,12 9,6"/></svg>
            <svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
            <span class="name">${escapeHtml(node.name)}</span>
        `;
        div.addEventListener('click', () => {
            if (state.expanded.has(node.path)) {
                state.expanded.delete(node.path);
            } else {
                state.expanded.add(node.path);
            }
            renderTree();
        });
    } else {
        div.innerHTML = `
            <span style="width:12px"></span>
            <svg class="icon" width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/><polyline points="14,2 14,8 20,8"/></svg>
            <span class="name">${escapeHtml(node.name)}</span>
        `;
        if (state.notePath === node.path) {
            div.classList.add('active');
        }
        div.addEventListener('click', () => {
            loadNote(node.path);
            renderTree();
        });
    }

    return div;
}

function escapeHtml(s) {
    const div = document.createElement('div');
    div.textContent = s;
    return div.innerHTML;
}

// ── Search ─────────────────────────────────────────────────────────────

async function openSearch() {
    els.searchPalette.classList.remove('hidden');
    els.searchInput.focus();
    await runSearch('');
}

function closeSearch() {
    els.searchPalette.classList.add('hidden');
    els.searchInput.value = '';
    els.searchResults.innerHTML = '';
}

async function runSearch(query) {
    if (state.spaces.length === 0) return;
    const root = state.spaces[state.activeSpace];
    try {
        const hits = await invoke('search_notes', { root, query });
        renderSearchResults(hits, query);
    } catch (e) {
        console.error('Search failed:', e);
    }
}

function renderSearchResults(hits, query) {
    els.searchResults.innerHTML = '';
    if (hits.length === 0) {
        els.searchResults.innerHTML = `<div style="padding:16px;color:var(--faint);font-size:12px">${t('noNotesFound')}</div>`;
        return;
    }
    for (const hit of hits) {
        const div = document.createElement('div');
        div.className = 'search-hit';
        div.innerHTML = `
            <div class="hit-title">${escapeHtml(hit.title || t('untitled'))}</div>
            <div class="hit-snippet">${escapeHtml(hit.snippet || '')}</div>
        `;
        div.addEventListener('click', () => {
            closeSearch();
            loadNote(hit.path);
        });
        els.searchResults.appendChild(div);
    }
}

// ── Wiki-links ─────────────────────────────────────────────────────────

function openWikiLink(target) {
    // Try to find the note in the tree
    const root = state.spaces[state.activeSpace];
    const found = findInTree(state.tree, target);
    if (found) {
        loadNote(found);
    } else {
        // Create new note with this title
        createNoteWithName(target);
    }
}

function findInTree(nodes, target) {
    const lower = target.toLowerCase().trim();
    for (const node of nodes) {
        if (node.kind === 'note') {
            const stem = node.name.replace(/\.md$/, '').toLowerCase();
            if (stem === lower) return node.path;
        }
        if (node.kind === 'folder') {
            const found = findInTree(node.children, target);
            if (found) return found;
        }
    }
    return null;
}

async function createNoteWithName(name) {
    const root = state.spaces[state.activeSpace];
    const path = `${root}/${name}.md`;
    try {
        await invoke('create_note', { path, content: `# ${name}\n` });
        await refreshTree();
        await loadNote(path);
    } catch (e) {
        console.error('Failed to create note:', e);
    }
}

// ── New note / folder ──────────────────────────────────────────────────

async function newNote() {
    const root = state.spaces[state.activeSpace];
    if (!root) return;
    let name = t('untitled');
    let path = `${root}/${name}.md`;
    let counter = 2;
    while (await fileExists(path)) {
        name = `${t('untitled')} ${counter}`;
        path = `${root}/${name}.md`;
        counter++;
    }
    try {
        await invoke('create_note', { path, content: `# ${name}\n` });
        await refreshTree();
        await loadNote(path);
        startEditing();
    } catch (e) {
        console.error('Failed to create note:', e);
    }
}

async function newFolder() {
    const root = state.spaces[state.activeSpace];
    if (!root) return;
    const name = prompt(t('newFolder'));
    if (!name) return;
    const path = `${root}/${name}`;
    try {
        await invoke('create_folder', { path });
        await refreshTree();
    } catch (e) {
        console.error('Failed to create folder:', e);
    }
}

async function fileExists(path) {
    // Check in tree
    const found = findInTree(state.tree, path.split('/').pop().replace(/\.md$/, ''));
    return found !== null;
}

// ── Editing ────────────────────────────────────────────────────────────

function startEditing() {
    if (!state.notePath) return;
    state.editing = true;
    els.preview.classList.add('hidden');
    els.editor.classList.add('visible');
    els.editor.focus();
    // Move cursor to end
    els.editor.setSelectionRange(els.editor.value.length, els.editor.value.length);
}

function stopEditing() {
    state.editing = false;
    state.noteContent = els.editor.value;
    els.editor.classList.remove('visible');
    els.preview.classList.remove('hidden');
    updatePreview();
    updateWordCount();
    updateTitle();
    scheduleSave();
}

// ── Backlinks ──────────────────────────────────────────────────────────

function showBacklinks(links) {
    if (links.length === 0) {
        hideBacklinks();
        return;
    }
    els.backlinks.classList.remove('hidden');
    els.backlinkList.innerHTML = '';
    for (const [path, title] of links) {
        const li = document.createElement('li');
        const a = document.createElement('a');
        a.textContent = title;
        a.addEventListener('click', () => loadNote(path));
        li.appendChild(a);
        els.backlinkList.appendChild(li);
    }
}

function hideBacklinks() {
    els.backlinks.classList.add('hidden');
}

// ── Tour ───────────────────────────────────────────────────────────────

const TOUR_STEPS = [
    { title: 'tour1Title', body: 'tour1Body' },
    { title: 'tour2Title', body: 'tour2Body' },
    { title: 'tour3Title', body: 'tour3Body' },
    { title: 'tour4Title', body: 'tour4Body' },
    { title: 'tour5Title', body: 'tour5Body' },
    { title: 'tour6Title', body: 'tour6Body' },
];

function startTour() {
    state.tourStep = 0;
    els.tourOverlay.classList.remove('hidden');
    renderTourStep();
}

function stopTour() {
    state.tourStep = -1;
    els.tourOverlay.classList.add('hidden');
}

function renderTourStep() {
    if (state.tourStep < 0 || state.tourStep >= TOUR_STEPS.length) {
        stopTour();
        return;
    }
    const step = TOUR_STEPS[state.tourStep];
    els.tourTitle.textContent = t(step.title);
    els.tourBody.textContent = t(step.body);
    els.tourStep.textContent = t('stepOf', { step: state.tourStep + 1, n: TOUR_STEPS.length });
    els.tourBack.classList.toggle('hidden', state.tourStep === 0);
    els.tourNext.textContent = state.tourStep === TOUR_STEPS.length - 1 ? t('done') : t('next');
}

function tourNext() {
    if (state.tourStep >= TOUR_STEPS.length - 1) {
        stopTour();
    } else {
        state.tourStep++;
        renderTourStep();
    }
}

function tourBack() {
    if (state.tourStep > 0) {
        state.tourStep--;
        renderTourStep();
    }
}

// ── Settings ───────────────────────────────────────────────────────────

async function loadSettings() {
    try {
        const settings = await invoke('get_settings');
        state.settings = settings;
        applyTheme(settings.theme);
        lang = settings.lang === 'en' ? 'en' : settings.lang === 'pt-BR' ? 'pt' : detectLang();
        document.documentElement.lang = lang;
    } catch (e) {
        console.error('Failed to load settings:', e);
    }
}

async function saveSettings() {
    try {
        await invoke('save_settings', { settings: state.settings });
    } catch (e) {
        console.error('Failed to save settings:', e);
    }
}

function cycleTheme() {
    const order = ['system', 'light', 'dark'];
    const idx = order.indexOf(state.settings.theme);
    state.settings.theme = order[(idx + 1) % order.length];
    applyTheme(state.settings.theme);
    saveSettings();
}

// ── Spaces ─────────────────────────────────────────────────────────────

async function loadSpaces() {
    try {
        const spaces = await invoke('load_spaces');
        state.spaces = spaces.paths || [];
        state.activeSpace = spaces.active || 0;
        if (state.spaces.length === 0) {
            // Wait for spaces to be initialized
            await loadSpaces();
            return;
        }
        await refreshTree();
    } catch (e) {
        console.error('Failed to load spaces:', e);
    }
}

async function saveSpaces() {
    try {
        await invoke('save_spaces', { spaces: { paths: state.spaces, active: state.activeSpace } });
    } catch (e) {
        console.error('Failed to save spaces:', e);
    }
}

// ── Delete note ────────────────────────────────────────────────────────

async function deleteCurrentNote() {
    if (!state.notePath) return;
    try {
        await invoke('delete_item', { path: state.notePath });
        state.notePath = null;
        state.noteContent = '';
        els.editor.value = '';
        els.preview.innerHTML = '';
        els.btnDelete.classList.add('hidden');
        hideBacklinks();
        els.titlebarTitle.textContent = 'Leadown';
        document.title = 'Leadown';
        await refreshTree();
    } catch (e) {
        console.error('Failed to delete note:', e);
    }
}

// ── Event listeners ────────────────────────────────────────────────────

function bindEvents() {
    // Titlebar buttons
    $('btn-sidebar').addEventListener('click', () => {
        state.sidebarOpen = !state.sidebarOpen;
        els.sidebar.classList.toggle('hidden', !state.sidebarOpen);
    });

    $('btn-search').addEventListener('click', () => {
        if (els.searchPalette.classList.contains('hidden')) {
            openSearch();
        } else {
            closeSearch();
        }
    });

    $('btn-theme').addEventListener('click', cycleTheme);

    els.btnDelete.addEventListener('click', deleteCurrentNote);

    // Sidebar buttons
    $('btn-new-note').addEventListener('click', newNote);
    $('btn-new-folder').addEventListener('click', newFolder);

    // Search input
    let searchTimeout;
    els.searchInput.addEventListener('input', (e) => {
        clearTimeout(searchTimeout);
        searchTimeout = setTimeout(() => {
            runSearch(e.target.value);
        }, 200);
    });

    els.searchInput.addEventListener('keydown', (e) => {
        if (e.key === 'Escape') {
            closeSearch();
        }
    });

    // Editor
    els.editor.addEventListener('input', () => {
        state.noteContent = els.editor.value;
        updateWordCount();
        scheduleSave();
        schedulePreview();
    });

    // Empty state button
    els.btnEmptyNew.addEventListener('click', newNote);

    els.editor.addEventListener('blur', () => {
        if (state.editing) {
            stopEditing();
        }
    });

    // Tour
    els.tourSkip.addEventListener('click', stopTour);
    els.tourBack.addEventListener('click', tourBack);
    els.tourNext.addEventListener('click', tourNext);

    // Window controls
    if (window.__TAURI__) {
        const win = window.__TAURI__.window.getCurrent();
        
        $('win-min').addEventListener('click', () => win.minimize());
        $('win-max').addEventListener('click', () => win.toggleMaximize());
        $('win-close').addEventListener('click', () => win.close());
        
        // Drag region for moving the window
        let dragging = false;
        let dragStart = { x: 0, y: 0 };
        
        els.titlebar.addEventListener('mousedown', (e) => {
            if (e.target.closest('button')) return;
            dragging = true;
            dragStart = { x: e.screenX, y: e.screenY };
        });
        
        document.addEventListener('mousemove', (e) => {
            if (!dragging) return;
            const dx = e.screenX - dragStart.x;
            const dy = e.screenY - dragStart.y;
            win.setPosition({ x: e.screenX - e.clientX, y: e.screenY - e.clientY });
        });
        
        document.addEventListener('mouseup', () => {
            dragging = false;
        });
    }

    // Keyboard shortcuts
    document.addEventListener('keydown', (e) => {
        const mod = isMac() ? e.metaKey : e.ctrlKey;

        if (mod && e.key === 'n' && !e.shiftKey) {
            e.preventDefault();
            newNote();
        } else if (mod && e.key === 'n' && e.shiftKey) {
            e.preventDefault();
            newFolder();
        } else if (mod && e.key === 'p') {
            e.preventDefault();
            openSearch();
        } else if (mod && e.key === 's') {
            e.preventDefault();
            saveNote();
        } else if (mod && e.key === 'l' && e.shiftKey) {
            e.preventDefault();
            cycleTheme();
        } else if (mod && e.key === '\\') {
            e.preventDefault();
            state.sidebarOpen = !state.sidebarOpen;
            els.sidebar.classList.toggle('hidden', !state.sidebarOpen);
        } else if (mod && e.key === 'Backspace' && e.shiftKey) {
            e.preventDefault();
            deleteCurrentNote();
        } else if (e.key === 'F1') {
            e.preventDefault();
            startTour();
        } else if (e.key === 'Escape' && state.editing) {
            stopEditing();
        }
    });

    // Click outside search to close
    els.searchPalette.addEventListener('click', (e) => {
        if (e.target === els.searchPalette) {
            closeSearch();
        }
    });
}

// ── Initialization ─────────────────────────────────────────────────────

async function init() {
    cacheEls();
    bindEvents();
    await loadSettings();
    await loadSpaces();

    // Load the first available note or create a new one
    if (state.tree.length > 0) {
        const firstNote = findFirstNote(state.tree);
        if (firstNote) {
            await loadNote(firstNote);
        } else {
            await newNote();
        }
    } else {
        await newNote();
    }

    // Show tour on first run
    if (!state.settings.tourDone) {
        startTour();
    }

    // Apply initial theme
    applyTheme(state.settings.theme);

    // Set language
    lang = state.settings.lang === 'en' ? 'en' : state.settings.lang === 'pt-BR' ? 'pt' : detectLang();
    document.documentElement.lang = lang;
}

function findFirstNote(nodes) {
    for (const node of nodes) {
        if (node.kind === 'note') return node.path;
        if (node.kind === 'folder') {
            const found = findFirstNote(node.children);
            if (found) return found;
        }
    }
    return null;
}

function safeInit() {
    try {
        if (!window.__TAURI__) {
            console.error('TAURI not available');
            setTimeout(safeInit, 100);
            return;
        }
        init();
    } catch (e) {
        console.error('Init error:', e);
    }
}

document.addEventListener('DOMContentLoaded', safeInit);
