mod app_settings;
mod mem_info;
mod report_saver;

use std::{env, path::PathBuf};

use memmap;

use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator, ParallelIterator,
};

use self_recorder_packet::PageData;
use structopt::StructOpt;

const FULL_DUMP_FILE_NAME: &str = "data_raw.hs";
const USED_DUMP_FILE_NAME: &str = "data_use.hs";
const CONFIG_FILE_NAME: &str = "config.var";
const STORAGE_FILE_NAME: &str = "storage.var";

#[derive(Debug, StructOpt)]
#[structopt(about = "Decoder for data, collected via stm32-usb-self-writer")]
struct Cli {
    /// Input dirrectory [files: "config.var", "storage.var" and "data.hs"], default: current dirreectory
    #[structopt(parse(from_os_str))]
    src: Option<PathBuf>,

    /// Destination directory for output files, default: current dirreectory
    #[structopt(long, short, parse(from_os_str))]
    output: Option<PathBuf>,

    /// Process full dump file "data_raw.hs" instead of used "data_use.hs"
    #[structopt(long)]
    full: bool,

    /// Save frequencies
    #[structopt(long, short)]
    freq: bool,

    /// Verbose output
    #[structopt(long, short)]
    verbose: bool,
}

fn main() {
    let args = Cli::from_args();
    let current_dir = env::current_dir().expect("Can't get current dirrectory");

    let src = get_dir(&args.src, "Input", &current_dir);
    let dest = get_dir(&args.output, "Output", &current_dir);

    verify_input(&src, &dest, args.full)
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

    let unpacked_pages = {
        println!("Reading data, it may take several minutes...");
        let file = std::fs::File::open(src.join(if args.full {
            FULL_DUMP_FILE_NAME
        } else {
            USED_DUMP_FILE_NAME
        }))
        .expect("Failed to read data file");

        let data = unsafe {
            memmap::MmapOptions::new()
                .map(&file)
                .expect("Failed to memmap data file")
        };
        self_recorder_packet::unpack_pages(
            &data,
            storage_cfg.FlashPageSize as usize,
            settings.Fref as f32,
            false,
        )
    };

    let chains = {
        if args.verbose {
            println!("Detected data chains:");
        }
        let mut chains = Vec::<&[PageData]>::new();
        let chain_starts = detect_chains_starts(&unpacked_pages);

        // first -> last
        (0..chain_starts.len() - 1)
            .enumerate()
            .for_each(|(i, start)| {
                let stop = start + 1;
                let chain_name = i + 1;
                if args.verbose {
                    println!(
                        "chain-{} blocks: {} -> {}",
                        chain_name,
                        chain_starts[start],
                        chain_starts[stop] - 1
                    );
                }
                chains.push(&unpacked_pages[chain_starts[start]..chain_starts[stop] - 1]);

                create_chain_filder(&dest, chain_name);
            });
        // last -> end
        if args.verbose {
            println!(
                "chain-{} blocks: {} -> {}",
                chain_starts.len(),
                *chain_starts.last().unwrap(),
                unpacked_pages.len() - 1
            );
        }
        chains.push(&unpacked_pages[*chain_starts.last().unwrap()..unpacked_pages.len() - 1]);
        create_chain_filder(&dest, chain_starts.len());

        chains
    };

    chains.into_iter().enumerate().for_each(|(ch, chain)| {
        let chain_number = ch + 1;
        println!("Decoding chain: {}... ", chain_number);
        chain
            .into_par_iter()
            .enumerate()
            .for_each(|(n, page_data)| {
                if page_data.consistant {
                    let outpath = if n == 0 {
                        dest.join(format!(
                            "{}/{:06}+start.csv", // + чтобы при сортировке по имени всегда было выше цыфр
                            chain_folder(chain_number),
                            page_data.header.this_block_id,
                        ))
                    } else {
                        dest.join(format!(
                            "{}/{:06}.csv",
                            chain_folder(chain_number),
                            page_data.header.this_block_id,
                        ))
                    };
                    report_saver::save_page_report(
                        &page_data,
                        args.freq,
                        outpath.clone(),
                        &settings,
                    )
                    .expect("Faild to save page");
                    if args.verbose {
                        println!("Decoded page: {}... => {:?}", n, outpath);
                    }
                } else if page_data.header.this_block_id != u32::MAX
                    && page_data.header.prev_block_id != u32::MAX
                {
                    std::fs::write(
                        dest.join(format!(
                            "{}/{}-corrupted-0x{:08X}.csv",
                            chain_folder(chain_number),
                            n,
                            page_data.header.data_crc32,
                        )),
                        b"data corrupted",
                    )
                    .expect("Failed to write file");
                    println!("page corrupted!");
                } else {
                    println!("No data");
                }
            })
    });
}

fn detect_chains_starts(unpacked_pages: &Vec<PageData>) -> Vec<usize> {
    unpacked_pages
        .par_iter()
        .enumerate()
        .filter_map(|(i, page)| {
            if page.header.prev_block_id == 0 && page.header.this_block_id == 0 {
                Some(i)
            } else {
                None
            }
        })
        .collect::<Vec<usize>>()
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

fn verify_input(src_dir: &PathBuf, dest_dir: &PathBuf, full: bool) -> Result<(), String> {
    if !src_dir.is_dir() {
        return Err(format!(
            "Dirrectory {:?} is not exists or not a dirrectory",
            src_dir
        ));
    }
    let data_file_path = src_dir.join(if full {
        FULL_DUMP_FILE_NAME
    } else {
        USED_DUMP_FILE_NAME
    });
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
            dest_dir
        ));
    }
    Ok(())
}
fn chain_folder(chain: usize) -> String {
    format!("chain-{}", chain)
}

fn create_chain_filder(dest: &PathBuf, chain: usize) {
    std::fs::create_dir(dest.join(chain_folder(chain)))
        .expect(format!("Failed to create dirrectory {}", chain_folder(chain)).as_str())
}
