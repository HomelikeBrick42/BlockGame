use encase::ShaderType;

#[derive(Debug, Clone, Copy, ShaderType)]
pub struct Point {
    pub e012: f32,
    pub e013: f32,
    pub e023: f32,
    pub e123: f32,
}

impl Point {
    pub const IDENTITY: Self = Self {
        e012: 0.0,
        e013: 0.0,
        e023: 0.0,
        e123: 1.0,
    };

    pub fn transform(self, motor: Motor) -> Self {
        let a = motor.s;
        let b = motor.e12;
        let c = motor.e13;
        let d = motor.e23;
        let e = motor.e01;
        let f = motor.e02;
        let g = motor.e03;
        let h = motor.e0123;
        let i = self.e012;
        let j = self.e013;
        let k = self.e023;
        let l = self.e123;

        /*
        Apply motor to point

        (a + b*e2*e1 + c*e3*e1 + d*e3*e2 + e*e1*e0 + f*e2*e0 + g*e3*e0 + h*e3*e2*e1*e0)
        *(i*e0*e1*e2 + j*e0*e1*e3 + k*e0*e2*e3 + l*e1*e2*e3)
        *(a + b*e1*e2 + c*e1*e3 + d*e2*e3 + e*e0*e1 + f*e0*e2 + g*e0*e3 + h*e0*e1*e2*e3)

        (
              -2*a*d*j + -2*a*g*l +   a*a*i + 2*a*c*k
            + -1*d*d*i + -2*d*f*l + 2*b*d*k + -2*b*h*l
            + -2*c*e*l +    b*b*i + 2*b*c*j + -1*c*c*i
        )*e0*e1*e2
        +
        (
              -2*a*b*k + -1*b*b*j + 2*b*c*i +  2*b*e*l
            +    a*a*j +  2*a*d*i + 2*a*f*l + -2*c*h*l
            + -2*d*g*l + -1*d*d*j + 2*c*d*k +    c*c*j
        )*e0*e1*e3
        +
        (
              -2*a*c*i + -2*a*e*l +   a*a*k +  2*a*b*j
            + -1*c*c*k +  2*c*d*j + 2*c*g*l + -2*d*h*l
            +  2*b*f*l + -1*b*b*k + 2*b*d*i +    d*d*k
        )*e0*e2*e3
        +
        (
            a*a*l + b*b*l + c*c*l + d*d*l
        )*e1*e2*e3

        */

        Self {
            e012: -2.0 * a * d * j
                + -2.0 * a * g * l
                + 1.0 * a * a * i
                + 2.0 * a * c * k
                + -1.0 * d * d * i
                + -2.0 * d * f * l
                + 2.0 * b * d * k
                + -2.0 * b * h * l
                + -2.0 * c * e * l
                + 1.0 * b * b * i
                + 2.0 * b * c * j
                + -1.0 * c * c * i,
            e013: -2.0 * a * b * k
                + -1.0 * b * b * j
                + 2.0 * b * c * i
                + 2.0 * b * e * l
                + 1.0 * a * a * j
                + 2.0 * a * d * i
                + 2.0 * a * f * l
                + -2.0 * c * h * l
                + -2.0 * d * g * l
                + -1.0 * d * d * j
                + 2.0 * c * d * k
                + 1.0 * c * c * j,
            e023: -2.0 * a * c * i
                + -2.0 * a * e * l
                + 1.0 * a * a * k
                + 2.0 * a * b * j
                + -1.0 * c * c * k
                + 2.0 * c * d * j
                + 2.0 * c * g * l
                + -2.0 * d * h * l
                + 2.0 * b * f * l
                + -1.0 * b * b * k
                + 2.0 * b * d * i
                + 1.0 * d * d * k,
            e123: a * a * l + b * b * l + c * c * l + d * d * l,
        }
    }
}

impl From<cgmath::Vector3<f32>> for Point {
    fn from(value: cgmath::Vector3<f32>) -> Self {
        Self {
            e012: value.z,
            e013: -value.y,
            e023: value.x,
            e123: 1.0,
        }
    }
}

impl From<Point> for cgmath::Vector3<f32> {
    fn from(value: Point) -> Self {
        Self {
            x: value.e023 / value.e123,
            y: -value.e013 / value.e123,
            z: value.e012 / value.e123,
        }
    }
}

#[derive(Debug, Clone, Copy, ShaderType)]
pub struct Motor {
    pub s: f32,
    pub e12: f32,
    pub e13: f32,
    pub e23: f32,
    pub e01: f32,
    pub e02: f32,
    pub e03: f32,
    pub e0123: f32,
}

