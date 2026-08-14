import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import UPlot from "uplot";
import "uplot/dist/uPlot.min.css";

// ---------- Определение ОС для отображения хоткеев ----------
// Формат хранения (CmdOrCtrl+Shift+Space, F13 и т.д.) — тот, что понимает
// tauri-plugin-global-shortcut, и он не меняется. Здесь только отображение.

type Platform = "mac" | "windows" | "linux";

function detectPlatform(): Platform {
  const uaData = (navigator as any).userAgentData;
  const platformStr: string = uaData?.platform ?? navigator.platform ?? navigator.userAgent;
  const p = platformStr.toLowerCase();
  if (p.includes("mac")) return "mac";
  if (p.includes("win")) return "windows";
  return "linux";
}

const PLATFORM = detectPlatform();

const KEY_SYMBOLS: Record<Platform, Record<string, string>> = {
  mac: { CmdOrCtrl: "⌘", Shift: "⇧", Alt: "⌥", Super: "⌘", Space: "Space" },
  windows: { CmdOrCtrl: "Ctrl", Shift: "Shift", Alt: "Alt", Super: "Win", Space: "Space" },
  linux: { CmdOrCtrl: "Ctrl", Shift: "Shift", Alt: "Alt", Super: "Super", Space: "Space" },
};

/** Форматирует accelerator-строку (как хранится в настройках) для показа пользователю. */
function formatAccelerator(accel: string): string {
  if (!accel) return "—";
  const symbols = KEY_SYMBOLS[PLATFORM];
  const parts = accel.split("+").map((token) => symbols[token] ?? token);
  return PLATFORM === "mac" ? parts.join("") : parts.join("+");
}

// ---------- i18n ----------

type Lang = "ru" | "en";

