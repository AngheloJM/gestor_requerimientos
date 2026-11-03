//! Identificadores tipados: el compilador impide pasar un `ClienteId` donde va un
//! `CentroCostoId` (relevante porque `servicios` tiene FKs independientes a ambos).
//!
//! SUPUESTO de tipo: todas las PK son `uuid` (convención de Supabase/Lovable). Verificar.

use std::fmt;

use uuid::Uuid;

macro_rules! id_type {
    ($($(#[$doc:meta])* $nombre:ident),+ $(,)?) => {$(
        $(#[$doc])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $nombre(pub Uuid);

        impl $nombre {
            pub fn new() -> Self { Self(Uuid::new_v4()) }
        }

        impl Default for $nombre {
            fn default() -> Self { Self::new() }
        }

        impl From<Uuid> for $nombre {
            fn from(u: Uuid) -> Self { Self(u) }
        }

        impl From<$nombre> for Uuid {
            fn from(id: $nombre) -> Self { id.0 }
        }

        impl fmt::Display for $nombre {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
        }
    )+};
}

id_type!(
    ClienteId,
    CentroCostoId,
    ContratoComercialId,
    ProyectoId,
    ServicioId,
    CargoId,
    /// `servicio_cargos.id` (fila de Composición Salarial).
    ServicioCargoId,
    /// `servicio_cargo_metricas.id`.
    ServicioCargoMetricaId,
    MetricaId,
    UsuarioId,
    RequerimientoId,
    SolicitudCambioId,
);
