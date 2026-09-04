# Planes pendientes

## Plan 1: Preview de uso de VRAM según GPU Offload — ✅ IMPLEMENTADO (2026-09-03)

**Objetivo:** mostrar una estimación de VRAM que consumirá el modelo al mover el slider de GPU Offload (RightPanel y LoadModelModal), antes de cargar.

### Estado actual (datos disponibles)
- `scan_models` (lib.rs:200) ya parsea el header GGUF vía `read_gguf_metadata` (lib.rs:37) y expone: `size_bytes`, `layer_count` (block_count), `is_moe`, `expert_count`, `expert_used_count`, `max_context`, `arch`.
- No existe ninguna lectura de memoria de GPU (ni `nvidia-smi`, ni parsing del log del server).
- Sliders de offload: RightPanel.vue:48 (`:max="activeModel?.layer_count ?? 999"`) y LoadModelModal.vue:69.
- Selects de cuantización KV existentes: `kCacheQuant` / `vCacheQuant` (F32/F16/Q8_0/Q4_0).

### Cambios

**1. Backend — `src-tauri/src/lib.rs`**
- `ModelFile`: agregar 3 campos (u32, default 0):
  - `embedding_length` → `{arch}.embedding_length`
  - `head_count` → `{arch}.attention.head_count`
  - `head_count_kv` → `{arch}.attention.head_count_kv`
  - (el parser ya los lee al map; solo falta extraerlos, mismo patrón que `layer_count`)
- Comando nuevo `get_gpu_memory() -> Option<{ total: u64, free: u64 }>`:
  - `nvidia-smi --query-gpu=memory.total,memory.free --format=csv,noheader,nounits` (primer GPU)
  - parsear 2 ints; `None` si nvidia-smi no existe o falla (la UI oculta la comparación y muestra solo el absoluto)
- Registrar en `generate_handler!` (lib.rs:749)

**2. Estimación — helper frontend (`src/utils/vram.ts`)**

