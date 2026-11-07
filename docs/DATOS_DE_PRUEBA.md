# Datos de prueba para la demostración

Juego de datos **ficticio y solo local** para mostrar lo hecho en la revisión del 06/11/2026.
Nunca se carga en producción.

- Script: [`infra/db/seed_demo.sql`](../infra/db/seed_demo.sql).
- Cómo correrlo: `infra/db/seed-demo.sh`. Solo acepta la URL de la base local (`infra/db/db.sh url`). Además, el SQL se corta si no encuentra la tabla `credenciales`, que solo existe en la base local.
- **Reseteable:** cada corrida borra todo lo "Demo" y lo vuelve a crear. Se puede repetir sin que nada se duplique.
- **Variante con nombres reales:** `infra/db/seed-demo.sh real`. Es solo local: `seed_demo_real.sql` y su documento `infra/db/DATOS_DE_PRUEBA_REAL.md` están en `infra/db/.gitignore`. Cada variante borra la otra al empezar, así que en pantalla hay un solo juego por vez.
- **Usuario para la demo:** `rrhh@demo.local`. La contraseña está en `infra/db/.credenciales-dev` (archivo local, no versionado).
- **Folios:** cambian en cada corrida, porque la base los numera sola. Cada caso se reconoce por el inicio de su comentario: **`[DEMO 1]` … `[DEMO 5]`**. Al terminar, el script imprime una tabla con el folio de cada caso.
- **Otros datos en pantalla:** también se ven los datos locales que ya había. Los de la demo son los que dicen "Demo".

## Cómo se creó cada cosa

| Qué | Cómo | Por qué |
|---|---|---|
| Cliente, centros de costo, contrato, servicios, cargos, sueldos, cantidades, tiempos, bono, reparto del cargo compartido, feriado | **INSERT directo** en la base local | Es el catálogo de partida. Equivale a cargarlo desde Matriz de servicio y Parámetros, pero los centros de costo no se pueden crear desde la pantalla, a propósito. |
| Los 5 requerimientos | **Función oficial `crear_requerimiento_con_snapshot`**, ejecutada **como el usuario RRHH** (mismo contexto que arma el backend: rol `authenticated` + datos del token) | Pasan por las mismas validaciones y triggers que cuando se crean desde "Nuevo requerimiento": congelan sueldo y bonos, calculan la fecha de capacitación, generan el folio y guardan la cantidad y las fechas originales. |
| Cambios de cantidad, de fecha de operación y de estado | **Función oficial `editar_requerimiento_con_historial`**, como RRHH, con glosa y, cuando corresponde, motivo del desvío | Es la misma función que usan "Cambiar cantidad", "Cambiar fecha" y Aprobaciones. Por eso queda el historial por usuario. Cada cambio va en su propia transacción, para que el historial tenga horas distintas y salga en orden. |
| Entrega y rectificación del caso 3 | **INSERT directo** | La app no tiene pantalla para cargar entregas, y la base local no tiene una función que lo haga (en producción existe una validación, `p2_validar_entrega`). Se respetaron sus reglas: la rectificación corrige una entrega **aceptada** del mismo requerimiento, su cantidad es distinta de 0 y no lleva `cantidad_reclutada`. |

## Catálogo de la demo

| Elemento | Datos |
|---|---|
| Cliente | Cliente Demo S.A. |
| Centros de costo | `DEMO-A · Centro Demo Atención` y `DEMO-B · Centro Demo Cobranza` |
| Contrato | `K-DEMO · Contrato Demo`, del 01/01/2026 al 31/12/2027. Está vigente, así que los dos servicios cuentan como **configurados** en el Tablero. |
| Servicio **Demo Atención** (`DEMO-ATE`) | Centro DEMO-A, Santa Cruz, por turno, lunes a viernes, **tiempo de capacitación general: 7 días** |
| Servicio **Demo Cobranza** (`DEMO-COB`) | Centro DEMO-B, La Paz, continuo, lunes a sábado, **tiempo de capacitación general: 5 días** |
| Feriado | **Viernes 20/11/2026, "Feriado de demostración (ficticio)"**. No es un feriado real: está para que se note en los días hábiles. |

Cargos en cada servicio (`servicio_cargos`):

| Cargo | Servicio | Haber diurno / nocturno | Mujeres / hombres | Capacitación / OJT / ajustada | Bono |
|---|---|---|---|---|---|
| Agente Demo | Demo Atención | Bs 3.800 / Bs 400 | 6 / 4 | **10 / 5 / 12 días** | Calidad, Bs 300 |
| Asistente Demo | Demo Atención | Bs 3.700 / — | 2 / 1 | — / — / — (sin tiempos propios) | — |
| Supervisor Demo Compartido | Demo Atención | Bs 5.000 / — | 1 / 1 | — | — |
| Supervisor Demo Compartido | Demo Cobranza | Bs 4.800 / — | 1 / 0 | — | — |

