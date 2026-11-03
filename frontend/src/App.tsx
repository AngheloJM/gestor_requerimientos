import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { RequireAuth } from '@/auth/RequireAuth'
import { Layout } from '@/components/Layout'
import { LoginPage } from '@/pages/LoginPage'
import { Pendiente } from '@/pages/Pendiente'
import { TableroPage } from '@/pages/TableroPage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route element={<RequireAuth />}>
          <Route element={<Layout />}>
            <Route path="/tablero" element={<TableroPage />} />
            <Route path="/matriz" element={<Pendiente titulo="Matriz de servicio" hook="useMatrizServicio" />} />
            <Route path="/almanaque" element={<Pendiente titulo="Almanaque" hook="useAlmanaque" />} />
            <Route path="/aprobaciones" element={<Pendiente titulo="Aprobaciones" hook="useAprobaciones / useEditarFechaFin" />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/tablero" replace />} />
      </Routes>
    </BrowserRouter>
  )
}
