import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

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
    waveCtx.fillStyle = "#f4ead9";
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
  text.textContent = "Слушаю…";
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
  text.textContent = "Распознаю…";
});

listen<{ text: string; outcome: string }>("transcription-done", (event) => {
  setState("done");
  const snippet = event.payload.text.length > 40 ? event.payload.text.slice(0, 40) + "…" : event.payload.text;
  text.textContent = event.payload.outcome === "AutoInserted" ? `✓ ${snippet}` : `⧉ скопировано: ${snippet}`;
  scheduleHide(1600);
});

listen("no-model-active", () => {
  setState("warn");
  text.textContent = "Модель не выбрана — откройте Настройки";
  scheduleHide(2200);
});
