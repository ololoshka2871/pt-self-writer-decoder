use std::{
    fs::File,
    io::{Result, Write},
    path::Path,
    time::Duration,
};

use self_recorder_packet::{PageData, PrettyDuration, Record};

pub(crate) fn save_page_report<P: AsRef<Path>>(
    page: &PageData,
    save_freq: bool,
    file: P,
    settings: &crate::app_settings::AppSettings,
) -> Result<()> {
    let mut file = File::create(file)?;

    let write_data_string = move |f: &mut File, ts, fp, ft| -> Result<()> {
        f.write_fmt(format_args!(
            "{};{:.6};{:.6}",
            PrettyDuration(ts),
            calc_p(
                fp,
                ft,
                &settings.P_Coefficients,
                settings.pressureMeassureUnits,
                settings.P_enabled,
                settings.T_enabled,
            ),
            calc_t(ft, &settings.T_Coefficients, settings.T_enabled,)
        ))?;

        if save_freq {
            f.write_fmt(format_args!(";{:.6};{:.6}", fp, ft))?;
        }
        f.write("\n".as_bytes())?;

        Ok(())
    };

    if page.header.is_initial() {
        file.write_fmt(format_args!(
            "Стартовая страница;{}\n",
            page.header.this_block_id
        ))?;
    } else {
        file.write_fmt(format_args!(
            "Страница {};предыдущая {}\n",
            page.header.this_block_id, page.header.prev_block_id
        ))?;
    }

    let start = Duration::from_millis(page.header.timestamp);
    file.write_fmt(format_args!(
        "Время начала страницы;{}\n",
        PrettyDuration(start)
    ))?;
    file.write_fmt(format_args!(
        "Базовый интервал;{};мс.\n",
        page.header.base_interval_ms
    ))?;
    file.write_fmt(format_args!(
        "Температура процессора;{};*С.\nЗаряд батареи;{};В\n",
        page.header.t_cpu, page.header.v_bat
    ))?;

    file.write("\n".as_bytes())?;

    file.write("Время;".as_bytes())?;
    file.write_fmt(format_args!(
        "Давление, {:?};Температура, *С;",
        settings.pressureMeassureUnits
    ))?;
    if save_freq {
        file.write("Частота давления, Гц;Частота температуры, Гц".as_bytes())?;
    }
    file.write("\n".as_bytes())?;

    {
        let mut p_iter = page.fp.iter();
        let mut t_iter = page.ft.iter();
        let mut c_fp = if let Some(fp) = p_iter.next() {
            *fp
        } else {
            Record::default()
        };
        let mut c_ft = if let Some(ft) = t_iter.next() {
            *ft
        } else {
            Record::default()
        };

        write_data_string(
            &mut file,
            Duration::from_millis(page.header.timestamp),
            c_fp.freq,
            c_ft.freq,
        )?;

        for i in 1.. {
            let timstamp = Duration::from_millis(
                page.header.timestamp + (i * page.header.base_interval_ms) as u64,
            );
            let mut has_result = false;

            if i % page.header.interleave_ratio[0] == 0 {
                if let Some(fp) = p_iter.next() {
                    c_fp = *fp;
                    has_result |= true;
                } else {
                    break;
                }
            }

            if i % page.header.interleave_ratio[1] == 0 {
                if let Some(ft) = t_iter.next() {
                    c_ft = *ft;
                    has_result |= true;
                } else {
                    break;
                }
            }

            if has_result {
                write_data_string(&mut file, timstamp, c_fp.freq, c_ft.freq)?;
            }
        }
    }

    Ok(())
}

fn calc_p(
    fp: f32,
    ft: f32,
    coeffs: &crate::app_settings::P16Coeffs,
    mu: crate::app_settings::PressureMeassureUnits,
    p_enabled: bool,
    t_enabled: bool,
) -> f32 {
    if p_enabled {
        let presf_minus_fp0 = fp as f64 - coeffs.Fp0 as f64;
        let ft_minus_ft0 = if !t_enabled || ft.is_nan() {
            0.0f64
        } else {
            ft as f64 - coeffs.Ft0 as f64
        };

        let a = &coeffs.A;

        let k0 = a[0] as f64
            + ft_minus_ft0
                * (a[1] as f64 + ft_minus_ft0 * (a[2] as f64 + ft_minus_ft0 * a[12] as f64));
        let k1 = a[3] as f64
            + ft_minus_ft0
                * (a[5] as f64 + ft_minus_ft0 * (a[7] as f64 + ft_minus_ft0 * a[13] as f64));
        let k2 = a[4] as f64
            + ft_minus_ft0
                * (a[6] as f64 + ft_minus_ft0 * (a[8] as f64 + ft_minus_ft0 * a[14] as f64));
        let k3 = a[9] as f64
            + ft_minus_ft0
                * (a[10] as f64 + ft_minus_ft0 * (a[11] as f64 + ft_minus_ft0 * a[15] as f64));

        let p = k0 + presf_minus_fp0 * (k1 + presf_minus_fp0 * (k2 + presf_minus_fp0 * k3));

        wrap_mu(p, mu) as f32
    } else {
        f32::NAN
    }
}

fn wrap_mu(p: f64, mu: crate::app_settings::PressureMeassureUnits) -> f64 {
    use crate::app_settings::PressureMeassureUnits;

    let multiplier = match mu {
        PressureMeassureUnits::INVALID_ZERO => panic!(),
        PressureMeassureUnits::Pa => 100000.0,
        PressureMeassureUnits::Bar => 1.0,
        PressureMeassureUnits::At => 1.0197162,
        PressureMeassureUnits::mmH20 => 10197.162,
        PressureMeassureUnits::mHg => 750.06158 / 1000.0,
        PressureMeassureUnits::Atm => 0.98692327,
        PressureMeassureUnits::PSI => 14.5,
    };

    p * multiplier
}

fn calc_t(f: f32, coeffs: &crate::app_settings::T5Coeffs, t_enabled: bool) -> f32 {
    if t_enabled {
        let temp_f_minus_fp0 = f as f64 - coeffs.F0 as f64;
        let mut result = coeffs.T0 as f64;
        let mut mu = temp_f_minus_fp0;

        for i in 0..3 {
            result += mu * coeffs.C[i] as f64;
            mu *= temp_f_minus_fp0;
        }

        result as f32
    } else {
        f32::NAN
    }
}