## "Hoy" y los días hábiles

El tiempo de reclutamiento se cuenta **desde hoy**, según la fecha de Bolivia. Los valores de este documento son con **hoy = sábado 07/11/2026**. Otro día, el número va a ser distinto, y se recalcula así:

> **Días hábiles** = días entre hoy (incluido) y la fecha de entrega a capacitación (excluida), sin sábados, domingos ni feriados cargados.

---

## Caso A: cargo con los 3 tiempos cargados (puntos 9 a 11)

- **Qué muestra:** el cálculo automático de fechas con datos propios del cargo, no con el respaldo del servicio.
- **Dónde verlo:** *Nuevo requerimiento*. Elegí servicio **Demo Atención**, cargo **Agente Demo** y fecha de inicio de operación **15/12/2026**. Los tres datos aparecen solos en "4. Datos adicionales".
- **Cómo se armó:** los tiempos del cargo se cargaron por INSERT directo (10 / 5 / 12). El cálculo lo hace la API en el momento; no se guarda.

| Dato en pantalla | Valor | De dónde sale |
|---|---|---|
| Fecha de inicio máximo de capacitación | **05/12/2026** | 15/12 − 10 días (`tiempo_capacitacion_dias` del cargo = 10), en días corridos |
| Fecha de inicio máximo de capacitación ajustada | **03/12/2026** | 15/12 − 12 días (`tiempo_capacitacion_ajustada_dias` = 12) |
| Tiempo de reclutamiento | **17 días hábiles** | Se cuenta hasta la **ajustada** (03/12), porque existe. Del 07/11 al 02/12 hay 18 días de lunes a viernes; se resta 1 por el feriado del viernes 20/11. |

El OJT (5 días) se ve en Matriz → Servicios → Demo Atención → "Cargos de este servicio", pero no entra en el cálculo, como se decidió.

## Caso B: cargo sin tiempos propios (usa el del servicio)

- **Dónde verlo:** *Nuevo requerimiento* con **Demo Atención** / **Asistente Demo** / operación **15/12/2026**.

| Dato en pantalla | Valor | De dónde sale |
|---|---|---|
| Fecha de inicio máximo de capacitación | **08/12/2026**, con la nota "Con el tiempo general del servicio (el cargo no tiene uno propio)" | 15/12 − 7 días (tiempo general de **Demo Atención**) |
| Capacitación ajustada | **Sin dato**, con la nota "El cargo no tiene tiempo de capacitación ajustada cargado" | El cargo no tiene ajustada. Para esta fecha no se usa el tiempo del servicio como respaldo. |
| Tiempo de reclutamiento | **20 días hábiles** | Sin ajustada, se cuenta hasta la normal (08/12). Del 07/11 al 07/12 hay 21 días de lunes a viernes, menos el feriado del 20/11. |

Otra combinación sin tiempos propios: **Demo Cobranza / Supervisor Demo Compartido** con operación 15/12 da **10/12/2026** (15/12 − 5, el tiempo general de Demo Cobranza) y **22 días hábiles**.

## Caso C: cargo compartido entre 2 centros de costo (Composición salarial)

- **Qué muestra:** Composición salarial de punta a punta.
- **Dónde verlo:** *Matriz de servicio → Composición salarial*, en el bloque "Cargos compartidos entre centros de costo".
- **Cómo se armó:** **Supervisor Demo Compartido** está en Demo Atención (centro DEMO-A) y en Demo Cobranza (centro DEMO-B). Por eso tiene 2 centros posibles. El reparto se cargó con INSERT directo en `cargo_centros_costo`, que es la misma tabla donde guarda "Guardar composición".

| Dato en pantalla | Valor | De dónde sale |
|---|---|---|
| DEMO-A · Centro Demo Atención | **Bs 4.200,00, 60.00 %** | 4.200 / 7.000 |
| DEMO-B · Centro Demo Cobranza | **Bs 2.800,00, 40.00 %** | 2.800 / 7.000 |
| Total | **Bs 7.000,00** | 4.200 + 2.800. Los porcentajes no se guardan: se calculan al mostrar. |

Con **Editar** se abre el formulario con esos montos. También se puede probar en vivo "+ Crear cargo nuevo": un cargo recién creado no tiene servicio, así que se ofrecen todos los centros de costo.

