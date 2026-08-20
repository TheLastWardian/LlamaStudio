import { ref } from 'vue'
import en from './en'
import es from './es'

// type TranslationKeys = typeof en

export const currentLang = ref<'en' | 'es'>('en')

export function t(key: string, params?: Record<string, string | number>): string {
  const parts = key.split('.')
  let obj: any = currentLang.value === 'es' ? es : en
  for (const part of parts) {
    if (obj && obj[part] !== undefined) {
      obj = obj[part]
    } else {
      return key
    }
  }
  let result = obj
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      result = result.replace(new RegExp(`{${k}}`, 'g'), String(v))
    }
  }
  return result
}

export function setLang(lang: 'en' | 'es') {
  currentLang.value = lang
}
