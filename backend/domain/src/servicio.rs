use crate::{CentroCostoId, ClienteId, ContratoComercialId, ProyectoId, ServicioId};

/// `servicios`: FKs **independientes** a cliente, centro de costo, contrato comercial y
/// proyecto (todas opcionales). Las consultas filtran siempre por `activo = true`.
///
/// Columnas aún no modeladas (no las usa ningún endpoint todavía): tipo_contrato, modalidad,
/// tipo_turno, dias_trabajo, departamento_region, manual_funciones_codigo,
/// tiempo_capacitacion_dias, capacitacion_pagada, tiene_refrigerio.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Servicio {
    pub id: ServicioId,
    pub codigo: Option<String>,
    pub nombre: String,
    pub centro_costo_id: Option<CentroCostoId>,
    pub contrato_comercial_id: Option<ContratoComercialId>,
    pub cliente_id: Option<ClienteId>,
    pub proyecto_id: Option<ProyectoId>,
    pub activo: bool,
}