> **Revisión 2026-09-04 (fix v2, calibrado contra el log de llama.cpp b10784):**
> la fórmula original sobreestimaba el KV de híbridos y subestimaba el SSM y el
> runtime. Calibración con Qwen3.8-27B (qwen35, ngl 65, ctx 155648, KV Q4_0, -np 2,
> draft-mtp): el server reportó weights 15,839 + KV 2,736 (16 capas) + SSM 1,197 +
> compute 861+840 + KV-MTP 171 = 21,620 MiB GPU; medido con nvidia-smi: 22,462 MiB
> (con Windows). Estimación actual: **22.52 GiB** (KV exacto, SSM +1.7%, runtime
> −3.8%, weights con margen por mlock). Fórmula:
> ```
> head_dim    = key_length || (embedding_length / head_count)
> bytesKV     = { F32: 4, F16: 2, Q8_0: 1.0625, Q4_0: 0.5625 }
> interval    = full_attention_interval || 1          (1 = denso, sin cambio)
> n_full      = floor(ngl / interval)                 (medido: 16 capas para ngl 65)
> weights_gpu = size_bytes × (ngl / layer_count)
>             × (1 − frac_experts × n_cpu_moe / expert_count)   [solo MoE con n_cpu_moe > 0]
>   donde frac_experts = expert_params / (expert_params + attn_params) (por capa, desde GGUF)
> kv_cache    = (n_full + [1 si MTP]) × ctx × head_count_kv × head_dim × (bytesK + bytesV)
> ssm_state   = ngl × ssm.state_size × ssm.inner_size × 4 × n_parallel × rs_seq
>   [solo híbridos; rs_seq = spec_draft_n_max si hay MTP (medido: n_max 3→3 rs,
>    n_max 5→5 rs), 1 sin spec]
> runtime     = max(0.5 GiB, weights_gpu × [5% denso / 2% MoE] × [2 si MTP]) + 0.5 GiB
>   [compute buffers escalan con compute activa: medido 861/15839 (5.4%) denso,
>    360/19400 (1.9%) MoE A3B; + contexto CUDA ~0.5 GiB]
> total       = weights_gpu + kv_cache + ssm_state + runtime
> ```
> `--cache-ram` NO afecta la VRAM (es prompt cache en RAM, PR llama.cpp#16391).
> Segunda calibración con Qwen3.6-35B-A3B (qwen35moe, ngl 41, ctx 104960, KV Q4_0,
> n_max 5): server reportó 19,400 + 576.56 (10 capas) + 376.88 + 360+355.8 + 57.66
> = 21,114 MiB GPU. Estimación: **22.23 GiB** (KV exacto, SSM +8%, runtime +19%,
> weights +988 por mlock).
> `--load-mode mlock` deja parte de los weights en RAM host (medido: 896-987 MiB);
> la estimación usa el archivo completo: sobreestima ~1 GiB, lado seguro para OOM.

- Fórmula original (pre-fix, solo válida para densos estándar):
  ```
  head_dim    = embedding_length / head_count
  bytesKV     = { F32: 4, F16: 2, Q8_0: 1, Q4_0: 0.5 }
  weights_gpu = size_bytes × (ngl / layer_count)
  kv_cache    = ngl > 0 ? ctx × layer_count × head_count_kv × head_dim × (bytesK + bytesV) : 0
  buffer      ≈ 200 MB
  total       = weights_gpu + kv_cache + buffer
  ```
- `ngl = 0` → solo runtime (la capa de output queda en GPU, subestimación aceptable)

**3. UI — RightPanel.vue + LoadModelModal.vue**
- Bajo el slider de GPU Offload:
  - Texto: `≈ {n} GiB`
  - Barra fina contra VRAM total (si `get_gpu_memory` devolvió datos): verde <70%, ámbar <90%, rojo ≥90%
  - Tooltip con breakdown: `weights {x} · KV {y} · buffer {z}`
- `get_gpu_memory` se invoca una vez al mount (y opcional: refetch al detener el model, para actualizar `free`)

**4. i18n — `en.ts` / `es.ts`**
- `load.vramEstimate`: "≈ {size} GiB VRAM"
- (opcional) `load.vramOfTotal`: "≈ {size} / {total} GiB"

### Precisión esperada
±10-15%. La proporción `size × ngl/layers` es una aproximación (MoE y capas de embedding/output no son uniformes). Es una preview; el número real lo da el log del server al cargar (`llama_model_load: VRAM used: X MiB`).

### Archivos tocados
`src-tauri/src/lib.rs`, `src/components/RightPanel.vue`, `src/components/LoadModelModal.vue`, `src/stores/config.ts` (o helper nuevo), `src/i18n/en.ts`, `src/i18n/es.ts`

### Verificación
- `cargo check` + `npm run build`
- Manual: mover el slider con un modelo grande en la 3090 (24 GB) — el estimado de "todas las capas" debe acercarse al `VRAM used` real del log; ngl=0 → ~0.2 GiB

---

## Plan 2: Detección dinámica de modelos (sin exigir `creador/modelo/modelo.gguf`) — ✅ IMPLEMENTADO (2026-09-03)

**Objetivo:** detectar cualquier `.gguf` dentro de la carpeta de modelos sin importar la profundidad de la carpeta.

### Estado actual
`scan_models` (lib.rs:200-299) exige exactamente 3 niveles:
```
models_path/
  <publisher>/        ← nivel 1 (obligatorio, dir)
    <family>/         ← nivel 2 (obligatorio, dir)
      *.gguf          ← nivel 3 (archivos .gguf; "mmproj" en el nombre → projector)
```
- Un `.gguf` directamente en `models_path/` o en nivel 1 → **invisible**
- Un `.gguf` en nivel 3+ (subcarpetas dentro de family) → **invisible**
- `publisher` y `model_family` = nombres de los dirs 1 y 2
- mmproj: solo se asocia si está en el **mismo dir de family**
- UI (ModelsView) ya soporta cualquier valor de `publisher`/`model_family` (columna + grupos de usuario por path) → no requiere cambios para funcionar

### Cambios

**1. Backend — `scan_models` → walk recursivo**
- Reemplazar el doble bucle de dirs por una recursión sobre `models_path`:
  - profundidad máx ~6; saltar dirs ocultos (`.git`, `.cache`, etc.)
  - todo `*.gguf` cuyo nombre NO contenga "mmproj" → modelo
  - todo `*.gguf` con "mmproj" → candidato a projector
- Derivar `publisher` / `model_family` desde el path relativo (manteniendo el comportamiento actual para layouts de 3 niveles):
  | path relativo | publisher | family |
  |---|---|---|
  | `a.gguf` | `Ungrouped` | `—` |
  | `pub/a.gguf` | `pub` | `—` |
  | `pub/fam/a.gguf` | `pub` | `fam` *(como hoy)* |
  | `pub/fam/deep/a.gguf` | `pub` | `fam` *(primeros 2 niveles)* |
- Recursión interna con `fs::read_dir` (sin nueva dependencia; `walkdir` no está en Cargo.toml)

**2. Asociación de mmproj (mejorar)**
- Hoy: mismo dir de family. Cambiar a, en orden:
  1. **Mismo directorio** del model (convención común: `model.gguf` + `mmproj-*.gguf` juntos)
  2. Match por prefijo de nombre (`qwen2.5-vl.gguf` ↔ `qwen2.5-vl-mmproj.gguf`) — opcional, fase 2
  3. Fallback: comportamiento actual (mismo family)

**3. Compatibilidad**
- Los modelos existentes (3 niveles) conservan `publisher`/`family` idénticos → grupos de usuario (keyeado por path completo) siguen válidos sin migración
- `restoreLoadedModel` y el config guardado usan `path` completo → sin impacto

**4. UI (opcional, mínimo)**
- Tooltip con el path relativo del model en la fila (útil para saber dónde vive si la carpeta es plana)

### Archivos tocados
`src-tauri/src/lib.rs` (todo el cambio dura acá, salvo el tooltip opcional en `src/views/ModelsView.vue`)

### Verificación
- `cargo check` + `npm run build`
- Manual:
  - modelo en `models/x.gguf` (plano) → aparece como `Ungrouped`
  - modelo en `models/publisher/model.gguf` (2 niveles) → aparece con publisher
  - modelo en `models/p/f/s/muy/deep.gguf` → aparece como `p / f`
  - layout actual de 3 niveles → idéntico a hoy (publisher/family/mmproj)
  - un mmproj al lado del model en el mismo dir → se asocia
