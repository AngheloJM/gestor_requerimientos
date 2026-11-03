//! Limitador de intentos fallidos en memoria (por email y por IP), con ventana deslizante.
//!
//! Alcance: una sola instancia del backend. TODO(infra): con varias réplicas, mover a un
//! almacenamiento compartido (ej. Redis o una tabla).

use std::{
    collections::{HashMap, VecDeque},
    sync::Mutex,
    time::{Duration, Instant},
};

pub struct Limitador {
    ventana: Duration,
    max_por_clave: usize,
    fallos: Mutex<HashMap<String, VecDeque<Instant>>>,
}

impl Limitador {
    pub fn new(ventana: Duration, max_por_clave: usize) -> Self {
        Self {
            ventana,
            max_por_clave,
            fallos: Mutex::new(HashMap::new()),
        }
    }

    fn podar(cola: &mut VecDeque<Instant>, ventana: Duration, ahora: Instant) {
        while cola
            .front()
            .is_some_and(|t| ahora.duration_since(*t) >= ventana)
        {
            cola.pop_front();
        }
    }

    /// `Some(espera)` si la clave está bloqueada.
    pub fn bloqueado(&self, clave: &str) -> Option<Duration> {
        let ahora = Instant::now();
        let mut m = self.fallos.lock().expect("mutex limitador");
        let cola = m.get_mut(clave)?;
        Self::podar(cola, self.ventana, ahora);
        (cola.len() >= self.max_por_clave)
            .then(|| self.ventana - ahora.duration_since(*cola.front().expect("no vacía")))
    }

    pub fn registrar_fallo(&self, clave: &str) {
        let ahora = Instant::now();
        let mut m = self.fallos.lock().expect("mutex limitador");
        // Limpieza oportunista para acotar memoria.
        if m.len() > 10_000 {
            m.retain(|_, c| {
                Self::podar(c, self.ventana, ahora);
                !c.is_empty()
            });
        }
        let cola = m.entry(clave.to_string()).or_default();
        Self::podar(cola, self.ventana, ahora);
        cola.push_back(ahora);
    }

    pub fn limpiar(&self, clave: &str) {
        self.fallos.lock().expect("mutex limitador").remove(clave);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bloquea_tras_max_fallos_y_libera_al_limpiar() {
        let l = Limitador::new(Duration::from_secs(60), 3);
        for _ in 0..2 {
            l.registrar_fallo("a");
        }
        assert!(l.bloqueado("a").is_none());
        l.registrar_fallo("a");
        let espera = l.bloqueado("a").unwrap();
        assert!(espera <= Duration::from_secs(60) && espera > Duration::from_secs(55));
        assert!(l.bloqueado("b").is_none());
        l.limpiar("a");
        assert!(l.bloqueado("a").is_none());
    }

    #[test]
    fn ventana_expira() {
        let l = Limitador::new(Duration::from_millis(30), 1);
        l.registrar_fallo("a");
        assert!(l.bloqueado("a").is_some());
        std::thread::sleep(Duration::from_millis(40));
        assert!(l.bloqueado("a").is_none());
    }
}
