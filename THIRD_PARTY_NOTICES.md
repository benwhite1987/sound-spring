# Third-Party Notices

Sound Spring bundles or links the following third-party components.

## ECAPA-TDNN speaker embedding

- **Source:** [vedk00/ecapa-voxceleb-speaker-embedding-onnx](https://huggingface.co/vedk00/ecapa-voxceleb-speaker-embedding-onnx)
- **Upstream:** [speechbrain/spkrec-ecapa-voxceleb](https://huggingface.co/speechbrain/spkrec-ecapa-voxceleb)
- **License:** Apache-2.0
- **Usage:** Embedded in the release binary via `include_bytes!` for speaker verification and enrollment.

## SpeechBrain fbank matrix

- **File:** `assets/models/fbank-80x201-f32.bin`
- **Source:** Same `vedk00` ONNX package (frozen mel filterbank for ECAPA preprocessing)
- **License:** Apache-2.0

## Silero VAD

- **Crate:** `voice_activity_detector` 0.2.1
- **Model:** Silero VAD ONNX (bundled by the crate)
- **License:** See crate and upstream Silero model license

## DeepFilterNet3

- **Crate:** `deep_filter` (git tag v0.5.6, Rikorose/DeepFilterNet)
- **License:** See upstream DeepFilterNet repository

## ONNX Runtime

- **Crate:** `ort` 2.0.0-rc.10
- **Usage:** ECAPA inference backend (native library linked at build time)

## Qt 6

- **Usage:** GUI framework (linked dynamically at runtime unless statically packaged by a distributor)
