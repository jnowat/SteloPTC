# Local AI Assistant — Setup & Usage

SteloPTC includes an **optional, fully local** AI assistant that helps with three
day-to-day tasks:

| Feature | What it does | Where |
|---|---|---|
| **Summarize Notes** | Condenses a specimen's free-text notes into 2–3 sentences, preserving measurements, dates, and contamination observations verbatim | Specimen Detail → notes area |
| **Suggest Passage Comment** | Drafts a factual observation for the next passage from the specimen's recent passage history | Specimen Detail → notes area |
| **Analyze Photo for Contamination** | Examines an attached photo for visible bacterial/fungal growth, discoloration, or turbidity | Photo lightbox |

Every result is a **draft**. Nothing an AI produces is ever written into a real
record automatically — a person must explicitly **approve** each suggestion,
and approval goes through the same audit-logged edit path a manual note edit
uses. The approving user (not the model) is recorded as the author, with the
model name and prompt kept alongside for full traceability.

> **The app works identically with AI turned off.** If no local runtime is
> configured or running, the AI buttons simply report that the model is
> unreachable — every manual workflow is completely unaffected.

---

## Privacy model

SteloPTC's AI is **on-device by construction**. It talks only to a local model
runtime you control — on this computer or on your own LAN. **Specimen data,
notes, and photos are never sent to any cloud AI service.** There is no
telemetry and no remote fallback: if the local runtime is down, the feature is
simply unavailable.

Two runtimes are supported, both local:

- **[Ollama](https://ollama.com)** — the default, and the simplest to set up.
- **[LocalAI](https://localai.io)** — or any OpenAI-compatible server, for labs
  that already run one.

---

## Option A — Ollama (recommended)

Ollama is a single-binary local model runtime. It is the same runtime the
[Gruper](https://github.com/jnowat/gruper) project drives, and the default for
SteloPTC.

### 1. Install Ollama

Download from **[ollama.com/download](https://ollama.com/download)** (Windows,
macOS, Linux). On Linux you can also run:

```bash
curl -fsSL https://ollama.com/install.sh | sh
```

### 2. Start the Ollama server

Ollama usually runs as a background service after install. If not, start it
manually:

```bash
ollama serve
```

It listens on `http://127.0.0.1:11434` by default — the value SteloPTC expects
out of the box.

### 3. Pull the models

SteloPTC needs a **text model** (for summaries and passage comments) and a
**vision model** (for photo analysis):

```bash
ollama pull llama3.1     # text model (default)
ollama pull llava        # vision model (default)
```

Any Ollama model works — see [Model recommendations](#model-recommendations)
below. If you change the model, update it in **Settings → AI Assistant** to
match.

### 4. Point SteloPTC at it

Open **Settings → AI Assistant** (admin/supervisor only):

- **Runtime:** `Ollama`
- **Base URL:** `http://127.0.0.1:11434`
- **Text model:** `llama3.1`
- **Vision model:** `llava`

Click **Test Connection**. You should see a green *"Reachable"* result listing
your installed models, with a ✓ next to both the text and vision model.

---

## Option B — LocalAI (OpenAI-compatible)

If you already run [LocalAI](https://localai.io) — or any server that exposes
the OpenAI API (`/v1/chat/completions` and `/v1/models`) — SteloPTC can use it
directly.

1. Start your LocalAI server (default port `8080`).
2. In **Settings → AI Assistant**:
   - **Runtime:** `LocalAI (OpenAI-compatible)`
   - **Base URL:** `http://127.0.0.1:8080`
   - **Text model / Vision model:** the model ids your server exposes (as shown
     by `GET /v1/models`).
3. Click **Test Connection** to confirm reachability and see the available
   model ids.

Vision analysis sends the image as an OpenAI-style `image_url` data URI, so the
configured vision model must be one of LocalAI's multimodal backends.

> **Tip:** Ollama also exposes an OpenAI-compatible `/v1` endpoint. If you
> prefer the OpenAI request shape, you can select the LocalAI runtime and point
> it at Ollama's `http://127.0.0.1:11434/v1`.

---

## Model recommendations

| Role | Good default | Lighter (less RAM) | Higher quality |
|---|---|---|---|
| Text | `llama3.1` (8B) | `llama3.2:3b`, `qwen2.5:3b` | `llama3.1:70b`, `qwen2.5:14b` |
| Vision | `llava` | `llava:7b` | `llama3.2-vision`, `llava:13b` |

- An **8B text model** runs comfortably on most modern laptops (≈ 8 GB RAM).
- **Vision models** are heavier; if photo analysis is slow or times out, try a
  smaller vision model or run it on a machine with more memory.
- The first request after startup is slow because the model must load into
  memory ("cold start"); subsequent requests are much faster.

---

## Configuration reference

All AI settings live in **Settings → AI Assistant** and are stored in the local
`app_settings` table. They can be changed at any time; only supervisors and
admins can edit them.

| Setting | Key | Default | Notes |
|---|---|---|---|
| Runtime / provider | `ai_provider` | `ollama` | `ollama` or `localai` |
| Base URL | `ai_ollama_base_url` | `http://127.0.0.1:11434` | Loopback/LAN host only |
| Text model | `ai_ollama_text_model` | `llama3.1` | Summaries & passage comments |
| Vision model | `ai_ollama_vision_model` | `llava` | Photo contamination analysis |

**Test Connection** runs a live reachability probe: it lists the models the
runtime actually has installed and flags whether your configured text and
vision models are present — so you can catch a "model not pulled" problem before
you ever run a suggestion.

---

## How approval works (audit trail)

1. You click **Summarize Notes** / **Suggest Passage Comment** / **Analyze
   Photo**.
2. SteloPTC sends the prompt to your local runtime and stores the result as a
   **pending** suggestion in the `ai_suggestions` table. Nothing is written to
   the record yet.
3. You review the draft and either **Approve** or **Reject** it.
4. On approval, the suggestion text is **appended** (never overwrites existing
   notes) to the record's `notes` field through the normal update + audit path,
   tagged `[AI-assisted, approved by <you>]`. The audit entry attributes the
   change to you, and the linked `ai_suggestions` row preserves the model name
   and exact prompt for traceability.

This keeps the AI strictly advisory: it can *propose*, but only a human can
*commit*, and the record shows exactly who approved what.

---

## Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| *"Could not reach the local Ollama … is it running?"* | Runtime not started | Run `ollama serve` (or start your LocalAI server); confirm the Base URL/port |
| *"The model … isn't installed in Ollama. Run `ollama pull …`"* | Model not pulled | Run the exact `ollama pull` command shown, or change the model in Settings |
| Photo analysis returns vague/unhelpful text | Text-only model set as the vision model | Set a vision-capable model (e.g. `llava`) as the vision model |
| First request very slow | Cold model load | Normal — the model loads into RAM on first use, then stays warm |
| *"Only supervisors and admins can change the AI configuration"* | Insufficient role | Ask an admin/supervisor to update Settings → AI Assistant |

See also: [`ROADMAP.md`](../ROADMAP.md) (WP-56, WP-56b) and the
[User Manual](../UserManual.md) §19 for the end-user walkthrough.