impl Motor {
    pub const IDENTITY: Self = Self {
        s: 1.0,
        e12: 0.0,
        e13: 0.0,
        e23: 0.0,
        e01: 0.0,
        e02: 0.0,
        e03: 0.0,
        e0123: 0.0,
    };

    pub fn translation(offset: cgmath::Vector3<f32>) -> Self {
        Self {
            s: 1.0,
            e12: 0.0,
            e13: 0.0,
            e23: 0.0,
            e01: offset.x * -0.5,
            e02: offset.y * -0.5,
            e03: offset.z * -0.5,
            e0123: 0.0,
        }
    }

    pub fn rotation_xy(angle: f32) -> Self {
        let (sin, cos) = (angle * 0.5).sin_cos();
        Self {
            s: cos,
            e12: sin,
            e13: 0.0,
            e23: 0.0,
            e01: 0.0,
            e02: 0.0,
            e03: 0.0,
            e0123: 0.0,
        }
    }

    pub fn rotation_xz(angle: f32) -> Self {
        let (sin, cos) = (angle * 0.5).sin_cos();
        Self {
            s: cos,
            e12: 0.0,
            e13: sin,
            e23: 0.0,
            e01: 0.0,
            e02: 0.0,
            e03: 0.0,
            e0123: 0.0,
        }
    }

    pub fn rotation_yz(angle: f32) -> Self {
        let (sin, cos) = (angle * 0.5).sin_cos();
        Self {
            s: cos,
            e12: 0.0,
            e13: 0.0,
            e23: sin,
            e01: 0.0,
            e02: 0.0,
            e03: 0.0,
            e0123: 0.0,
        }
    }

    pub fn apply(self, other: Self) -> Self {
        let a = self.s;
        let b = self.e12;
        let c = self.e13;
        let d = self.e23;
        let e = self.e01;
        let f = self.e02;
        let g = self.e03;
        let h = self.e0123;
        let i = other.s;
        let j = other.e12;
        let k = other.e13;
        let l = other.e23;
        let m = other.e01;
        let n = other.e02;
        let o = other.e03;
        let p = other.e0123;

        /*
        Combining Motors

        (a + b*e1*e2 + c*e1*e3 + d*e2*e3 + e*e0*e1 + f*e0*e2 + g*e0*e3 + h*e0*e1*e2*e3)
        *(i + j*e1*e2 + k*e1*e3 + l*e2*e3 + m*e0*e1 + n*e0*e2 + o*e0*e3 + p*e0*e1*e2*e3)

        -1*b*j + -1*c*k + -1*d*l + a*i
        + (-1*c*l + a*j + b*i + d*k)*e1*e2
        + (-1*d*j + a*k + b*l + c*i)*e1*e3
        + (-1*b*k + a*l + c*j + d*i)*e2*e3
        + (-1*d*p + -1*f*j + -1*g*k + -1*h*l + a*m + b*n + c*o + e*i)*e0*e1
        + (-1*b*m + -1*g*l + a*n + c*p + d*o + e*j + f*i + h*k)*e0*e2
        + (-1*b*p + -1*c*m + -1*d*n + -1*h*j + a*o + e*k + f*l + g*i)*e0*e3
        + (-1*c*n + -1*f*k + a*p + b*o + d*m + e*l + g*j + h*i)*e0*e1*e2*e3
        */

        Self {
            s: -b * j + -c * k + -d * l + a * i,
            e12: -c * l + a * j + b * i + d * k,
            e13: -d * j + a * k + b * l + c * i,
            e23: -b * k + a * l + c * j + d * i,
            e01: -d * p + -f * j + -g * k + -h * l + a * m + b * n + c * o + e * i,
            e02: -b * m + -g * l + a * n + c * p + d * o + e * j + f * i + h * k,
            e03: -b * p + -c * m + -d * n + -h * j + a * o + e * k + f * l + g * i,
            e0123: -c * n + -f * k + a * p + b * o + d * m + e * l + g * j + h * i,
        }
    }

    pub fn pre_apply(self, other: Self) -> Self {
        other.apply(self)
    }

    pub fn inverse(self) -> Self {
        Self {
            s: self.s,
            e12: -self.e12,
            e13: -self.e13,
            e23: -self.e23,
            e01: -self.e01,
            e02: -self.e02,
            e03: -self.e03,
            e0123: self.e0123,
        }
    }
}
