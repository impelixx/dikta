# Дикта

Локальная диктовка на русском: хоткей → распознавание речи GigaAM-CTC (Sber) полностью
на устройстве, без облака. Tauri (Rust) + тонкий TS-фронт, тема — тёплый крем и
приглушённая терракота, Lora + JetBrains Mono.

## Возможности

- Push-to-talk и toggle хоткеи (настраиваются в приложении)
- Автостоп записи по тишине с регулируемой чувствительностью
- Автовставка текста в текущее поле, fallback в буфер обмена
- История распознаваний: поиск, копирование, удаление
- Статистика: время диктовки, слова, сессии, графики по дням

## Запуск

```bash
./scripts/download-model.sh   # один раз: скачивает GigaAM-CTC (~200МБ)
npm install
npm run tauri dev
```

На macOS при первом запуске система запросит доступ к микрофону и, при включённой
автовставке, к Accessibility (System Settings → Privacy & Security).

## Модель

Используется готовая ONNX-версия [GigaAM-CTC](https://github.com/salute-developers/GigaAM)
из [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) model zoo. Лицензия модели —
некоммерческая (GigaAM License NC, см. PDF внутри архива модели).

## Стек

Rust (tauri 2, sherpa-rs, cpal, rusqlite, enigo) + TypeScript (без фреймворка, uPlot
для графиков).