const TRANSLATIONS: Record<Lang, Record<string, string>> = {
  ru: {
    "nav.home": "Главная",
    "nav.history": "История",
    "nav.stats": "Статистика",
    "nav.settings": "Настройки",
    "home.title": "Дикта",
    "home.subtitle": "Локальное распознавание русской речи",
    "home.ready": "Готова слушать",
    "home.readyHint": "— настройте хоткеи во вкладке «Настройки»",
    "home.lastLabel": "Последняя запись",
    "home.lastEmpty": "Пока ничего не распознано.",
    "home.listening": "Слушаю…",
    "home.recognizing": "Распознаю…",
    "home.modeToggle": "режим: toggle — нажмите хоткей ещё раз",
    "home.modePush": "режим: push-to-talk — держите клавишу",
    "home.autoInserted": "✓ вставлено автоматически",
    "home.copiedToClipboard": "ℹ скопировано в буфер — вставьте вручную (Cmd/Ctrl+V)",
    "home.noModel": "Модель не выбрана",
    "home.noModelHint": "откройте «Настройки» → «Модель распознавания» и скачайте одну из моделей",
    "history.title": "История",
    "history.subtitle": "Поиск, копирование, удаление",
    "history.searchPlaceholder": "Поиск по тексту…",
    "history.empty": "Ничего не найдено",
    "history.copy": "Копировать",
    "history.delete": "Удалить",
    "history.words": "слов",
    "stats.title": "Статистика",
    "stats.subtitle": "Активность и качество сигнала",
    "stats.timeChart": "Время диктовки по дням",
    "stats.qualityChart": "Качество сигнала (RMS-прокси, не модельная confidence)",
    "stats.todayTime": "Сегодня наговорено",
    "stats.weekTime": "За неделю",
    "stats.wordsToday": "Слов сегодня",
    "stats.sessionsAll": "Сессий всего",
    "stats.empty": "Пока нет данных",
    "stats.minutesSeries": "минут",
    "stats.qualitySeries": "качество сигнала",
    "settings.title": "Настройки",
    "settings.subtitle": "Хоткеи, голос, данные",
    "settings.hotkeys": "Хоткеи",
    "settings.pushToTalk": "Push-to-talk",
    "settings.pushToTalkDesc": "Держать — говорить, отпустить — распознать",
    "settings.toggle": "Toggle",
    "settings.toggleDesc": "Нажать — начать, нажать ещё раз — остановить",
    "settings.pressCombo": "Нажмите комбинацию…",
    "settings.voice": "Голос",
    "settings.sensitivity": "Чувствительность автостопа",
    "settings.sensitivityDesc": "Насколько тихо должно быть, чтобы запись остановилась сама",
    "settings.hangover": "Пауза до автостопа",
    "settings.hangoverDesc": "Сколько тишины ждать перед остановкой — короче для команд, длиннее для связной речи",
    "settings.autostopPush": "Автостоп в push-to-talk",
    "settings.autostopPushDesc": "Дополнительно к отпусканию клавиши",
    "settings.autostopToggle": "Автостоп в toggle",
    "settings.autostopToggleDesc": "Не ждать повторного нажатия",
    "settings.denoise": "Шумоподавление",
    "settings.denoiseDesc": "Чистит голос перед распознаванием — проще модели, меньше ошибок в шуме",
    "settings.textInsertion": "Вставка текста",
    "settings.autopaste": "Автовставка в текущее поле",
    "settings.autopasteDesc": "Если недоступна — текст уйдёт в буфер обмена",
    "settings.inputDevice": "Устройство ввода",
    "settings.microphone": "Микрофон",
    "settings.microphoneDesc": "Какое устройство слушать",
    "settings.systemDefault": "Системное по умолчанию",
    "settings.theme": "Тема",
    "settings.themeLabel": "Цветовая гамма",
    "settings.themeDesc": "Крем и терракота, мятная или лавандовая",
    "settings.model": "Модель распознавания",
    "settings.customModelHint": "Добавить свою sherpa-onnx-совместимую модель с Hugging Face",
    "settings.customModelPlaceholder": "например: Smirnov75/GigaAM-v3-sherpa-onnx",
    "settings.findFiles": "Найти файлы",
    "sensitivity.whisper": "Шёпот — тишина",
    "sensitivity.quietRoom": "Тихая комната",
    "sensitivity.normal": "Обычный разговор",
    "sensitivity.cafe": "Шумное кафе",
    "sensitivity.openSpace": "Открытый опенспейс",
    "sensitivity.construction": "Стройка",
    "models.active": "Активна",
    "models.use": "Использовать",
    "models.download": "Скачать",
    "models.downloading": "Скачивается…",
    "models.retryError": "Ошибка — повторить",
    "models.remove": "Удалить",
    "models.punctuation": "пунктуация",
    "models.noPunctuation": "без пунктуации",
    "models.measuredNote": "Замерено локально, на вашей машине может отличаться",
    "models.speedFactor": "реального времени",
    "models.loadTime": "загрузка",
    "hf.searching": "Ищу файлы…",
    "hf.searchError": "Не удалось получить список файлов",
    "hf.modelType": "Тип модели",
    "hf.ctcOneFile": "CTC (один файл)",
    "hf.transducerThreeFiles": "Transducer (3 файла)",
    "hf.addAndDownload": "Добавить и скачать",
    "hf.adding": "Добавляю…",
    "hf.tokens": "Файл токенов",
    "hf.model": "Модель (CTC)",
    "hf.encoder": "Encoder",
    "hf.decoder": "Decoder",
    "hf.joiner": "Joiner",
  },
  en: {
    "nav.home": "Home",
    "nav.history": "History",
    "nav.stats": "Stats",
    "nav.settings": "Settings",
    "home.title": "Dikta",
    "home.subtitle": "Local Russian speech recognition",
    "home.ready": "Ready to listen",
    "home.readyHint": "— set up hotkeys in the Settings tab",
    "home.lastLabel": "Last transcription",
    "home.lastEmpty": "Nothing recognized yet.",
    "home.listening": "Listening…",
    "home.recognizing": "Recognizing…",
    "home.modeToggle": "mode: toggle — press the hotkey again",
    "home.modePush": "mode: push-to-talk — hold the key",
    "home.autoInserted": "✓ inserted automatically",
    "home.copiedToClipboard": "ℹ copied to clipboard — paste manually (Cmd/Ctrl+V)",
    "home.noModel": "No model selected",
    "home.noModelHint": "open Settings → \"Recognition model\" and download one",
    "history.title": "History",
    "history.subtitle": "Search, copy, delete",
    "history.searchPlaceholder": "Search text…",
    "history.empty": "Nothing found",
    "history.copy": "Copy",
    "history.delete": "Delete",
    "history.words": "words",
    "stats.title": "Stats",
    "stats.subtitle": "Activity and signal quality",
    "stats.timeChart": "Dictation time by day",
    "stats.qualityChart": "Signal quality (RMS proxy, not model confidence)",
    "stats.todayTime": "Dictated today",
    "stats.weekTime": "This week",
    "stats.wordsToday": "Words today",
    "stats.sessionsAll": "Sessions total",
    "stats.empty": "No data yet",
    "stats.minutesSeries": "minutes",
    "stats.qualitySeries": "signal quality",
    "settings.title": "Settings",
    "settings.subtitle": "Hotkeys, voice, data",
    "settings.hotkeys": "Hotkeys",
    "settings.pushToTalk": "Push-to-talk",
    "settings.pushToTalkDesc": "Hold to speak, release to recognize",
    "settings.toggle": "Toggle",
    "settings.toggleDesc": "Press to start, press again to stop",
    "settings.pressCombo": "Press a combination…",
    "settings.voice": "Voice",
    "settings.sensitivity": "Auto-stop sensitivity",
    "settings.sensitivityDesc": "How quiet it needs to get for recording to stop itself",
    "settings.hangover": "Pause before auto-stop",
    "settings.hangoverDesc": "How long to wait in silence before stopping — shorter for commands, longer for connected speech",
    "settings.autostopPush": "Auto-stop in push-to-talk",
    "settings.autostopPushDesc": "In addition to releasing the key",
    "settings.autostopToggle": "Auto-stop in toggle",
    "settings.autostopToggleDesc": "Don't wait for a second press",
    "settings.denoise": "Noise reduction",
    "settings.denoiseDesc": "Cleans up your voice before recognition — easier for the model, fewer errors in noise",
    "settings.textInsertion": "Text insertion",
    "settings.autopaste": "Auto-paste into the focused field",
    "settings.autopasteDesc": "If unavailable — text goes to the clipboard",
    "settings.inputDevice": "Input device",
    "settings.microphone": "Microphone",
    "settings.microphoneDesc": "Which device to listen to",
    "settings.systemDefault": "System default",
    "settings.theme": "Theme",
    "settings.themeLabel": "Color palette",
    "settings.themeDesc": "Cream & terracotta, mint, or lavender",
    "settings.model": "Recognition model",
    "settings.customModelHint": "Add your own sherpa-onnx-compatible model from Hugging Face",
    "settings.customModelPlaceholder": "e.g. Smirnov75/GigaAM-v3-sherpa-onnx",
    "settings.findFiles": "Find files",
    "sensitivity.whisper": "Whisper-quiet",
    "sensitivity.quietRoom": "Quiet room",
    "sensitivity.normal": "Normal conversation",
    "sensitivity.cafe": "Noisy café",
    "sensitivity.openSpace": "Open-plan office",
    "sensitivity.construction": "Construction site",
    "models.active": "Active",
    "models.use": "Use",
    "models.download": "Download",
    "models.downloading": "Downloading…",
    "models.retryError": "Error — retry",
    "models.remove": "Remove",
    "models.punctuation": "punctuation",
    "models.noPunctuation": "no punctuation",
    "models.measuredNote": "Measured locally, may differ on your machine",
    "models.speedFactor": "realtime",
    "models.loadTime": "load",
    "hf.searching": "Looking up files…",
    "hf.searchError": "Couldn't fetch the file list",
    "hf.modelType": "Model type",
    "hf.ctcOneFile": "CTC (single file)",
    "hf.transducerThreeFiles": "Transducer (3 files)",
    "hf.addAndDownload": "Add and download",
    "hf.adding": "Adding…",
    "hf.tokens": "Tokens file",
    "hf.model": "Model (CTC)",
    "hf.encoder": "Encoder",
    "hf.decoder": "Decoder",
    "hf.joiner": "Joiner",
  },
};

