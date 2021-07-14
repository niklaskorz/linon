#[derive(Debug, PartialEq)]
pub enum PredefinedFunction {
    Custom,
    TranslationX,
    TranslationZ,
    Rotation,
    LorenzAttractor,
    RoesslerAttractor,
}

impl ToString for PredefinedFunction {
    fn to_string(&self) -> String {
        match self {
            Self::Custom => "Custom".to_string(),
            Self::TranslationX => "Translation (x-axis)".to_string(),
            Self::TranslationZ => "Translation (z-axis)".to_string(),
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
            Self::TranslationX => "let dx = 0.5 * t;
let dy = 0.0;
let dz = 0.0;
return translate(v0, dx, dy, dz);"
                .to_string(),
            Self::TranslationZ => "let dx = 0.0;
let dy = 0.0;
let dz = 0.5 * t;
return translate(v0, dx, dy, dz);"
                .to_string(),
            Self::Rotation => "return normalize(rotateZ(v, PI / 2.0));".to_string(),
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
