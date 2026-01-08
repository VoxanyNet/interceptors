use nalgebra::Vector2;

/// Connections are simple event listeners and handlers that allow some custom logic to be added to entity definitions
pub struct Connection {

}

pub enum Event {

}

pub enum Input {
    Teleport(Vector2<f32>)
}