let currentLang: Lang = (localStorage.getItem("dikta_lang") as Lang) || "ru";

function t(key: string): string {
  return TRANSLATIONS[currentLang][key] ?? TRANSLATIONS.ru[key] ?? key;
}

function applyStaticI18n() {
  document.documentElement.lang = currentLang;
  document.querySelectorAll<HTMLElement>("[data-i18n]").forEach((el) => {
    el.textContent = t(el.dataset.i18n!);
  });
  document.querySelectorAll<HTMLElement>("[data-i18n-title]").forEach((el) => {
    el.title = t(el.dataset.i18nTitle!);
  });
  document.querySelectorAll<HTMLInputElement>("[data-i18n-placeholder]").forEach((el) => {
    el.placeholder = t(el.dataset.i18nPlaceholder!);
  });
  const toggle = document.getElementById("lang-toggle")!;
  toggle.textContent = currentLang.toUpperCase();
}

async function setLanguage(lang: Lang) {
  currentLang = lang;
  localStorage.setItem("dikta_lang", lang);
  applyStaticI18n();
  // Динамически отрисованные куски нужно перерисовать явно — они не в DOM
  // на момент applyStaticI18n, их разметка генерируется через innerHTML.
  if (currentSettings) {
    updateSensitivityLabel(currentSettings.vad_sensitivity);
    document.getElementById("silence-hangover-label")!.textContent = formatSeconds(currentSettings.silence_hangover_ms);
    await loadInputDevices();
    await loadModels();
  }
  const activeView = document.querySelector<HTMLElement>(".panel.active")?.id;
  if (activeView === "view-history") loadHistory((document.getElementById("history-search") as HTMLInputElement).value);
  if (activeView === "view-stats") loadStats();
}

document.getElementById("lang-toggle")!.addEventListener("click", () => {
  setLanguage(currentLang === "ru" ? "en" : "ru");
});

applyStaticI18n();

// ---------- Типы, зеркалящие Rust-структуры ----------

interface AppSettings {
  hotkey_push_to_talk: string;
  hotkey_toggle: string;
  vad_sensitivity: number;
  silence_hangover_ms: number;
  autostop_push_to_talk: boolean;
  autostop_toggle: boolean;
  autopaste_enabled: boolean;
  denoise_enabled: boolean;
  active_model_id: string;
  input_device: string | null;
  theme: string;
}

interface ModelListItem {
  source: "Builtin" | "Custom";
  id: string;
  name: string;
  description?: string;
  kind: "Ctc" | "Transducer" | "Whisper" | "WhisperCpp";
  size_mb?: number;
  repo_id?: string;
  languages?: string[];
  punctuation?: boolean;
  measured_rtf?: number;
  measured_load_ms?: number;
  downloaded: boolean;
  active: boolean;
}

interface Recording {
  id: number;
  text: string;
  created_at: string;
  duration_ms: number;
  confidence_avg: number;
  word_count: number;
  mode: string;
}

interface OverallStats {
  total_ms_today: number;
  total_ms_week: number;
  total_ms_all: number;
  words_today: number;
  words_all: number;
  sessions_today: number;
  sessions_all: number;
}

interface DayStat {
  day: string;
  total_ms: number;
  words: number;
  sessions: number;
  confidence_avg: number;
}

// ---------- Навигация по вкладкам ----------

const views = ["home", "history", "stats", "settings"] as const;
type View = (typeof views)[number];

