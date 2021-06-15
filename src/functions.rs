#[derive(Debug, PartialEq)]
pub enum PredefinedFunction {
    Custom,
    Rotation,
    LorenzAttractor,
    RoesslerAttractor,
}

impl ToString for PredefinedFunction {
    fn to_string(&self) -> String {
        match self {
            Self::Custom => "Custom".to_string(),
            Self::Rotation => "Rotation".to_string(),
            Self::LorenzAttractor => "Lorenz attractor".to_string(),
            Self::RoesslerAttractor => "Roessler attractor".to_string(),
        }
    }
}

impl PredefinedFunction {
    pub fn to_code(&self) -> String {
        match self {
            Self::Custom => "".to_string(),
            Self::Rotation => "let phi = 1.5;
let A = rotate(phi);
let u = A * vec4<f32>(v, 1.0);
return u.xyz;"
                .to_string(),
            Self::LorenzAttractor => "let rho = 28.0;
let sigma = 10.0;
let beta = 8.0 / 3.0;
return vec3<f32>(
    sigma * (p.y - p.x),
    p.x * (rho - p.z) - p.y,
    p.x * p.y - beta * p.z,
);"
            .to_string(),
            Self::RoesslerAttractor => "let a = 0.1;
let b = 0.1;
let c = 14.0;
return vec3<f32>(
    -p.y - p.z,
    p.x + a * p.y,
    b + p.z * (p.x - c),
);"
            .to_string(),
        }
    }
}
