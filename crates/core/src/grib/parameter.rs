#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GribParameter {
    pub discipline: u8,
    pub category: u8,
    pub number: u8,
}

pub(crate) const PARAM_U: GribParameter = GribParameter {
    discipline: 0,
    category: 2,
    number: 2,
};
pub(crate) const PARAM_V: GribParameter = GribParameter {
    discipline: 0,
    category: 2,
    number: 3,
};
pub(crate) const PARAM_T: GribParameter = GribParameter {
    discipline: 0,
    category: 0,
    number: 0,
};
pub(crate) const PARAM_H: GribParameter = GribParameter {
    discipline: 0,
    category: 3,
    number: 5,
};

pub(crate) const GRIB1_ISOBARIC_SURFACE: u8 = 100;

impl GribParameter {
    pub(crate) fn is_supported(&self) -> bool {
        matches!(*self, PARAM_U | PARAM_V | PARAM_T | PARAM_H)
    }

    pub(crate) fn from_grib1_parameter_number(number: u8) -> Option<(GribParameter, f64)> {
        match number {
            131 => Some((PARAM_U, 1.0)),
            132 => Some((PARAM_V, 1.0)),
            130 => Some((PARAM_T, 1.0)),
            129 => Some((PARAM_H, 0.1)),
            _ => None,
        }
    }
}