function switchView(view: View) {
  for (const v of views) {
    document.getElementById(`view-${v}`)?.classList.toggle("active", v === view);
  }
  document.querySelectorAll<HTMLButtonElement>("nav.rail button[data-view]").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.view === view);
  });
  if (view === "history") loadHistory("");
  if (view === "stats") loadStats();
}

document.querySelectorAll<HTMLButtonElement>("nav.rail button[data-view]").forEach((btn) => {
  btn.addEventListener("click", () => switchView(btn.dataset.view as View));
});

// ---------- Живая волна записи ----------

const waveCanvas = document.getElementById("wave") as HTMLCanvasElement;
const waveCtx = waveCanvas.getContext("2d")!;
let waveHistory: number[] = new Array(48).fill(0);
let isRecording = false;

function drawWave() {
  const w = waveCanvas.width;
  const h = waveCanvas.height;
  waveCtx.clearRect(0, 0, w, h);
  const barW = w / waveHistory.length;
  waveCtx.fillStyle = isRecording ? "#c9795f" : "#e8dcc5";
  waveHistory.forEach((level, i) => {
    const barH = Math.max(3, level * h * 0.9);
    waveCtx.fillRect(i * barW + 1, (h - barH) / 2, barW - 2, barH);
  });
  requestAnimationFrame(drawWave);
}
requestAnimationFrame(drawWave);

function pushLevel(level: number) {
  waveHistory.push(level);
  waveHistory.shift();
}

// ---------- События из бэкенда ----------

const recDot = document.getElementById("rec-dot")!;
const statusHeadline = document.getElementById("status-headline")!;
const statusHint = document.getElementById("status-hint")!;
const lastText = document.getElementById("last-text")!;

listen("no-model-active", () => {
  statusHeadline.textContent = t("home.noModel");
  statusHint.textContent = t("home.noModelHint");
});

listen<boolean>("recording-started", (event) => {
  isRecording = true;
  recDot.classList.add("live");
  statusHeadline.textContent = t("home.listening");
  statusHint.textContent = event.payload ? t("home.modeToggle") : t("home.modePush");
  lastText.textContent = "…";
});

listen<string>("partial-transcript", (event) => {
  lastText.textContent = event.payload;
});

listen("recording-stopped", () => {
  isRecording = false;
  recDot.classList.remove("live");
  statusHeadline.textContent = t("home.recognizing");
  waveHistory = waveHistory.map(() => 0);
});

listen<number>("audio-level", (event) => {
  pushLevel(event.payload);
});

listen<{ text: string; outcome: string }>("transcription-done", (event) => {
  statusHeadline.textContent = t("home.ready");
  statusHint.textContent = event.payload.outcome === "AutoInserted" ? t("home.autoInserted") : t("home.copiedToClipboard");
  lastText.textContent = event.payload.text;
});

// ---------- Настройки ----------

let currentSettings: AppSettings;

async function loadSettings() {
  currentSettings = await invoke<AppSettings>("get_settings");
  renderSettings();
  applyTheme(currentSettings.theme);
  await loadInputDevices();
  await loadModels();
}

function formatSeconds(ms: number): string {
  return `${(ms / 1000).toFixed(1)}${currentLang === "ru" ? "с" : "s"}`;
}

function renderSettings() {
  (document.getElementById("hotkey-push") as HTMLButtonElement).textContent = formatAccelerator(currentSettings.hotkey_push_to_talk);
  (document.getElementById("hotkey-toggle") as HTMLButtonElement).textContent = formatAccelerator(currentSettings.hotkey_toggle);
  const vadSlider = document.getElementById("vad-sensitivity") as HTMLInputElement;
  vadSlider.value = String(Math.round(currentSettings.vad_sensitivity * 100));
  updateSensitivityLabel(currentSettings.vad_sensitivity);
  const hangoverSlider = document.getElementById("silence-hangover") as HTMLInputElement;
  hangoverSlider.value = String(currentSettings.silence_hangover_ms);
  document.getElementById("silence-hangover-label")!.textContent = formatSeconds(currentSettings.silence_hangover_ms);
  (document.getElementById("autostop-push") as HTMLInputElement).checked = currentSettings.autostop_push_to_talk;
  (document.getElementById("autostop-toggle") as HTMLInputElement).checked = currentSettings.autostop_toggle;
  (document.getElementById("autopaste") as HTMLInputElement).checked = currentSettings.autopaste_enabled;
  (document.getElementById("denoise") as HTMLInputElement).checked = currentSettings.denoise_enabled;
  document.querySelectorAll<HTMLButtonElement>("#theme-picker button").forEach((btn) => {
    btn.classList.toggle("active", btn.dataset.theme === currentSettings.theme);
  });
}

// ---------- Тема ----------

function applyTheme(theme: string) {
  document.documentElement.setAttribute("data-theme", theme);
}

document.querySelectorAll<HTMLButtonElement>("#theme-picker button").forEach((btn) => {
  btn.addEventListener("click", async () => {
    const theme = btn.dataset.theme!;
    applyTheme(theme);
    document.querySelectorAll<HTMLButtonElement>("#theme-picker button").forEach((b) => b.classList.toggle("active", b === btn));
    await setSetting("theme", theme);
  });
});

// ---------- Устройство ввода ----------

