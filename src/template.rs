use std::collections::HashMap;
use std::fs::File;
use std::path::Path;
use std::io::prelude::*;

use walkdir::WalkDir;

use tera::{Value, Result, Tera, Context};
use serde_json;

fn indent4(v: Value, _: HashMap<String, Value>) -> Result<Value> {
    let s : String = try_get_value!("indent", "value", String, v);
    let mut xs = vec![];
    for l in s.lines() {
        // indent all non-empty lines by 4 spaces
        xs.push(if l == "" { l.to_string() } else { format!("    {}", l) });
    }
    Ok(serde_json::to_value(&xs.join("\n")).unwrap())
}

pub fn init(env: &str, service: &str) -> super::Result<Tera> {
    let mut tera = Tera::default();
    tera.autoescape_on(vec!["html"]);
    tera.register_filter("indent4", indent4);

    // TODO: rather than using PWD we could have a CATHULK_ROOT evar here
    // TODO: add service subfolder first, THEN env subfolder so we have specificity!
    let cathulk_root = Path::new(".");
    let edir = Path::new(&cathulk_root).join(env);
    let edirs = WalkDir::new(&edir)
        .min_depth(1)
        .max_depth(2)
        .into_iter()
        .filter_map(|e| e.ok())
        // files only
        .filter(|e| e.file_type().is_file())
        // skip the shipcat files (never templated)
        .filter(|e| {
            e.file_name().to_string_lossy() != "shipcat.yml"
        })
        // skip hidden files
        .filter(|e| {
            !e.file_name().to_string_lossy().starts_with('.')
        })
        // if subdirectoried files, only from the directory of the relevant service
        .filter(|e| {
            let mut cmps = e.path().components();
            cmps.next(); // .
            cmps.next(); // envname
            let last_comp = cmps.next().unwrap(); // folder name or file name!
            let dirname = last_comp.as_os_str().to_str().unwrap();
            let dirpth = edir.join(dirname);
            (!dirpth.is_dir() || dirname == service)
        });

    // add all templates to the templating engine
    for entry in edirs {
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

    Ok(tera)
}

pub fn render(tera: &Tera, tmpl: &str, context: &Context) -> super::Result<String> {
    let result = tera.render(tmpl, context)?;
    let mut xs = vec![];
    for l in result.lines() {
        // trim whitespace (mostly to satisfy linters)
        xs.push(l.trim_right());
    }
    Ok(xs.join("\n"))
}