## Caso D: cargo de un solo servicio, bloqueado en Composición salarial

- **Dónde verlo:** *Composición salarial → Nueva composición → Cargo:* **Agente Demo**.
- **Qué se ve:** el aviso rojo *"Este apartado es solo para cargos que no pertenecen a un solo servicio. Este cargo solo tiene un centro de costo (DEMO-A · Centro Demo Atención): su composición se arma dentro del servicio."* No aparecen filas para cargar montos.
- **De dónde sale:** Agente Demo solo está en Demo Atención, así que tiene un único centro posible (DEMO-A). La regla exige al menos 2.

## Caso 1 `[DEMO 1]`: cantidad aumentada con glosa (historial con 2 movimientos)

- **Tipo / estado:** Nuevo / **Pendiente**.
- **Servicio / cargo:** Demo Atención / Agente Demo.
- **Dónde verlo:** *Mis requerimientos* → el que dice Demo Atención · Agente Demo · cantidad 8 · operación 15/12/2026 → detalle.
- **Cómo se armó:** se creó con cantidad 5 usando la función oficial. Después, como RRHH, se editó dos veces con la función oficial:
  1. **5 → 7**, glosa *"El cliente amplió la campaña en dos puestos"*, desvío **Cambio del cliente**, *"Ampliación de campaña de fin de año"*;
  2. **7 → 8**, glosa *"Se suma un puesto de respaldo para cubrir ausencias"*, sin desvío.

| Dato en pantalla | Valor | De dónde sale |
|---|---|---|
| Cantidad requerida (actual) | **8** | Último cambio |
| Original | **5 (no se modifica)** | Lo guardó la base al crear (`requerimiento_referencia`) |
| Total aceptado / cantidad actual | **0 / 8** | No tiene entregas |
| Fecha de inicio de operación | **15/12/2026** | La de la creación |
| Fecha de capacitación guardada | **05/12/2026** (se ve en Aprobaciones) | 15/12 − 10 días del cargo (trigger de la migración 0006) |
| Sueldo y bonos congelados | **Bs 3.800 / Bs 400; Calidad Bs 300** | Copiados del cargo al crear (snapshot) |
| Historial de cambios | **"Cantidad requerida: 7 → 8"** y **"5 → 7"** (este con el desvío), cada uno con "07/11/2026 · RRHH" | `requerimiento_historial`. Lo escribe el trigger de la base con el usuario que editó. |

## Caso 2 `[DEMO 2]`: fecha de operación modificada con glosa

- **Tipo / estado:** Modificación / **Aprobado**.
- **Servicio / cargo:** Demo Atención / Asistente Demo, cantidad 3.
- **Cómo se armó:** se creó con operación **01/12/2026**. Después, como RRHH, con la función oficial:
  1. **01/12 → 10/12**, glosa *"El cliente postergó el inicio de la campaña"*, desvío **Cambio del cliente**;
  2. **10/12 → 12/12**, glosa *"Ajuste por disponibilidad de la sala de capacitación"*;
  3. cambio de estado **pendiente → aprobado**, glosa *"Aprobado por RR.HH."*.

| Dato en pantalla | Valor | De dónde sale |
|---|---|---|
| Fecha de inicio de operación | **12/12/2026** | Último cambio |
| Historial | "Estado: pendiente → aprobado", "Inicio de operación: 10/12/2026 → 12/12/2026" e "Inicio de operación: 01/12/2026 → 10/12/2026" (este con el desvío) | `requerimiento_historial` |
| Fecha de capacitación guardada | **24/11/2026** (se ve en Aprobaciones) | Se calculó al crear: 01/12 − 7 días del servicio (el cargo no tiene tiempo propio). **No se movió** con la fecha de operación: es el TODO pendiente de confirmar con la clienta. |

## Caso 3 `[DEMO 3]`: entrega con rectificación (Almanaque "quedan N")

- **Tipo / estado:** Cambio / **En proceso**.
- **Servicio / cargo:** Demo Atención / Agente Demo, cantidad 6, operación 01/12/2026.
- **Cómo se armó:** se creó y se pasó a "en proceso" con las funciones oficiales. **La entrega y la rectificación se insertaron directo** (ver arriba por qué):
  - entrega del **18/11/2026**, **3 personas**, con decisión **aceptada**;
  - rectificación del **19/11/2026**, **+2**, que corrige esa entrega (sin decisión propia).
- **Dónde verlo:** *Almanaque*, noviembre 2026, y el detalle del requerimiento.

