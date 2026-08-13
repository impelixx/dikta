import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import UPlot from "uplot";
import "uplot/dist/uPlot.min.css";

// ---------- Типы, зеркалящие Rust-структуры ----------

interface AppSettings {
  hotkey_push_to_talk: string;
  hotkey_toggle: string;
  vad_sensitivity: number;
  silence_hangover_ms: number;
  autostop_push_to_talk: boolean;
  autostop_toggle: boolean;
  autopaste_enabled: boolean;
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
  waveCtx.fillStyle = isRecording ? "#c1694f" : "#e3d8c3";
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

listen<boolean>("recording-started", (event) => {
  isRecording = true;
  recDot.classList.add("live");
  statusHeadline.textContent = "Слушаю…";
  statusHint.textContent = event.payload ? "режим: toggle — нажмите хоткей ещё раз" : "режим: push-to-talk — держите клавишу";
});

listen("recording-stopped", () => {
  isRecording = false;
  recDot.classList.remove("live");
  statusHeadline.textContent = "Распознаю…";
  waveHistory = waveHistory.map(() => 0);
});

listen<number>("audio-level", (event) => {
  pushLevel(event.payload);
});

listen<{ text: string; outcome: string }>("transcription-done", (event) => {
  statusHeadline.textContent = "Готова слушать";
  statusHint.textContent =
    event.payload.outcome === "AutoInserted"
      ? "✓ вставлено автоматически"
      : "ℹ скопировано в буфер — вставьте вручную (Cmd/Ctrl+V)";
  lastText.textContent = event.payload.text;
});

// ---------- Настройки ----------

let currentSettings: AppSettings;

async function loadSettings() {
  currentSettings = await invoke<AppSettings>("get_settings");
  renderSettings();
}

function renderSettings() {
  (document.getElementById("hotkey-push") as HTMLButtonElement).textContent = currentSettings.hotkey_push_to_talk;
  (document.getElementById("hotkey-toggle") as HTMLButtonElement).textContent = currentSettings.hotkey_toggle;
  const vadSlider = document.getElementById("vad-sensitivity") as HTMLInputElement;
  vadSlider.value = String(Math.round(currentSettings.vad_sensitivity * 100));
  updateSensitivityLabel(currentSettings.vad_sensitivity);
  (document.getElementById("autostop-push") as HTMLInputElement).checked = currentSettings.autostop_push_to_talk;
  (document.getElementById("autostop-toggle") as HTMLInputElement).checked = currentSettings.autostop_toggle;
  (document.getElementById("autopaste") as HTMLInputElement).checked = currentSettings.autopaste_enabled;
}

async function updateSensitivityLabel(value: number) {
  const label = await invoke<string>("sensitivity_label", { value });
  document.getElementById("vad-sensitivity-label")!.textContent = label;
}

async function setSetting(key: string, value: string) {
  await invoke("set_setting", { key, value });
}

document.getElementById("vad-sensitivity")!.addEventListener("input", async (e) => {
  const raw = Number((e.target as HTMLInputElement).value);
  const value = raw / 100;
  await updateSensitivityLabel(value);
  await setSetting("vad_sensitivity", String(value));
});

document.getElementById("autostop-push")!.addEventListener("change", (e) => {
  setSetting("autostop_push_to_talk", String((e.target as HTMLInputElement).checked));
});
document.getElementById("autostop-toggle")!.addEventListener("change", (e) => {
  setSetting("autostop_toggle", String((e.target as HTMLInputElement).checked));
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
    btn.textContent = "Нажмите комбинацию…";

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
      btn.textContent = accel;
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
  return d.toLocaleString("ru-RU", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" });
}

function qualityColor(q: number): string {
  if (q > 0.6) return "#7a9471";
  if (q > 0.3) return "#c98a3a";
  return "#c1694f";
}

async function loadHistory(query: string) {
  const items = await invoke<Recording[]>("search_history", { query, limit: 200 });
  const list = document.getElementById("history-list")!;
  list.innerHTML = "";
  if (items.length === 0) {
    list.innerHTML = `<div class="empty-state">Ничего не найдено</div>`;
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
          <span>${(item.duration_ms / 1000).toFixed(1)}с</span>
          <span>${item.word_count} слов</span>
        </div>
      </div>
      <div class="actions">
        <button class="icon-btn" data-action="copy" title="Копировать">⧉</button>
        <button class="icon-btn" data-action="delete" title="Удалить">✕</button>
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
  return `${m}м ${s}с`;
}

async function loadStats() {
  const overall = await invoke<OverallStats>("get_overall_stats");
  const grid = document.getElementById("stat-grid")!;
  grid.innerHTML = `
    <div class="stat-card"><div class="value">${formatDuration(overall.total_ms_today)}</div><div class="label">Сегодня наговорено</div></div>
    <div class="stat-card"><div class="value">${formatDuration(overall.total_ms_week)}</div><div class="label">За неделю</div></div>
    <div class="stat-card"><div class="value">${overall.words_today}</div><div class="label">Слов сегодня</div></div>
    <div class="stat-card"><div class="value">${overall.sessions_all}</div><div class="label">Сессий всего</div></div>
  `;

  const daily = await invoke<DayStat[]>("get_daily_stats", { days: 30 });
  renderTimeChart(daily);
  renderQualityChart(daily);
}

function renderTimeChart(daily: DayStat[]) {
  const el = document.getElementById("chart-time")!;
  el.innerHTML = "";
  if (daily.length === 0) {
    el.innerHTML = `<div class="empty-state">Пока нет данных</div>`;
    return;
  }
  const xs = daily.map((d) => Date.parse(d.day) / 1000);
  const ys = daily.map((d) => d.total_ms / 60000);
  new UPlot(
    {
      width: el.clientWidth || 640,
      height: 200,
      series: [{}, { label: "минут", stroke: "#c1694f", fill: "rgba(193,105,79,0.15)", width: 2 }],
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
    el.innerHTML = `<div class="empty-state">Пока нет данных</div>`;
    return;
  }
  const xs = daily.map((d) => Date.parse(d.day) / 1000);
  const ys = daily.map((d) => d.confidence_avg);
  new UPlot(
    {
      width: el.clientWidth || 640,
      height: 160,
      series: [{}, { label: "качество сигнала", stroke: "#a1503a", width: 2 }],
      axes: [{ stroke: "#6b5c4e" }, { stroke: "#6b5c4e" }],
      scales: { x: { time: true }, y: { range: [0, 1] } },
    },
    [xs, ys],
    el
  );
}

// ---------- Инициализация ----------

loadSettings();