async function loadInputDevices() {
  const devices = await invoke<string[]>("list_input_devices");
  const select = document.getElementById("input-device") as HTMLSelectElement;
  select.innerHTML = "";
  const defaultOpt = document.createElement("option");
  defaultOpt.value = "";
  defaultOpt.textContent = t("settings.systemDefault");
  select.appendChild(defaultOpt);
  for (const name of devices) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = name;
    select.appendChild(opt);
  }
  select.value = currentSettings.input_device ?? "";
}

document.getElementById("input-device")!.addEventListener("change", async (e) => {
  const value = (e.target as HTMLSelectElement).value;
  await setSetting("input_device", value);
});

// ---------- Модели распознавания ----------

let allModels: ModelListItem[] = [];
let modelLangFilter = "all";
let modelKindFilter: "all" | "Ctc" | "Transducer" | "Whisper" | "WhisperCpp" = "all";

function renderModelFilters() {
  const container = document.getElementById("model-filters")!;
  const langs = new Set<string>();
  const kinds = new Set<"Ctc" | "Transducer" | "Whisper" | "WhisperCpp">();
  for (const m of allModels) {
    kinds.add(m.kind);
    for (const l of m.languages ?? []) {
      if (/^[a-z]{2}$/i.test(l)) langs.add(l.toLowerCase());
    }
  }
  const kindLabels: Record<string, string> = {
    all: currentLang === "ru" ? "Все типы" : "All types",
    Ctc: "CTC",
    Transducer: "Transducer",
    Whisper: "Whisper (ONNX)",
    WhisperCpp: "Whisper (whisper.cpp)",
  };
  const allLangsLabel = currentLang === "ru" ? "Все языки" : "All languages";

  const langOptions = ["all", ...Array.from(langs).sort()]
    .map((l) => `<option value="${l}"${modelLangFilter === l ? " selected" : ""}>${l === "all" ? allLangsLabel : l.toUpperCase()}</option>`)
    .join("");
  const kindOptions = ["all", ...Array.from(kinds)]
    .map((k) => `<option value="${k}"${modelKindFilter === k ? " selected" : ""}>${kindLabels[k]}</option>`)
    .join("");

  container.innerHTML = `
    <select class="filter-select" id="model-filter-lang">${langOptions}</select>
    <select class="filter-select" id="model-filter-kind">${kindOptions}</select>
  `;

  container.querySelector<HTMLSelectElement>("#model-filter-lang")!.addEventListener("change", (e) => {
    modelLangFilter = (e.target as HTMLSelectElement).value;
    renderModelList();
  });
  container.querySelector<HTMLSelectElement>("#model-filter-kind")!.addEventListener("change", (e) => {
    modelKindFilter = (e.target as HTMLSelectElement).value as typeof modelKindFilter;
    renderModelList();
  });
}

async function loadModels() {
  allModels = await invoke<ModelListItem[]>("list_models");
  renderModelFilters();
  renderModelList();
}

function renderModelList() {
  const models = allModels.filter((m) => {
    if (modelKindFilter !== "all" && m.kind !== modelKindFilter) return false;
    if (modelLangFilter !== "all" && !(m.languages ?? []).map((l) => l.toLowerCase()).includes(modelLangFilter)) return false;
    return true;
  });
  const list = document.getElementById("model-list")!;
  list.innerHTML = "";
  if (models.length === 0) {
    list.innerHTML = `<div class="empty-state">${t("history.empty")}</div>`;
    return;
  }
  for (const m of models) {
    const el = document.createElement("div");
    el.className = "model-item" + (m.active ? " active" : "");
    const sizeLabel = m.size_mb ? `${m.size_mb}${currentLang === "ru" ? "МБ" : "MB"}` : "";
    const desc = m.source === "Custom" ? `HF: ${m.repo_id}` : m.description ?? "";
    const kindLabel = m.kind === "Ctc" ? "CTC" : m.kind === "Transducer" ? "Transducer" : m.kind === "WhisperCpp" ? "Whisper.cpp" : "Whisper";

    const badges: string[] = [`<span class="badge">${kindLabel}</span>`];
    if (m.languages?.length) {
      badges.push(`<span class="badge">${m.languages.join("/").toUpperCase()}</span>`);
    }
    if (m.source === "Builtin") {
      badges.push(
        m.punctuation
          ? `<span class="badge badge-good">${t("models.punctuation")}</span>`
          : `<span class="badge">${t("models.noPunctuation")}</span>`
      );
    }
    if (m.measured_rtf) {
      const speedFactor = (1 / m.measured_rtf).toFixed(0);
      badges.push(`<span class="badge" title="${t("models.measuredNote")}">⚡ ×${speedFactor} ${t("models.speedFactor")}</span>`);
    }
    if (m.measured_load_ms) {
      badges.push(`<span class="badge">${t("models.loadTime")} ~${formatSeconds(m.measured_load_ms)}</span>`);
    }

    let actionHtml = "";
    if (m.active) {
      actionHtml = `<span class="small-btn primary">${t("models.active")}</span>`;
    } else if (m.downloaded) {
      actionHtml = `<button class="small-btn" data-action="activate">${t("models.use")}</button>`;
    } else {
      actionHtml = `<button class="small-btn" data-action="download">${t("models.download")}</button>`;
    }
    const removeBtn = m.source === "Custom" ? `<button class="icon-btn" data-action="remove" title="${t("models.remove")}">✕</button>` : "";

    el.innerHTML = `
      <div class="info">
        <div class="name">${escapeHtml(m.name)}</div>
        <div class="desc">${escapeHtml(desc)}</div>
        <div class="badge-row">${badges.join("")}</div>
        <div class="progress-bar" data-progress-for="${m.id}" style="display:none;"><div class="fill"></div></div>
      </div>
      <span class="size">${sizeLabel}</span>
      ${actionHtml}
      ${removeBtn}
    `;

    el.querySelector('[data-action="download"]')?.addEventListener("click", async (ev) => {
      const btn = ev.target as HTMLButtonElement;
      btn.textContent = t("models.downloading");
      btn.setAttribute("disabled", "true");
      const bar = el.querySelector(`[data-progress-for="${m.id}"]`) as HTMLElement;
      bar.style.display = "block";
      try {
        await invoke("download_model", { id: m.id });
        await loadModels();
      } catch (err) {
        btn.textContent = t("models.retryError");
        btn.removeAttribute("disabled");
        console.error(err);
      }
    });
    el.querySelector('[data-action="activate"]')?.addEventListener("click", async () => {
      await invoke("set_active_model", { id: m.id });
      await loadSettings();
    });
    el.querySelector('[data-action="remove"]')?.addEventListener("click", async () => {
      await invoke("remove_custom_model", { id: m.id });
      await loadModels();
    });

    list.appendChild(el);
  }
}

