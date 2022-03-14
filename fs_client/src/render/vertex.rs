use fs_common::game::common::world::material::Color;


#[derive(Copy, Clone)]
pub struct Vertex2 {
    pub position: [f32; 2],
}

glium::implement_vertex!(Vertex2, position);

impl From<[f32; 2]> for Vertex2 {
    fn from(position: [f32; 2]) -> Self {
        Vertex2 { position }
    }
}

impl From<(f32, f32)> for Vertex2 {
    fn from(v: (f32, f32)) -> Self {
        Vertex2 { position: [v.0, v.1] }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex2T {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
}

glium::implement_vertex!(Vertex2T, position, tex_coord);

impl From<((f32, f32), (f32, f32))> for Vertex2T {
    fn from(v: ((f32, f32), (f32, f32))) -> Self {
        Vertex2T { position: [v.0.0, v.0.1], tex_coord: [v.1.0, v.1.1] }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex2TA {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
    pub tex_layer: f32,
}

glium::implement_vertex!(Vertex2TA, position, tex_coord, tex_layer);

impl From<((f32, f32), (f32, f32), f32)> for Vertex2TA {
    fn from(v: ((f32, f32), (f32, f32), f32)) -> Self {
        Self { position: [v.0.0, v.0.1], tex_coord: [v.1.0, v.1.1], tex_layer: v.2 }
    }
}

#[derive(Copy, Clone)]
pub struct Vertex2C {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

glium::implement_vertex!(Vertex2C, position, color);

impl From<((f32, f32), Color)> for Vertex2C {
    fn from(v: ((f32, f32), Color)) -> Self {
        Vertex2C { position: [v.0.0, v.0.1], color: v.1.into() }
    }
}

impl From<(Vertex2, Color)> for Vertex2C {
    fn from(v: (Vertex2, Color)) -> Self {
        Vertex2C { position: v.0.position, color: v.1.into() }
    }
}

impl From<(Vertex2, [f32; 2])> for Vertex2T {
    fn from(v: (Vertex2, [f32; 2])) -> Self {
        Vertex2T { position: v.0.position, tex_coord: v.1 }
    }
}