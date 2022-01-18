#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use serde::Deserialize;

pub const P_COEFFS_COUNT: usize = 16;
pub const T_COEFFS_COUNT: usize = 5;

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct P16Coeffs {
    pub Fp0: f32,
    pub Ft0: f32,
    pub A: [f32; P_COEFFS_COUNT],
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct T5Coeffs {
    pub F0: f32,
    pub T0: f32,
    pub C: [f32; T_COEFFS_COUNT],
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct WorkRange {
    pub minimum: f32,
    pub maximum: f32,
    pub absolute_maximum: Option<f32>,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct CalibrationDate {
    pub Day: u32,
    pub Month: u32,
    pub Year: u32,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct WriteConfig {
    pub BaseInterval_ms: u32,
    pub PWriteDevider: u32,
    pub TWriteDevider: u32,
}

#[repr(packed(1))]
#[derive(Debug, Copy, Clone, Deserialize, Default)]
pub(crate) struct Monitoring {
    pub Ovarpress: bool,
    pub Ovarheat: bool,
    pub CPUOvarheat: bool,
    pub OverPower: bool,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(crate) enum PressureMeassureUnits {
    INVALID_ZERO = 0,

    // Паскали
    Pa = 0x00220000,

    // Бар
    Bar = 0x004E0000,

    // Ат (кгс/см2)
    At = 0x00A10000,

    // мм водного столба
    mmH20 = 0x00A20000,

    // м. ртутного столба
    mHg = 0x00A30000,

    // Атм
    Atm = 0x00A40000,

    // Фунт на квадратный дюйм
    PSI = 0x00AB0000,
}

#[derive(Debug, Copy, Clone, Deserialize)]
pub(crate) struct AppSettings {
    pub Serial: u32,
    pub PMesureTime_ms: u32,
    pub TMesureTime_ms: u32,

    pub Fref: u32,

    pub P_enabled: bool,
    pub T_enabled: bool,
    pub TCPUEnabled: bool,
    pub VBatEnabled: bool,

    pub P_Coefficients: P16Coeffs,
    pub T_Coefficients: T5Coeffs,

    pub PWorkRange: WorkRange,
    pub TWorkRange: WorkRange,
    pub TCPUWorkRange: WorkRange,
    pub VbatWorkRange: WorkRange,

    pub PZeroCorrection: f32,
    pub TZeroCorrection: f32,

    pub calibration_date: CalibrationDate,

    pub writeConfig: WriteConfig,

    pub startDelay: u32,

    pub pressureMeassureUnits: PressureMeassureUnits,

    pub monitoring: Monitoring,
}

impl Monitoring {
    pub fn is_set(&self) -> bool {
        self.Ovarpress | self.Ovarheat | self.CPUOvarheat | self.OverPower
    }
}
