// Copyright © 2018–2019 Sebastian Gottfried <sebastian.gottfried@posteo.de>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.

extern crate chrono;
extern crate clap;
extern crate rexiv2;
extern crate rayon;

use std::path::Path;
use std::os::unix::ffi::OsStrExt;
use rayon::prelude::*;

#[derive(PartialEq, Debug)]
struct ImgInfo {
    path: std::path::PathBuf,
    date: chrono::NaiveDateTime,
    model: String,
}

fn scan_for_images(dir: &Path) -> std::io::Result<Vec<ImgInfo>> {
    let mut entries: Vec<std::fs::DirEntry> = Vec:: new();
    for entry in std::fs::read_dir(dir)? { 
        let entry = entry?;
        if entry.path().file_name().unwrap().as_bytes()[0] == b'.' {
            continue;
        }
        entries.push(entry);
    }
    let mut imgs: Vec<ImgInfo> = entries.par_iter().filter_map(|ref entry| {
        match read_img(&entry) {
            Ok(img) => Some(img),
            Err(error) => {
                let path = entry.path();
                let path_str = path.to_str().unwrap();
                eprintln!("Error reading image metatada from {}: {}. Skipping.", path_str, error);
                None
            }
        }
    }).collect();
    imgs.sort_by(|a, b| a.date.cmp(&b.date));
    Ok(imgs)
}


fn read_img(entry: &std::fs::DirEntry) -> Result<ImgInfo, rexiv2::Rexiv2Error> {
    let date_tag = "Exif.Photo.DateTimeOriginal";
    let model_tag = "Exif.Image.Model";
    let metadata = rexiv2::Metadata::new_from_path(entry.path())?;
    let date_str = metadata.get_tag_string(date_tag)?;
    let model = metadata.get_tag_string(model_tag).unwrap();
    let date = chrono::NaiveDateTime::parse_from_str(&date_str, "%Y:%m:%d  %H:%M:%S").unwrap();
    let img = ImgInfo { path: entry.path(), date: date, model: model };
    Ok(img)
}

fn reorganize_images(imgs: &Vec<ImgInfo>, prefix: &str, dryrun: &bool) -> std::io::Result<()> {
    let digits = (imgs.len() as f32).log10().ceil() as usize;
    for (index, img) in imgs.iter().enumerate() {
        let old_path = &img.path;
        let parent = old_path.parent().unwrap();
        let date_str = img.date.format("%Y-%m-%d %H-%M-%S");
        let old_file_name = old_path.file_name().unwrap();
        let suffix = match old_path.extension() {
            Some(ext) => format!(".{}", ext.to_str().unwrap()),
            None => String::from("")
        };
        let new_file_name = if prefix.is_empty() {
            format!("{:0digits$} {} {}{}", index, date_str, img.model, suffix, digits=digits)
        }
        else {
            format!("{} {:0digits$} {} {}{}", prefix, index, date_str, img.model, suffix, digits=digits)
        };
        let new_path = parent.join(&new_file_name);
        if &new_path != old_path {
            println!("{}/{{{} -> {}}}", parent.to_str().unwrap(), old_file_name.to_str().unwrap(), new_file_name);
            if !dryrun {
                std::fs::rename(&old_path, &new_path)?
            }
        }
    }
    Ok(())
}

fn main() {
    let matches = clap::App::new("Organize images by date")
        .version("0.0.1")
        .author("Sebastian Gottfried <sebastian.gottfried@posteo.de>")
        .arg(clap::Arg::with_name("PATH")
            .help("Path to folder with images")
            .default_value(".")
            .index(1))
        .arg(clap::Arg::with_name("dryrun")
            .long("dryrun")
            .help("Do not write out changes. Just show what would happen."))
        .arg(clap::Arg::with_name("prefix")
            .long("prefix")
            .takes_value(true)
            .help("Sets a custom prefix for the generated image file names."))
        .get_matches();
    let path = Path::new(matches.value_of("PATH").unwrap());
    let dryrun = matches.is_present("dryrun");
    let prefix = matches.value_of("prefix").unwrap_or("");
    if dryrun {
        println!("Dry run. No changes will be written out.");
    }
    rexiv2::initialize().expect("Error initializing libexiv2");
    let imgs: Vec<ImgInfo> = match scan_for_images(path) {
        Ok(r)=> r,
        Err(error) => {
            panic!("Error: {:?}", error)
        },
    };
    match reorganize_images(&imgs, &prefix, &dryrun) {
        Ok(_) => (),
        Err(error) => {
            panic!("Error: {:?}", error)
        },
    }
}
