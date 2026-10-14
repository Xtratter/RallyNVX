"""Renders the co-driver clips (tools/voices/clips.json) with the free Piper voices of voices.json into
app/src/main/assets/voices/<voice id>/<clip id>.wav (16 kHz, mono, 16-bit) and writes assets/voices/index.json.
Run in CI (Piper needs onnxruntime, which Termux lacks): see tools/voices/README.md."""
import json, os, subprocess, sys, urllib.request, wave
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
OUT = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "..", "..", "app", "src", "main", "assets", "voices")
clips = json.load(open(os.path.join(HERE, "clips.json"), encoding="utf-8"))
catalog = json.load(open(os.path.join(HERE, "voices.json"), encoding="utf-8"))["voices"]
base = "https://huggingface.co/rhasspy/piper-voices/resolve/main/"
index = json.load(urllib.request.urlopen(base + "voices.json"))
os.makedirs("models", exist_ok=True); os.makedirs(OUT, exist_ok=True)

def read_wav(path):
    with wave.open(path) as w:
        data = np.frombuffer(w.readframes(w.getnframes()), dtype=np.int16).astype(np.float32) / 32768.0
        return data, w.getframerate()

def finish(x, rate, dst):
    """Trims the silence, matches the loudness (RMS) across clips, fades the edges, resamples to 16 kHz and writes the wav."""
    peak = float(np.max(np.abs(x))) or 1.0
    idx = np.where(np.abs(x) > 0.03 * peak)[0]
    if len(idx): x = x[max(0, idx[0] - int(0.02 * rate)): idx[-1] + int(0.05 * rate)]
    rms = float(np.sqrt(np.mean(x * x))) or 1e-4
    x = np.tanh(x * (0.11 / rms) * 1.0) * 0.95   # a soft limiter keeps the peaks of a loud clip in range
    n = int(0.008 * rate); x[:n] *= np.linspace(0, 1, n); x[-n:] *= np.linspace(1, 0, n)
    tmp = dst + ".tmp.wav"
    with wave.open(tmp, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(rate); w.writeframes((x * 32767).astype(np.int16).tobytes())
    subprocess.run(["ffmpeg", "-y", "-loglevel", "error", "-i", tmp, "-ar", "16000", "-ac", "1", "-sample_fmt", "s16", dst], check=True)
    os.remove(tmp)

done = []
for v in catalog:
    try:
        files = index[v["model"]]["files"]
        onnx = [f for f in files if f.endswith(".onnx")][0]
        for f in (onnx, onnx + ".json"):
            dst = os.path.join("models", os.path.basename(f))
            if not os.path.exists(dst): urllib.request.urlretrieve(base + f, dst)
        model = os.path.join("models", os.path.basename(onnx))
        vdir = os.path.join(OUT, v["id"]); os.makedirs(vdir, exist_ok=True)
        for cid, text in clips[v["lang"]].items():
            raw = os.path.join("models", "raw.wav")
            cmd = ["piper", "-m", model, "-f", raw] + (["-s", str(v["speaker"])] if v["speaker"] is not None else [])
            subprocess.run(cmd, input=text.encode("utf-8"), check=True, capture_output=True)
            x, rate = read_wav(raw)
            finish(x, rate, os.path.join(vdir, cid + ".wav"))
        done.append(v); print("ok", v["id"], flush=True)
    except Exception as e:
        print("FAILED", v["id"], e, flush=True)
json.dump({"voices": [{k: v[k] for k in ("id", "lang", "name", "license", "source")} for v in done]}, open(os.path.join(OUT, "index.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(len(done), "voices")
