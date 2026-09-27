# Live translation

Live translation is experimental. It reads text from a RetroArch game screen,
translates it locally, and draws an English overlay near the original text.
It does not modify the ROM.

It can miss text, choose awkward wording, or take time to update. It is not
a substitute for a finished community translation patch.

## What runs on your computer?

- **OCR** finds and reads the text. Lunchbox runs the downloaded PP-OCRv6
  models through ONNX Runtime.
- **Ollama** runs the TranslateGemma model that translates the recognized text.
- **RetroArch's AI Service** supplies the game image and displays the result.

Ollama does not run the OCR model. Using Docker for Ollama does not move
Lunchbox's OCR into that container.

## Requirements

You need a supported GPU setup for both OCR and translation. Lunchbox checks
them before starting and reports an error instead of silently using CPU OCR.

| Installation | GPU setup |
| --- | --- |
| Linux Nix/development build on supported AMD hardware | ROCm/MIGraphX OCR and GPU-backed Ollama |
| Windows | DirectML OCR and a compatible GPU-backed Ollama setup |
| Apple Silicon macOS | Core ML OCR and host-side Ollama |
| Linux AppImage or Flatpak | GPU OCR is not currently included; live translation is unavailable |

Hardware support is not guaranteed by the GPU's memory size alone. Use the
wizard's checks rather than assuming that a model download proves GPU support.

## Set it up

Open **Settings → Game translation → Set up GPU translation…**.

The wizard guides you through a backend, model downloads, and GPU checks.
Available translation models are TranslateGemma 4B, 12B, and 27B. Larger models
need more memory and may take longer; choose one your GPU can run comfortably.

The Docker option needs Docker already installed and running, with supported
GPU access. On Linux, the AMD route needs ROCm device access; NVIDIA needs
working GPU passthrough. Model data is kept in a persistent Docker volume.

On macOS, use host-side Ollama for Metal acceleration. Ordinary Docker Desktop
Linux containers cannot use that GPU route.

The wizard does not silently replace an existing Ollama service. Check which
service is using the configured address before starting another one.

## Turn it on for a game

Enable **Translate this game** in Game details. Lunchbox remembers the choice
for that game; other games launch without its translation bridge.

In RetroArch, press **F10 once** to start automatic updates and again to stop.
**F8 takes a screenshot**. You can also configure RetroArch's AI Service
controller hotkey.

Automatic translation is paced rather than run on every game frame. It still
takes time to capture, recognize, translate, and display a new screen.

## If it is slow or misses text

- Wait until a dialogue box has finished drawing before judging the result.
- Check that the wizard verified both GPU backends.
- Try a smaller translation model if GPU memory or inference speed is the problem.
- Turn translation off briefly to compare game performance.
- If one screen fails repeatedly, capture it for a bug report.

GPU inference can still compete with the emulator for resources. “On the GPU”
does not guarantee zero audio or video stutter.

Low-resolution fonts, animated text, complex backgrounds, and unusual layouts
can all reduce recognition quality. Background matching and font sizing are
approximate.

The normal local setup does not send game screenshots to a cloud translation
service. If you deliberately use a remote Ollama endpoint, the recognized text
is sent to that endpoint.

