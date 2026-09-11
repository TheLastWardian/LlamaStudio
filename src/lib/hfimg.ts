// Imágenes de README/comentarios: los hosts allowlistados van por el protocolo
// `hfimg` para que el fetch lo haga llamastudio.exe (reqwest) y el webview no
// haga tráfico a internet. Mantener en sync con IMG_HOSTS/host_allowed en
// src-tauri/src/hf.rs (la política real la aplica el backend; esto es solo
// el pre-filtro del frontend).
export const HFIMG_BASE = 'http://hfimg.localhost/'

export function routeReadmeImages(html: string): string {
  return html.replace(
    /(src\s*=\s*)(["'])(https:\/\/(?:cdn-uploads\.huggingface\.co|cdn\.huggingface\.co|huggingface\.co|raw\.githubusercontent\.com|github\.com|[a-z0-9-]+\.gitbook\.io)\/[^"']+)\2/gi,
    (_m, pre: string, q: string, u: string) => `${pre}${q}${HFIMG_BASE}${encodeURIComponent(u)}${q}`,
  )
}
