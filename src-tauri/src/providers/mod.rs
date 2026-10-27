//! Providers: cada uno vigila una fuente del sistema y publica actividades en el
//! [`crate::activity::ActivityHub`]. Corren en sus propios hilos para que una API lenta
//! o colgada de Windows no frene a las demás.

pub mod media;
