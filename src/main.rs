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

use std::collections::HashMap;
use std::path::Path;
use std::os::unix::ffi::OsStrExt;
use rayon::prelude::*;

#[derive(PartialEq, Debug)]
struct ImgInfo {
    path: std::path::PathBuf,
    sidecar_path: Option<std::path::PathBuf>,
    date: chrono::NaiveDateTime,
    model: String,
}


#[derive(Debug, PartialEq, Eq, Hash)]
enum ImgRole {
    Raw,
    CameraJPG,
}

#[derive(Debug)]
#[derive(Default)]
struct ImgGroup {
    members: HashMap<ImgRole, ImgInfo>,
}

impl ImgInfo {
    fn base_path(&self) -> String {
        self.path.parent().unwrap().join(self.path.file_stem().unwrap()).into_os_string().into_string().unwrap()
    }

    fn role(&self) -> Option<ImgRole> {
        let extension = self.path.extension().unwrap().to_str().unwrap().to_ascii_lowercase();
        match extension.as_str() {
            "nef" => Some(ImgRole::Raw),
            "raf" => Some(ImgRole::Raw),
            "jpg" => Some(ImgRole::CameraJPG),
            _ => None
        }
    }
}

impl ImgGroup {
    fn first_img (&self) -> &ImgInfo {
        self.members.values().next().expect("can't get first image of empty group")
    }

    fn date(&self) -> &chrono::NaiveDateTime
    {
        &self.first_img().date
    }

    fn base_path(&self) -> String {
        self.first_img().base_path()
    }
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
    let imgs: Vec<ImgInfo> = entries.par_iter().filter_map(|ref entry| {
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
    Ok(imgs)
}

fn build_imgs_groups(imgs: Vec<ImgInfo>) -> Vec<ImgGroup> {
    let mut img_group_map: HashMap<String, ImgGroup> = HashMap::new();
    for img in imgs {
        let role = img.role().unwrap();
        let key = img.base_path();
        let group = img_group_map.entry(key).or_insert(Default::default());
        group.members.insert(role, img);
    }
    let mut img_groups: Vec<ImgGroup> = img_group_map.into_values().collect();
    img_groups.sort_by(|a, b| a.date().cmp(&b.date()).then(a.base_path().cmp(&b.base_path())));
    img_groups
}


fn read_img(entry: &std::fs::DirEntry) -> Result<ImgInfo, rexiv2::Rexiv2Error> {
    let date_tag = "Exif.Photo.DateTimeOriginal";
    let model_tag = "Exif.Image.Model";
    let metadata = rexiv2::Metadata::new_from_path(entry.path())?;
    let date_str = metadata.get_tag_string(date_tag)?;
    let model = metadata.get_tag_string(model_tag).unwrap();
    let date = chrono::NaiveDateTime::parse_from_str(&date_str, "%Y:%m:%d  %H:%M:%S").unwrap();
    let sidecar_path = find_sidecar_path(&entry.path());
    let img = ImgInfo { path: entry.path(), sidecar_path: sidecar_path, date: date, model: model };
    Ok(img)
}

fn find_sidecar_path(img_path: &Path) -> Option<std::path::PathBuf> {
    let sidecar_path = img_path.with_file_name(img_path.file_name().unwrap().to_str().unwrap().to_owned() + ".xmp");
    if sidecar_path.is_file() {
        return Some(sidecar_path);
    }
    return None;
}

fn reorganize_images(groups: &Vec<ImgGroup>, prefix: &str, dryrun: &bool) -> std::io::Result<()> {
    let digits = (groups.len() as f32).log10().ceil() as usize;
    for (index, group) in (1..).zip(groups.iter()) {
        for img in group.members.values() {
            rename_file(&img.path, index, &img, &prefix, digits, dryrun)?;
            match &img.sidecar_path {
                Some(path) => rename_file(path, index, &img, &prefix, digits, dryrun)?,
                None => (),
            }
        }
    }
    Ok(())
}

fn rename_file(src_path: &Path, index: usize, img: &ImgInfo, prefix: &str, index_digits: usize, dryrun: &bool) -> std::io::Result<( )> {
    let parent = src_path.parent().unwrap();
    let date_str = img.date.format("%Y-%m-%d %H-%M-%S");
    let src_file_name = src_path.file_name().unwrap().to_str().unwrap();
    let src_file_name_parts: Vec<&str> = src_file_name.split('.').collect();
    let suffix_parts = match src_file_name_parts.last() {
        Some(ext) => match ext.to_ascii_lowercase().as_str() {
            "xmp" => 2,
            _ => 1
        },
        _ => 0
    };
    let suffix = match suffix_parts {
        0 => String::from(""),
        i => format!(".{}", src_file_name_parts[src_file_name_parts.len()-i..].join("."))
    };
    let target_file_name = if prefix.is_empty() {
        format!("{:0digits$} {} {}{}", index, date_str, img.model, suffix, digits=index_digits)
    }
    else {
        format!("{:0digits$} {} {} {}{}", index, date_str, prefix, img.model, suffix, digits=index_digits)
    };
    let target_path = parent.join(&target_file_name);
    if &target_path != src_path {
        println!("{}/{{{} -> {}}}", parent.to_str().unwrap(), src_file_name, target_file_name);
        if !dryrun {
            std::fs::rename(&src_path, &target_path)?
        }
    }
    Ok(())
}

fn main() {
    let matches = clap::App::new("Organize images by date and type")
        .version("0.0.1")
        .author("Sebastian Gottfried <sebastian.gottfried@posteo.de>")
        .arg(clap::Arg::with_name("PATH")
            .help("Path to folder with images")
            .required(true)
            .index(1))
        .arg(clap::Arg::with_name("dryrun")
            .short("n")
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
    let img_groups = build_imgs_groups(imgs);
    match reorganize_images(&img_groups, &prefix, &dryrun) {
        Ok(_) => (),
        Err(error) => {
            panic!("Error: {:?}", error)
        },
    }
}
