/**
 * Alto contraste (revisión 04/11/2026: baja visión). Se guarda en localStorage solo como
 * comodidad; sin almacenamiento, se usa el valor por defecto (desactivado).
 */
export interface Preferencias {
  contraste: boolean
}

const CLAVE = 'nova.accesibilidad'

export function leerPreferencias(): Preferencias {
  try {
    const p = JSON.parse(localStorage.getItem(CLAVE) ?? 'null') as Partial<Preferencias> | null
    return { contraste: p?.contraste === true }
  } catch {
    return { contraste: false }
  }
}

export function aplicarPreferencias(p: Preferencias) {
  const html = document.documentElement
  // Limpia un tamaño de texto guardado por la versión anterior.
  html.style.removeProperty('font-size')
  html.classList.toggle('alto-contraste', p.contraste)
  try {
    localStorage.setItem(CLAVE, JSON.stringify(p))
  } catch {
    // Sin almacenamiento: la preferencia dura hasta recargar.
  }
}