listen<{ id: string; downloaded_bytes: number; total_bytes: number }>("model-download-progress", (event) => {
  const { id, downloaded_bytes, total_bytes } = event.payload;
  const bar = document.querySelector(`[data-progress-for="${id}"] .fill`) as HTMLElement | null;
  if (bar && total_bytes > 0) {
    bar.style.width = `${Math.min(100, (downloaded_bytes / total_bytes) * 100)}%`;
  }
});

// ---------- Добавление своей модели с Hugging Face ----------

function roleLabel(role: string): string {
  return t(`hf.${role}`);
}

function guessRole(filename: string): "tokens" | "model" | "encoder" | "decoder" | "joiner" | null {
  const f = filename.toLowerCase();
  if (f.endsWith(".txt") && f.includes("token")) return "tokens";
  if (!f.endsWith(".onnx")) return null;
  if (f.includes("encoder")) return "encoder";
  if (f.includes("decoder")) return "decoder";
  if (f.includes("joiner") || f.includes("joint")) return "joiner";
  if (f.includes("ctc")) return "model";
  return null;
}

document.getElementById("hf-list-btn")!.addEventListener("click", async () => {
  const repoId = (document.getElementById("hf-repo-id") as HTMLInputElement).value.trim();
  const container = document.getElementById("hf-file-mapping")!;
  if (!repoId) return;
  container.innerHTML = `<div class="empty-state">${t("hf.searching")}</div>`;
  let files: string[];
  try {
    files = await invoke<string[]>("hf_list_files", { repoId });
  } catch (err) {
    container.innerHTML = `<div class="empty-state">${t("hf.searchError")}: ${escapeHtml(String(err))}</div>`;
    return;
  }

  const hasTransducerHint = files.some((f) => guessRole(f) === "encoder");
  const kind: "Ctc" | "Transducer" = hasTransducerHint ? "Transducer" : "Ctc";

  container.innerHTML = `
    <div class="field-row" style="border-bottom:none; padding-top:4px;">
      <div class="field-label">${t("hf.modelType")}</div>
      <select id="hf-kind" class="select-field" style="min-width:160px;">
        <option value="Ctc" ${kind === "Ctc" ? "selected" : ""}>${t("hf.ctcOneFile")}</option>
        <option value="Transducer" ${kind === "Transducer" ? "selected" : ""}>${t("hf.transducerThreeFiles")}</option>
      </select>
    </div>
    <div id="hf-roles"></div>
    <button class="small-btn primary" id="hf-add-btn" style="margin-top:10px;">${t("hf.addAndDownload")}</button>
  `;

  function renderRoleRows(activeKind: "Ctc" | "Transducer") {
    const activeRoles = activeKind === "Ctc" ? (["tokens", "model"] as const) : (["tokens", "encoder", "decoder", "joiner"] as const);
    const rolesEl = document.getElementById("hf-roles")!;
    rolesEl.innerHTML = activeRoles
      .map((role) => {
        const guessed = files.find((f) => guessRole(f) === role) ?? "";
        const options = files.map((f) => `<option value="${escapeHtml(f)}" ${f === guessed ? "selected" : ""}>${escapeHtml(f)}</option>`).join("");
        return `<div class="hf-file-row"><span style="min-width:110px;">${roleLabel(role)}</span><select data-role="${role}">${options}</select></div>`;
      })
      .join("");
  }
  renderRoleRows(kind);

  document.getElementById("hf-kind")!.addEventListener("change", (e) => {
    renderRoleRows((e.target as HTMLSelectElement).value as "Ctc" | "Transducer");
  });

  document.getElementById("hf-add-btn")!.addEventListener("click", async () => {
    const selectedKind = (document.getElementById("hf-kind") as HTMLSelectElement).value as "Ctc" | "Transducer";
    const roleSelects = document.querySelectorAll<HTMLSelectElement>("#hf-roles select[data-role]");
    const values: Record<string, string> = {};
    roleSelects.forEach((sel) => (values[sel.dataset.role!] = sel.value));

    const addBtn = document.getElementById("hf-add-btn") as HTMLButtonElement;
    addBtn.textContent = t("hf.adding");
    addBtn.setAttribute("disabled", "true");
    try {
      const id = await invoke<string>("add_custom_model", {
        repoId,
        kind: selectedKind,
        tokens: values.tokens,
        model: selectedKind === "Ctc" ? values.model : undefined,
        encoder: selectedKind === "Transducer" ? values.encoder : undefined,
        decoder: selectedKind === "Transducer" ? values.decoder : undefined,
        joiner: selectedKind === "Transducer" ? values.joiner : undefined,
      });
      await invoke("download_model", { id });
      container.innerHTML = "";
      (document.getElementById("hf-repo-id") as HTMLInputElement).value = "";
      await loadModels();
    } catch (err) {
      addBtn.textContent = t("models.retryError");
      addBtn.removeAttribute("disabled");
      console.error(err);
    }
  });
});

