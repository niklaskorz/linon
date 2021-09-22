#[derive(Debug, PartialEq)]
pub enum PredefinedFunction {
    Custom,
    MirageSpherical,
    MiragePlane,
    MirageSphericalSigmoid,
    MiragePlaneSigmoid,
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
            Self::MirageSpherical => "Mirage (spherical)".to_string(),
            Self::MiragePlane => "Mirage (plane)".to_string(),
            Self::MirageSphericalSigmoid => "Mirage (spherical sigmoid)".to_string(),
            Self::MiragePlaneSigmoid => "Mirage (plane sigmoid)".to_string(),
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
            Self::MirageSpherical => "let t_env = 15.0; // °C
let t_src = 100.0; // °C
let max_dist = 0.25;
let center = vec3<f32>(-0.5, 0.5, -0.5);

let center_dest = p - center;
let normal = normalize(center_dest);
let dist_in = length(p_prev - center);
let dist_out = length(center_dest);
let part_in = clamp(0.0, 1.0, dist_in / max_dist);
let part_out = clamp(0.0, 1.0, dist_out / max_dist);
let t_in = part_in * t_env + (1.0 - part_in) * t_src;
let t_out = part_out * t_env + (1.0 - part_out) * t_src;

return refraction(t_in, t_out, v, normal);"
                .to_string(),
            Self::MiragePlane => "let t_env = 15.0; // °C
let t_src = 30.0; // °C
let max_dist = 0.01;
let plane_p0 = vec3<f32>(0.0, 0.1, 0.0);
let plane_n = vec3<f32>(0.0, 1.0, 0.0);

let dist_in = point_plane_distance(p_prev, plane_n, plane_p0);
let dist_out = point_plane_distance(p, plane_n, plane_p0);
let part_in = clamp(0.0, 1.0, dist_in / max_dist);
let part_out = clamp(0.0, 1.0, dist_out / max_dist);
let t_in = part_in * t_env + (1.0 - part_in) * t_src;
let t_out = part_out * t_env + (1.0 - part_out) * t_src;

return refraction(t_in, t_out, v, plane_n);"
                .to_string(),
            Self::MirageSphericalSigmoid => "let t_env = 15.0; // °C
let t_src = 100.0; // °C
let max_dist = 0.25;
let center = vec3<f32>(-0.5, 0.5, -0.5);

let center_dest = p - center;
let normal = normalize(center_dest);
let dist_in = length(p_prev - center);
let dist_out = length(center_dest);
let part_in = clamp(0.0, 1.0, sigmoid(dist_in / max_dist * 12.0 - 6.0));
let part_out = clamp(0.0, 1.0, sigmoid(dist_out / max_dist * 12.0 - 6.0));
let t_in = part_in * t_env + (1.0 - part_in) * t_src;
let t_out = part_out * t_env + (1.0 - part_out) * t_src;

return refraction(t_in, t_out, v, normal);"
                .to_string(),
            Self::MiragePlaneSigmoid => "let t_env = 15.0; // °C
let t_src = 30.0; // °C
let max_dist = 0.01;
let plane_p0 = vec3<f32>(0.0, 0.1, 0.0);
let plane_n = vec3<f32>(0.0, 1.0, 0.0);

let dist_in = point_plane_distance(p_prev, plane_n, plane_p0);
let dist_out = point_plane_distance(p, plane_n, plane_p0);
let part_in = clamp(0.0, 1.0, sigmoid(dist_in / max_dist * 12.0 - 6.0));
let part_out = clamp(0.0, 1.0, sigmoid(dist_out / max_dist * 12.0 - 6.0));
let t_in = part_in * t_env + (1.0 - part_in) * t_src;
let t_out = part_out * t_env + (1.0 - part_out) * t_src;

return refraction(t_in, t_out, v, plane_n);"
                .to_string(),
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
            Self::Rotation => "return normalize(v0 + rotateZ(v0, PI / 2.0 * t));".to_string(),
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
