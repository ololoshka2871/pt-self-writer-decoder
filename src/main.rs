mod app_settings;
mod mem_info;

use std::{env, path::PathBuf};

use structopt::StructOpt;

const DATA_FILE_NAME: &str = "data.hs";
const CONFIG_FILE_NAME: &str = "config.var";
const STORAGE_FILE_NAME: &str = "storage.var";

#[derive(Debug, StructOpt)]
#[structopt(about = "Decoder for data, collected via stm32-usb-self-writer")]
struct Cli {
    /// Input dirrectory [files: "config.var", "storage.var" and "data.hs"], default: current dirreectory
    #[structopt(parse(from_os_str))]
    src: Option<PathBuf>,

    /// Destination dirrectory for output files, default: current dirreectory
    #[structopt(long, short, parse(from_os_str))]
    dest: Option<PathBuf>,
}

fn get_dir(p: &Option<PathBuf>, direction: &str, current_dir: &PathBuf) -> PathBuf {
    if let Some(src) = p {
        println!("{} dirrectory: {:?}", direction, src);
        src.clone()
    } else {
        println!(
            "{} dirrctory not specified, use current: {:?}",
            direction, current_dir
        );
        current_dir.clone()
    }
}

fn verify_input(src_dir: &PathBuf, dest_dir: &PathBuf) -> Result<(), String> {
    if !src_dir.is_dir() {
        return Err(format!(
            "Dirrectory {:?} is not exists or not a dirrectory",
            src_dir
        ));
    }
    let data_file_path = src_dir.join(DATA_FILE_NAME);
    if !data_file_path.exists() {
        return Err(format!("Data file {:?} not found!", data_file_path));
    }
    let config_file_path = src_dir.join(CONFIG_FILE_NAME);
    if !config_file_path.exists() {
        return Err(format!(
            "Configuration file {:?} not found!",
            config_file_path
        ));
    }
    let storage_cfg_file_path = src_dir.join(STORAGE_FILE_NAME);
    if !storage_cfg_file_path.exists() {
        return Err(format!(
            "Storage configuration file {:?} not found!",
            config_file_path
        ));
    }
    if !dest_dir.is_dir() {
        return Err(format!(
            "Destination dirrectory {:?} is not found!",
            config_file_path
        ));
    }
    Ok(())
}

fn main() {
    let args = Cli::from_args();
    let current_dir = env::current_dir().expect("Can't get current dirrectory");

    let src = get_dir(&args.src, "Input", &current_dir);
    let dest = get_dir(&args.dest, "Output", &current_dir);

    verify_input(&src, &dest)
        .map_err(|e| panic!("{}", e))
        .unwrap();

    println!("Reading configuration...");
    let json_data = std::fs::read_to_string(src.join(CONFIG_FILE_NAME))
        .expect("Failed to read configuration file");
    let settings: app_settings::AppSettings =
        serde_json::from_str(json_data.as_str()).expect("Failed to parse configuration");
    let json_data = std::fs::read_to_string(src.join(STORAGE_FILE_NAME))
        .expect("Failed to read configuration file");
    let storage_cfg: mem_info::MemInfo =
        serde_json::from_str(json_data.as_str()).expect("Failed to parse storage configuration");

    println!("Reading data...");
    let data = std::fs::read(src.join(DATA_FILE_NAME)).expect("Failed to read data file");

    let unpacked_pages = self_recorder_packet::unpack_pages(
        data.as_slice(),
        storage_cfg.FlashPageSize as usize,
        settings.Fref as f32,
        true,
    );

    unpacked_pages.into_iter().for_each(|page| {
        print!("Decoding page: {}... ", page.header.this_block_id);
        if page.consistant {
            page.save_as_csv(dest.join(format!(
                "{}-0x{:08X}.csv",
                page.header.this_block_id, page.header.data_crc32,
            )))
            .expect("Faild to save page");
            println!("ok.");
        } else {
            std::fs::write(
                format!(
                    "{}-0x{:08X}-corrupted.csv",
                    page.header.this_block_id, page.header.data_crc32,
                ),
                b"data corrupted",
            )
            .expect("Failed to write file");
            println!("page corrupted!");
        }
    });
}
