import { BrowserRouter, Navigate, Route, Routes } from 'react-router-dom'

import { RequireAuth } from '@/auth/RequireAuth'
import { Layout } from '@/components/Layout'
import { AlmanaquePage } from '@/pages/AlmanaquePage'
import { AprobacionesPage } from '@/pages/AprobacionesPage'
import { LoginPage } from '@/pages/LoginPage'
import { MisRequerimientosPage } from '@/pages/MisRequerimientosPage'
import { NuevoRequerimientoPage } from '@/pages/NuevoRequerimientoPage'
import { RolEditorPage } from '@/pages/RolEditorPage'
import { RolesPage } from '@/pages/RolesPage'
import { Pendiente } from '@/pages/Pendiente'
import { TableroPage } from '@/pages/TableroPage'
import { UsuariosPage } from '@/pages/UsuariosPage'

export default function App() {
  return (
    <BrowserRouter>
      <Routes>
        <Route path="/login" element={<LoginPage />} />
        <Route element={<RequireAuth />}>
          <Route element={<Layout />}>
            <Route path="/tablero" element={<TableroPage />} />
            <Route path="/requerimientos" element={<MisRequerimientosPage />} />
            <Route path="/nuevo-requerimiento" element={<NuevoRequerimientoPage />} />
            <Route path="/usuarios" element={<UsuariosPage />} />
            <Route path="/roles" element={<RolesPage />} />
            <Route path="/roles/:id" element={<RolEditorPage />} />
            <Route path="/matriz" element={<Pendiente titulo="Matriz de servicio" hook="useMatrizServicio" />} />
            <Route path="/almanaque" element={<AlmanaquePage />} />
            <Route path="/aprobaciones" element={<AprobacionesPage />} />
          </Route>
        </Route>
        <Route path="*" element={<Navigate to="/tablero" replace />} />
      </Routes>
    </BrowserRouter>
  )
}
