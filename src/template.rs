use std::collections::HashMap;
use tera::{Value, Result, Tera};
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

pub fn init() -> Tera {
    let mut tera = compile_templates!("dev/*");
    tera.autoescape_on(vec!["html"]);
    tera.register_filter("indent4", indent4);
    tera
}
