#!/usr/bin/env bash
# Скачивает готовую ONNX-версию GigaAM-CTC (sherpa-onnx) для русского языка.
# Лицензия модели — некоммерческая (GigaAM License NC), см. PDF внутри архива.
set -euo pipefail

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)/src-tauri/resources/model"
URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-ctc-giga-am-russian-2024-10-24.tar.bz2"

mkdir -p "$DIR"
cd "$DIR"

if [ -f "sherpa-onnx-nemo-ctc-giga-am-russian-2024-10-24/model.int8.onnx" ]; then
  echo "Модель уже скачана: $DIR"
  exit 0
fi

echo "Скачиваю GigaAM-CTC (~200МБ)..."
curl -L -o giga-am.tar.bz2 "$URL"
tar xjf giga-am.tar.bz2
rm giga-am.tar.bz2
echo "Готово: $DIR/sherpa-onnx-nemo-ctc-giga-am-russian-2024-10-24"
