//! Providers: cada uno vigila una fuente del sistema y publica actividades en el
//! [`crate::activity::ActivityHub`]. Corren en sus propios hilos para que una API lenta
//! o colgada de Windows no frene a las demás.

mod audio;
pub mod brightness;
pub mod keyboard;
pub mod media;
pub mod power;
pub mod privacy;
pub mod spectrum;
pub mod volume;
