# Co-driver voices

`clips.json` lists the clips (id and spoken text, Russian and English) the pace notes are made of (`rust/src/game/pacenotes.rs`, `Note::clips`);
`voices.json` lists the free voices to render; `build_voices.py` renders them with Piper into `app/src/main/assets/voices/`.
Piper needs `onnxruntime`, which is not available in Termux, so the script runs in GitHub Actions (see `.github/workflows/voices.yml` on a
throwaway branch): `pip install piper-tts==1.2.0 numpy; python tools/voices/build_voices.py out/voices`, then the artifact is copied into the assets.
Only voices with free licenses are listed (CC0, public domain, CC BY 4.0: the credits are in the README).
