/**
 * Preferencias de lectura por usuario y navegador (revisión 04/11/2026: baja visión).
 * Se guardan en localStorage solo como comodidad; si no hay almacenamiento, se usan los valores
 * por defecto. El tamaño escala todo el texto (las clases de Tailwind usan rem).
 */
export const ESCALAS = [
  { etiqueta: 'Normal', porcentaje: 100 },
  { etiqueta: 'Grande', porcentaje: 112.5 },
  { etiqueta: 'Muy grande', porcentaje: 125 },
] as const

export interface Preferencias {
  escala: number
  contraste: boolean
}

const CLAVE = 'nova.accesibilidad'
const DEFECTO: Preferencias = { escala: 0, contraste: false }

export function leerPreferencias(): Preferencias {
  try {
    const p = JSON.parse(localStorage.getItem(CLAVE) ?? 'null') as Partial<Preferencias> | null
    const escala = Number(p?.escala)
    return {
      escala: Number.isInteger(escala) && escala >= 0 && escala < ESCALAS.length ? escala : 0,
      contraste: p?.contraste === true,
    }
  } catch {
    return DEFECTO
  }
}

export function aplicarPreferencias(p: Preferencias) {
  const html = document.documentElement
  html.style.fontSize = `${ESCALAS[p.escala].porcentaje}%`
  html.classList.toggle('alto-contraste', p.contraste)
  try {
    localStorage.setItem(CLAVE, JSON.stringify(p))
  } catch {
    // Sin almacenamiento: la preferencia dura hasta recargar.
  }
}
