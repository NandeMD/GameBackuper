use chrono::prelude::*;
use colored::Colorize;
use flate2::write::GzEncoder;
use flate2::Compression;
use ini::Ini;
use std::collections::HashMap;
use std::fs::{read_dir, File};
use std::path::{Path, PathBuf};
use std::thread::sleep;
use std::time::Duration;

const TIME_FORMAT: &str = "%d-%m-%Y %H_%M_%S";
const TIME_FORMAT_HUMAN: &str = "%d-%m-%Y %H:%M:%S";
const PRINT_SPACES: usize = 15;

fn load_ini_config(
) -> Result<(Duration, PathBuf, usize, HashMap<String, PathBuf>), Box<dyn std::error::Error>> {
    let ini = Ini::load_from_file("Config.ini")?;

    let interval = Duration::from_secs({
        let interval_str = &ini.general_section()["intervalminutes"];
        interval_str.parse::<u64>()? * 60
    });
    let outdir = &ini.general_section()["outdir"];

    let path = PathBuf::from(outdir);
    if !path.exists() {
        return Err(format_err_str("Output directory does not exist").into());
    }
    let outdir_path = path;

    let max_backups = &ini.general_section()["maxbackupsperworld"].parse::<usize>()?;

    let worlds_section = ini
        .section(Some("Worlds"))
        .ok_or(format_err_str("No 'Worlds' section found"))?;
    let mut worlds: HashMap<String, PathBuf> = HashMap::new();
    for (key, value) in worlds_section.iter() {
        let world_path = PathBuf::from(value);
        if world_path.exists() {
            worlds.insert(key.to_string(), world_path);
        } else {
            return Err(format_err_str(format!(
                "World directory {} does not exist for world {}",
                value, key
            ))
            .into());
        }
    }

    Ok((interval, outdir_path, *max_backups, worlds))
}

fn backup_worlds(
    now: &DateTime<Local>,
    outdir_path: impl AsRef<Path>,
    worlds: &HashMap<String, PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    for (world_name, world_path) in worlds {
        print_info(format!("Backing up world: {}", world_name.cyan()));
        let output_file = outdir_path.as_ref().join(format!(
            "World Backup {} {}.tar.gz",
            world_name,
            now.format(TIME_FORMAT)
        ));
        encode_as_tar_gz(&output_file, world_path);

        print_done(format!(
            "Backup completed: {}",
            output_file.display().to_string().cyan()
        ));
    }

    Ok(())
}

fn remove_old_backups(
    outdir_path: impl AsRef<Path>,
    worlds: &HashMap<String, PathBuf>,
    max_backups: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let entries = read_dir(&outdir_path)?
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|de| {
            de.file_type().is_ok()
                && de.file_type().unwrap().is_file()
                && de.file_name().to_str().unwrap().ends_with(".tar.gz")
        })
        .map(|de| de.path())
        .collect::<Vec<_>>();

    for (world_name, _) in worlds {
        // Filter out files that are not for the current world
        let mut this_world_files: Vec<PathBuf> = entries
            .iter()
            .filter(|p| {
                p.file_stem()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .contains(world_name)
            })
            .cloned()
            .collect();
        this_world_files.sort_by_key(|pb| pb.display().to_string());

        // Keep the latest max_backups backups
        if this_world_files.len() > max_backups {
            for i in 0..this_world_files.len() - max_backups {
                if let Err(e) = std::fs::remove_file(&this_world_files[i]) {
                    eprintln!(
                        "{}",
                        format_err_str(format!("Failed to remove old backup file: {}", e))
                    );
                }
                print_done(format!(
                    "Removed old backup file: {}",
                    this_world_files[i].display().to_string().cyan()
                ));
            }
        }
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (mut interval, mut outdir_path, mut max_backups, mut worlds) = load_ini_config()?;

    print_info(format!("{:?}", interval));
    print_info(format!("Output Directory: {}", outdir_path.display()));
    print_info(format!("Max Backups per World: {}", max_backups));
    print_info(format!("Found {} worlds", worlds.len()));
    println!();

    loop {
        (interval, outdir_path, max_backups, worlds) = load_ini_config()?;
        let now = Local::now();
        print_info(format!(
            "Backuping at: {}",
            now.format(TIME_FORMAT_HUMAN).to_string().cyan()
        ));

        backup_worlds(&now, &outdir_path, &worlds)?;

        remove_old_backups(&outdir_path, &worlds, max_backups)?;

        print_info(format!(
            "Sleeping for {} seconds...",
            interval.as_secs().to_string().cyan()
        ));
        println!();
        sleep(interval);
    }
}

pub fn encode_as_tar_gz(out_path: impl AsRef<Path>, data_dir: impl AsRef<Path>) {
    let tarball = File::create(out_path.as_ref()).unwrap();
    let enc = GzEncoder::new(tarball, Compression::best());
    let mut tar = tar::Builder::new(enc);
    tar.append_dir_all("7DaysToDieServer_Data", data_dir.as_ref())
        .unwrap();
    tar.finish().unwrap();
}

fn print_info(text: impl AsRef<str>) {
    println!(
        "{}{}{}",
        "[INFO]".yellow().bold(),
        " ".repeat(PRINT_SPACES - "[INFO]".len()),
        text.as_ref()
    );
}

fn format_err_str(err_str: impl AsRef<str>) -> String {
    format!(
        "{}{}{}",
        "[ERROR]".red().bold(),
        " ".repeat(PRINT_SPACES - "[ERROR]".len()),
        err_str.as_ref().purple().italic(),
    )
}

fn print_done(text: impl AsRef<str>) {
    println!(
        "{}{}{}",
        "[DONE]".green().bold(),
        " ".repeat(PRINT_SPACES - "[DONE]".len()),
        text.as_ref()
    );
}
