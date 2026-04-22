use std::{
    fs::File,
    io::{Result, Write},
    path::Path,
};

use self_recorder_packet::{PageData, Record};

// Константа для преобразования из "год 0000" в UNIX EPOCH
// 0000-01-01 00:00:00.000 до 1970-01-01 00:00:00.000
const FROM_YEAR_ZERO_TO_UNIX_EPOCH_MS: i128 = 62167132800000;

pub fn format_timestamp<'a>(
    format: &'a Vec<time::format_description::BorrowedFormatItem<'a>>,
    ts: u64,
) -> String {
    let datetime = time::OffsetDateTime::from_unix_timestamp_nanos(
        (ts as i128 - FROM_YEAR_ZERO_TO_UNIX_EPOCH_MS) * 1_000_000,
    )
    .expect("Invalid timestamp");
    datetime.format(format).unwrap()
}

pub(crate) fn save_page_report<P: AsRef<Path>>(
    page: &PageData,
    save_freq: bool,
    file: P,
    settings: &crate::app_settings::AppSettings,
) -> Result<()> {
    let mut file = File::create(file)?;

    let format: Vec<time::format_description::BorrowedFormatItem<'_>> =
        time::format_description::parse(
            "[year].[month].[day] [hour]:[minute]:[second].[subsecond digits:3]",
        )
        .unwrap();

    let write_data_string = |f: &mut File, ts, fp, ft| -> Result<()> {
        f.write_fmt(format_args!(
            "{:?};{:.6};{:.6}",
            format_timestamp(&format, ts),
            settings.pressure_meassure_units.wrap(
                settings
                    .p_coefficients
                    .calc(Some(fp as f64), Some(ft as f64))
            ),
            settings.t_coefficients.calc(Some(ft as f64)),
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

    file.write_fmt(format_args!(
        "Время начала страницы;{}\n",
        format_timestamp(&format, page.header.timestamp)
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
        settings.pressure_meassure_units
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

        write_data_string(&mut file, page.header.timestamp, c_fp.freq, c_ft.freq)?;

        for i in 1.. {
            let timstamp = page.header.timestamp + (i * page.header.base_interval_ms) as u64;
            let mut has_result = false;

            if page.header.interleave_ratio[0] == 0 || page.header.interleave_ratio[1] == 0 {
                break;
            }

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
