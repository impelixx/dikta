import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Оверлей — отдельное окно/бандл, поэтому у него свой маленький словарь;
// язык читается из того же localStorage-ключа, что и в главном окне.
type Lang = "ru" | "en";
const lang: Lang = (localStorage.getItem("dikta_lang") as Lang) || "ru";
const STRINGS: Record<Lang, Record<string, string>> = {
  ru: {
    listening: "Слушаю…",
    recognizing: "Распознаю…",
    autoInserted: "✓",
    copied: "⧉ скопировано:",
    noModel: "Модель не выбрана — откройте Настройки",
  },
  en: {
    listening: "Listening…",
    recognizing: "Recognizing…",
    autoInserted: "✓",
    copied: "⧉ copied:",
    noModel: "No model selected — open Settings",
  },
};
const s = STRINGS[lang];

const win = getCurrentWindow();
const pill = document.getElementById("pill")!;
const text = document.getElementById("pill-text")!;
const waveCanvas = document.getElementById("pill-wave") as HTMLCanvasElement;
const waveCtx = waveCanvas.getContext("2d")!;

let waveHistory: number[] = new Array(14).fill(0);
let waving = false;

function setState(state: "listening" | "processing" | "done" | "warn") {
  pill.className = `pill ${state === "listening" ? "" : state}`.trim();
}

function drawWave() {
  const w = waveCanvas.width;
  const h = waveCanvas.height;
  waveCtx.clearRect(0, 0, w, h);
  if (waving) {
    const barW = w / waveHistory.length;
    waveCtx.fillStyle = "#f6eee2";
    waveHistory.forEach((level, i) => {
      const barH = Math.max(2, level * h * 0.9);
      waveCtx.fillRect(i * barW + 1, (h - barH) / 2, barW - 2, barH);
    });
  }
  requestAnimationFrame(drawWave);
}
requestAnimationFrame(drawWave);

let hideTimer: number | undefined;
function scheduleHide(delayMs: number) {
  window.clearTimeout(hideTimer);
  hideTimer = window.setTimeout(() => {
    win.hide();
  }, delayMs);
}

listen("recording-started", () => {
  window.clearTimeout(hideTimer);
  waving = true;
  setState("listening");
  text.textContent = s.listening;
  win.show();
});

listen<string>("partial-transcript", (event) => {
  text.textContent = event.payload;
});

listen<number>("audio-level", (event) => {
  waveHistory.push(event.payload);
  waveHistory.shift();
});

listen("recording-stopped", () => {
  waving = false;
  waveHistory = waveHistory.map(() => 0);
  setState("processing");
  text.textContent = s.recognizing;
});

listen<{ text: string; outcome: string }>("transcription-done", (event) => {
  setState("done");
  const snippet = event.payload.text.length > 40 ? event.payload.text.slice(0, 40) + "…" : event.payload.text;
  text.textContent = event.payload.outcome === "AutoInserted" ? `${s.autoInserted} ${snippet}` : `${s.copied} ${snippet}`;
  scheduleHide(1600);
});

listen("no-model-active", () => {
  setState("warn");
  text.textContent = s.noModel;
  scheduleHide(2200);
});
