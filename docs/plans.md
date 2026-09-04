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

**2. Estimación — helper frontend**
- Nueva función (p.ej. `src/stores/config.ts` o `src/utils/vram.ts`):
  ```
  head_dim    = embedding_length / head_count
  bytesKV     = { F32: 4, F16: 2, Q8_0: 1, Q4_0: 0.5 }
  weights_gpu = size_bytes × (ngl / layer_count)
  kv_cache    = ngl > 0 ? ctx × layer_count × head_count_kv × head_dim × (bytesK + bytesV) : 0
  buffer      ≈ 200 MB
  total       = weights_gpu + kv_cache + buffer
  ```
- `ngl = 0` → solo buffer (la capa de output queda en GPU, subestimación aceptable)

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

## Plan 2: Detección dinámica de modelos (sin exigir `creador/modelo/modelo.gguf`)

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