| Dato en pantalla | Valor | De dónde sale |
|---|---|---|
| Almanaque, día 18 | **"Agente Demo, Demo Atención, 3"** | La entrega |
| Almanaque, día 19 | **"Agente Demo, Demo Atención, quedan 5"** | Entrega (3) + rectificación (2), con el criterio de suma decidido |
| Detalle del día 19 | Cliente **Cliente Demo S.A.**, estado **Pendiente de aceptación**, el folio, "Ingreso a operaciones pactado con el cliente: **01/12/2026**" | El estado es el de la rectificación, que no tiene decisión. El ingreso pactado es la fecha de operación del requerimiento. |
| Total aceptado / cantidad actual | **3 / 6** | Solo cuenta la entrega con decisión "aceptada" |
| Reclutados informados / cantidad original | **5 / 6** | 3 + 2 (misma suma que el Almanaque) |
| Fecha de capacitación guardada | **21/11/2026** | 01/12 − 10 días del cargo |

## Caso 4 `[DEMO 4]`: promoción rechazada

- **Tipo / estado:** Promoción / **Rechazado**.
- **Servicio / cargo:** Demo Cobranza / Supervisor Demo Compartido, cantidad 1, operación 07/12/2026.
- **Cómo se armó:** se creó con la función oficial. El cambio de estado a rechazado, con glosa *"El postulante no cumple la antigüedad mínima para la promoción"*, también se hizo con la función oficial.
- **Fecha de capacitación guardada:** **07/12/2026**, la misma de operación. En una **promoción** la base no resta días de capacitación, así que queda la fecha que envía la pantalla, que es la de operación.
- **Sueldo congelado:** Bs 4.800, el del cargo en **Demo Cobranza**, no el de Demo Atención.

## Caso 5 `[DEMO 5]`: creado desde un día vacío del Almanaque

- **Tipo / estado:** Nuevo / **Pendiente**.
- **Servicio / cargo:** Demo Cobranza / Supervisor Demo Compartido, cantidad 2, motivo Nuevo proyecto.
- **Fecha de operación:** **26/11/2026**, el día vacío del Almanaque. El comentario lo dice: *"Creado desde el día vacío 26/11/2026 del Almanaque"*.
- **Cómo se armó:** con la función oficial y esa fecha de operación. **No se hizo con clics en la pantalla.** El botón "Crear requerimiento" del Almanaque solo abre *Nuevo requerimiento* con la fecha ya cargada, y el resto es la misma creación de siempre. Por eso el resultado es igual a haberlo hecho desde la pantalla.
- **Para mostrarlo en vivo:** *Almanaque* → noviembre 2026 → tocar el **26** → el panel dice "Sin entregas este día" y muestra el botón **Crear requerimiento** → se abre el formulario con 26/11/2026 cargado. El día sigue vacío porque el calendario muestra entregas, no requerimientos.
- **Fecha de capacitación guardada:** **21/11/2026**. Es 26/11 − 5 días, el tiempo general de Demo Cobranza, porque el cargo no tiene tiempo propio en ese servicio.

---

## Cobertura de lo pedido

| Pedido | Dónde está |
|---|---|
| Cargo con los 3 tiempos | Caso A (Agente Demo) |
| Cargo sin tiempos propios | Caso B (Asistente Demo) |
| Cargo compartido con reparto en % | Caso C (Supervisor Demo Compartido, 60 % / 40 %) |
| Cargo de un solo servicio, bloqueado | Caso D (Agente Demo) |
| Cantidad aumentada con historial de 2 movimientos o más | Caso 1 |
| Fecha de operación modificada con historial | Caso 2 |
| Rectificación con "quedan N" real | Caso 3 (quedan 5) |
| Feriado que afecta los días hábiles | 20/11/2026: casos A y B (sin el feriado serían 18 y 21 días) |
| Estados Pendiente, Aprobado, Rechazado y En proceso | Casos 1 y 5, 2, 4 y 3 |
| Tipos Nuevo, Modificación, Promoción y Cambio | Casos 1 y 5, 2, 4 y 3 |
| Creado desde un día vacío del Almanaque | Caso 5 |

## Qué más se ve

- **Tablero:** los dos servicios de la demo suman 2 a "Servicios configurados", porque tienen contrato vigente y cargos. Los servicios incompletos y las solicitudes de cambio que se ven son datos locales previos, no de la demo.
- **Mis requerimientos:** los 5 casos aparecen como "mío", porque los creó el usuario RRHH.
- **Matriz → Servicios → Editar Demo Atención:** en "Cargos de este servicio" se ven los tres cargos con sueldos, cantidades y tiempos (Agente Demo: "10 d · 5 d · 12 d").