const SENSITIVITY_KEYS = [
  "sensitivity.whisper",
  "sensitivity.quietRoom",
  "sensitivity.normal",
  "sensitivity.cafe",
  "sensitivity.openSpace",
  "sensitivity.construction",
] as const;

/** Игривые подписи чувствительности считаются на фронте (не через Rust-команду),
 * чтобы переключение языка не требовало похода в бэкенд за переводом. */
function sensitivityLabel(value: number): string {
  const pct = Math.round(value * 100);
  const idx = pct <= 15 ? 0 : pct <= 35 ? 1 : pct <= 55 ? 2 : pct <= 75 ? 3 : pct <= 90 ? 4 : 5;
  return t(SENSITIVITY_KEYS[idx]);
}

function updateSensitivityLabel(value: number) {
  document.getElementById("vad-sensitivity-label")!.textContent = sensitivityLabel(value);
}

async function setSetting(key: string, value: string) {
  await invoke("set_setting", { key, value });
}

document.getElementById("vad-sensitivity")!.addEventListener("input", async (e) => {
  const raw = Number((e.target as HTMLInputElement).value);
  const value = raw / 100;
  updateSensitivityLabel(value);
  await setSetting("vad_sensitivity", String(value));
});

document.getElementById("silence-hangover")!.addEventListener("input", async (e) => {
  const ms = Number((e.target as HTMLInputElement).value);
  document.getElementById("silence-hangover-label")!.textContent = formatSeconds(ms);
  await setSetting("silence_hangover_ms", String(ms));
});

document.getElementById("autostop-push")!.addEventListener("change", (e) => {
  setSetting("autostop_push_to_talk", String((e.target as HTMLInputElement).checked));
});
document.getElementById("autostop-toggle")!.addEventListener("change", (e) => {
  setSetting("autostop_toggle", String((e.target as HTMLInputElement).checked));
});
document.getElementById("denoise")!.addEventListener("change", (e) => {
  setSetting("denoise_enabled", String((e.target as HTMLInputElement).checked));
});
document.getElementById("autopaste")!.addEventListener("change", (e) => {
  setSetting("autopaste_enabled", String((e.target as HTMLInputElement).checked));
});

// ---------- Захват хоткея ----------

function eventToAccelerator(e: KeyboardEvent): string | null {
  const mods: string[] = [];
  if (e.metaKey || e.ctrlKey) mods.push("CmdOrCtrl");
  if (e.shiftKey) mods.push("Shift");
  if (e.altKey) mods.push("Alt");

  const code = e.code;
  let key: string | null = null;
  if (/^Key[A-Z]$/.test(code)) key = code.slice(3);
  else if (/^Digit[0-9]$/.test(code)) key = code.slice(5);
  else if (/^F[0-9]{1,2}$/.test(code)) key = code;
  else if (code === "Space") key = "Space";
  else if (code === "Escape") return "__CANCEL__";
  else if (["ControlLeft", "ControlRight", "ShiftLeft", "ShiftRight", "AltLeft", "AltRight", "MetaLeft", "MetaRight"].includes(code)) {
    return null; // ждём основную клавишу
  } else key = code;

  if (!key) return null;
  return mods.length ? `${mods.join("+")}+${key}` : key;
}

function bindHotkeyCapture(buttonId: string, settingsKey: keyof AppSettings) {
  const btn = document.getElementById(buttonId) as HTMLButtonElement;
  btn.addEventListener("click", () => {
    btn.classList.add("listening");
    const original = btn.textContent;
    btn.textContent = t("settings.pressCombo");

    const onKeyDown = async (e: KeyboardEvent) => {
      e.preventDefault();
      const accel = eventToAccelerator(e);
      if (accel === null) return;
      window.removeEventListener("keydown", onKeyDown, true);
      btn.classList.remove("listening");
      if (accel === "__CANCEL__") {
        btn.textContent = original;
        return;
      }
      btn.textContent = formatAccelerator(accel);
      await setSetting(settingsKey, accel);
      await loadSettings();
    };
    window.addEventListener("keydown", onKeyDown, true);
  });
}

