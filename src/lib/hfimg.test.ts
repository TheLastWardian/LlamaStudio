import { describe, it, expect } from 'vitest'
import { routeReadmeImages, HFIMG_BASE } from './hfimg'

describe('routeReadmeImages', () => {
  it('rutea los hosts allowlistados al protocolo hfimg', () => {
    expect(routeReadmeImages('<img src="https://cdn.huggingface.co/x.png">'))
      .toBe(`<img src="${HFIMG_BASE}https%3A%2F%2Fcdn.huggingface.co%2Fx.png">`)
    expect(routeReadmeImages('<img src="https://huggingface.co/a/b/raw/main/x.png">')).toContain(`${HFIMG_BASE}https%3A%2F%2Fhuggingface.co%2Fa%2Fb`)
    expect(routeReadmeImages('<img src="https://raw.githubusercontent.com/o/r/main/x.png">')).toContain(`${HFIMG_BASE}https%3A%2F%2Fraw.githubusercontent.com`)
    expect(routeReadmeImages('<img src="https://github.com/o/r/raw/main/x.png">')).toContain(`${HFIMG_BASE}https%3A%2F%2Fgithub.com`)
    expect(routeReadmeImages('<img src="https://cdn-uploads.huggingface.co/production/uploads/abc/x.png">')).toContain(`${HFIMG_BASE}https%3A%2F%2Fcdn-uploads.huggingface.co`)
    expect(routeReadmeImages('<img src="https://3215535692-files.gitbook.io/~/files/v0/x.gif?token=abc">')).toContain(`${HFIMG_BASE}https%3A%2F%2F3215535692-files.gitbook.io`)
  })

  it('no toca hosts fuera del allowlist ni http', () => {
    const html = '<img src="https://unsloth.ai/x.png"><img src="http://cdn.huggingface.co/x.png"><img src="https://evil.com/x.png">'
    expect(routeReadmeImages(html)).toBe(html)
  })

  it('respeta comillas simples y urls con query', () => {
    const out = routeReadmeImages(`<img src='https://cdn.huggingface.co/x.png?a=1&b=2'>`)
    expect(out).toBe(`<img src='${HFIMG_BASE}https%3A%2F%2Fcdn.huggingface.co%2Fx.png%3Fa%3D1%26b%3D2'>`)
  })
})
