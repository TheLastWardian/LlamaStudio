# Plan: Selector de Idioma (EN/ES)

Estado: **Pendiente** — se implementa después de terminar las funcionalidades pendientes.

## Enfoque

Store simple + función `t(key)`. Sin librería externa (vue-i18n, etc.).

## Step 1: Nuevo store `src/stores/locale.ts`

- `lang: Ref<'en' | 'es'>` — persistido en `localStorage`
- `t(key: string): string` — lookup en diccionario
- Diccionario: `{ [key]: { en: string, es: string } }`
- Función `setLang(lang: 'en' | 'es')` que persiste y reactiva

## Step 2: `SettingsView.vue` — Agregar selector

Nueva sección "Language":
```vue
<div class="settings-field">
  <label>{{ t('settings.language') }}</label>
  <select class="field-select" v-model="lang">
    <option value="en">English</option>
    <option value="es">Español</option>
  </select>
</div>
```

## Step 3: Reemplazar strings hardcoded por `t('key')`

| Archivo | ~Strings | Ejemplos |
|---------|----------|----------|
| `src/views/SettingsView.vue` | 12 | `t('settings.models_folder')`, `t('settings.save')` |
| `src/views/ModelsView.vue` | 20 | `t('models.create_group')`, `t('models.column_arch')` |
| `src/components/RightPanel.vue` | 80 | `t('panel.context_length')`, `t('panel.gpu_offload')`, `t('panel.advanced')` |
| `src/components/LoadModelModal.vue` | 35 | `t('modal.back')`, `t('modal.load_model')` |
| `src/components/Sidebar.vue` | 4 | `t('sidebar.models')`, `t('sidebar.settings')` |
| `src/App.vue` | 2 | Títulos de topbar |

**Total: ~150 strings**

## Step 4: Ajustes menores

- Tooltips de RightPanel (actualmente en español hardcoded) → al diccionario
- Placeholders de inputs → `:placeholder="t('...')"`
- Strings dinámicos ("X models") → interpolación: `t('models.count', { n: section.models.length })`

## Lo que NO se toca

- `src-tauri/src/lib.rs` — backend no tiene i18n
- Nombres de modelos / archivos — nunca traducir
- Valores técnicos de selects (F16, Q8_0, MTP, etc.) — se mantienen
- Rutas de archivos

## Convención de keys

```
settings.*   → SettingsView
models.*     → ModelsView (columnas, context menu, grupos)
panel.*      → RightPanel (tabs Load/Info/Inference, secciones, labels)
modal.*      → LoadModelModal
sidebar.*    → Sidebar
app.*        → App.vue
```

## Ejemplo de entrada en el diccionario

```ts
'panel.context_length': { en: 'Context Length', es: 'Longitud de Contexto' },
'panel.gpu_offload':    { en: 'GPU Offload', es: 'Offload GPU' },
'panel.advanced':       { en: 'Advanced', es: 'Avanzado' },
'models.create_group':  { en: 'Create Group', es: 'Crear Grupo' },
'settings.language':    { en: 'Language', es: 'Idioma' },
```