bindHotkeyCapture("hotkey-push", "hotkey_push_to_talk");
bindHotkeyCapture("hotkey-toggle", "hotkey_toggle");

// ---------- История ----------

function formatDate(iso: string): string {
  const d = new Date(iso);
  const locale = currentLang === "ru" ? "ru-RU" : "en-US";
  return d.toLocaleString(locale, { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function qualityColor(q: number): string {
  if (q > 0.6) return "#82a075";
  if (q > 0.3) return "#cb9550";
  return "#c9795f";
}

async function loadHistory(query: string) {
  const items = await invoke<Recording[]>("search_history", { query, limit: 200 });
  const list = document.getElementById("history-list")!;
  list.innerHTML = "";
  if (items.length === 0) {
    list.innerHTML = `<div class="empty-state">${t("history.empty")}</div>`;
    return;
  }
  for (const item of items) {
    const el = document.createElement("div");
    el.className = "history-item";
    el.innerHTML = `
      <div class="text">
        ${escapeHtml(item.text)}
        <div class="meta">
          <span class="quality-dot" style="background:${qualityColor(item.confidence_avg)}"></span>
          <span>${formatDate(item.created_at)}</span>
          <span>${formatSeconds(item.duration_ms)}</span>
          <span>${item.word_count} ${t("history.words")}</span>
        </div>
      </div>
      <div class="actions">
        <button class="icon-btn" data-action="copy" title="${t("history.copy")}">⧉</button>
        <button class="icon-btn" data-action="delete" title="${t("history.delete")}">✕</button>
      </div>
    `;
    el.querySelector('[data-action="copy"]')!.addEventListener("click", () => {
      navigator.clipboard.writeText(item.text);
    });
    el.querySelector('[data-action="delete"]')!.addEventListener("click", async () => {
      await invoke("delete_history_item", { id: item.id });
      loadHistory((document.getElementById("history-search") as HTMLInputElement).value);
    });
    list.appendChild(el);
  }
}

function escapeHtml(s: string): string {
  const div = document.createElement("div");
  div.textContent = s;
  return div.innerHTML;
}

let searchDebounce: number | undefined;
document.getElementById("history-search")!.addEventListener("input", (e) => {
  window.clearTimeout(searchDebounce);
  const query = (e.target as HTMLInputElement).value;
  searchDebounce = window.setTimeout(() => loadHistory(query), 200);
});

// ---------- Статистика ----------

function formatDuration(ms: number): string {
  const totalSec = Math.round(ms / 1000);
  const m = Math.floor(totalSec / 60);
  const s = totalSec % 60;
  return currentLang === "ru" ? `${m}м ${s}с` : `${m}m ${s}s`;
}

async function loadStats() {
  const overall = await invoke<OverallStats>("get_overall_stats");
  const grid = document.getElementById("stat-grid")!;
  grid.innerHTML = `
    <div class="stat-card"><div class="value">${formatDuration(overall.total_ms_today)}</div><div class="label">${t("stats.todayTime")}</div></div>
    <div class="stat-card"><div class="value">${formatDuration(overall.total_ms_week)}</div><div class="label">${t("stats.weekTime")}</div></div>
    <div class="stat-card"><div class="value">${overall.words_today}</div><div class="label">${t("stats.wordsToday")}</div></div>
    <div class="stat-card"><div class="value">${overall.sessions_all}</div><div class="label">${t("stats.sessionsAll")}</div></div>
  `;

  const daily = await invoke<DayStat[]>("get_daily_stats", { days: 30 });
  renderTimeChart(daily);
  renderQualityChart(daily);
}

function renderTimeChart(daily: DayStat[]) {
  const el = document.getElementById("chart-time")!;
  el.innerHTML = "";
  if (daily.length === 0) {
    el.innerHTML = `<div class="empty-state">${t("stats.empty")}</div>`;
    return;
  }
  const xs = daily.map((d) => Date.parse(d.day) / 1000);
  const ys = daily.map((d) => d.total_ms / 60000);
  new UPlot(
    {
      width: el.clientWidth || 640,
      height: 200,
      series: [{}, { label: t("stats.minutesSeries"), stroke: "#c9795f", fill: "rgba(201,121,95,0.15)", width: 2 }],
      axes: [{ stroke: "#6b5c4e" }, { stroke: "#6b5c4e" }],
      scales: { x: { time: true } },
    },
    [xs, ys],
    el
  );
}

function renderQualityChart(daily: DayStat[]) {
  const el = document.getElementById("chart-quality")!;
  el.innerHTML = "";
  if (daily.length === 0) {
    el.innerHTML = `<div class="empty-state">${t("stats.empty")}</div>`;
    return;
  }
  const xs = daily.map((d) => Date.parse(d.day) / 1000);
  const ys = daily.map((d) => d.confidence_avg);
  new UPlot(
    {
      width: el.clientWidth || 640,
      height: 160,
      series: [{}, { label: t("stats.qualitySeries"), stroke: "#a1503a", width: 2 }],
      axes: [{ stroke: "#6b5c4e" }, { stroke: "#6b5c4e" }],
      scales: { x: { time: true }, y: { range: [0, 1] } },
    },
    [xs, ys],
    el
  );
}

// ---------- Инициализация ----------

loadSettings();
