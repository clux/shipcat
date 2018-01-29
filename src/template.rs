use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io::prelude::*;

use walkdir::WalkDir;

use tera::{self, Value, Tera, Context};
use serde_json;
use super::Result;

fn indent4(v: Value, _: HashMap<String, Value>) -> tera::Result<Value> {
    let s : String = try_get_value!("indent", "value", String, v);
    let mut xs = vec![];
    for l in s.lines() {
        // indent all non-empty lines by 4 spaces
        xs.push(if l == "" { l.to_string() } else { format!("    {}", l) });
    }
    Ok(serde_json::to_value(&xs.join("\n")).unwrap())
}

fn add_templates(tera: &mut Tera, dir: &PathBuf, svc: &str, depth: usize) -> Result<()> {
    let sdirs = WalkDir::new(&dir)
        .min_depth(depth)
        .max_depth(depth)
        .into_iter()
        .filter_map(|e| e.ok())
        // files only
        .filter(|e| e.file_type().is_file())
        // only add files that end in .j2
        .filter(|e| {
            e.file_name().to_string_lossy().ends_with(".j2")
        })
        // if subdirectoried files, only from the directory of the relevant service
        .filter(|e| {
            let mut cmps = e.path().components();
            cmps.next(); // .
            cmps.next(); // services or templates
            // this next bit is only relevant if maxdepth is 2 and we don't want directories
            let last_comp = cmps.next().unwrap(); // folder name or file name!
            let dirname = last_comp.as_os_str().to_str().unwrap();
            let dirpth = dir.join(dirname);
            (!dirpth.is_dir() || dirname == svc)
        });

    // add all templates to the templating engine
    for entry in sdirs {
        let tpth = entry.path();
        debug!("Reading {}", entry.path().display());

        // read it
        let mut f = File::open(&tpth)?;
        let mut data = String::new();
        f.read_to_string(&mut data)?;

        // store in template engine internal hashmap under a easy name
        let fname = tpth.file_name().unwrap().to_string_lossy();
        debug!("Storing {}", fname);
        tera.add_raw_template(&fname, &data)?;
    }
    Ok(())
}



pub fn init(service: &str) -> Result<Tera> {
    let mut tera = Tera::default();
    tera.autoescape_on(vec!["html"]);
    tera.register_filter("indent4", indent4);

    let services_root = Path::new("."); // TODO: cathulk repo root evar
    // adding templates from template subfolder first
    let tdir = Path::new(&services_root).join("templates");
    add_templates(&mut tera, &tdir, service, 1)?;
    // then templates from service subfolder (as they override)
    let sdir = Path::new(&services_root).join("services");
    add_templates(&mut tera, &sdir, service, 2)?;

    Ok(tera)
}

pub fn render(tera: &Tera, tmpl: &str, context: &Context) -> Result<String> {
    let result = tera.render(tmpl, context)?;
    let mut xs = vec![];
    for l in result.lines() {
        // trim whitespace (mostly to satisfy linters)
        xs.push(l.trim_right());
    }
    Ok(xs.join("\n"))
}
