use cgmath::Vector3;

pub fn normalize_vertices(vertices: &mut [f32]) {
    let mut max = f32::MIN;
    let mut min = f32::MAX;
    for x in vertices.iter() {
        if *x > max {
            max = *x;
        }
        if *x < min {
            min = *x;
        }
    }
    for x in vertices.iter_mut() {
        *x = (*x - min) / (max - min) * 2.0 - 1.0;
    }
}

pub fn get_center(vertices: &[f32]) -> Vector3<f32> {
    let mut min_x = vertices[0];
    let mut min_y = vertices[1];
    let mut min_z = vertices[2];
    let mut max_x = vertices[0];
    let mut max_y = vertices[1];
    let mut max_z = vertices[2];

    let num_vertices = vertices.len() / 3;
    for i in 1..num_vertices {
        let x = vertices[3 * i];
        if x < min_x {
            min_x = x;
        }
        if x > max_x {
            max_x = x;
        }
        let y = vertices[3 * i + 1];
        if y < min_y {
            min_y = y;
        }
        if y > max_y {
            max_y = y;
        }
        let z = vertices[3 * i + 2];
        if z < min_z {
            min_z = z;
        }
        if z > max_z {
            max_z = z;
        }
    }

    Vector3::new(
        (min_x + max_x) / 2.0,
        (min_y + max_y) / 2.0,
        (min_z + max_z) / 2.0,
    )
}
