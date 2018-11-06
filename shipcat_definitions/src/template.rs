use std::collections::HashMap;
use std::iter;

use tera::{self, Value, Tera, Context};
use super::{Result, ErrorKind, ResultExt};

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
fn indent(v: Value, m: HashMap<String, Value>) -> tera::Result<Value> {
    let s : String = try_get_value!("indent", "value", String, v);
    // look up indent value or use `2` as default
    let num_spaces : u64 = m.get("spaces").map(Value::as_u64).unwrap_or(None).unwrap_or(2);
    // create an indent string of `num_spaces` spaces
    let indent = iter::repeat(' ').take(num_spaces as usize).collect::<String>();
    // prefix all non-empty lines with `indent`
    let mut xs = vec![];
    for l in s.lines() {
        xs.push(if l == "" { l.to_string() } else { format!("{}{}", indent, l) });
    }
    Ok(serde_json::to_value(&xs.join("\n")).unwrap())
}

#[cfg_attr(feature = "cargo-clippy", allow(needless_pass_by_value))]
fn as_secret(v: Value, _: HashMap<String, Value>) -> tera::Result<Value> {
    let s = try_get_value!("secret", "value", String, v);
    Ok(format!("SHIPCAT_SECRET::{}", s).into())
}

#[cfg(feature = "filesystem")]
fn read_template_file(svc: &str, tmpl: &str) -> Result<String> {
    use std::fs::File;
    use std::path::Path;
    use std::io::prelude::*;
    // try to read file from ./services/{svc}/{tmpl} into `tpl` sting
    let pth = Path::new(".").join("services").join(svc).join(tmpl);
    let gpth = Path::new(".").join("templates").join(tmpl);
    let found_pth = if pth.exists() {
        debug!("Reading template in {}", pth.display());
        pth
    } else {
        if !gpth.exists() {
            bail!("Template {} does not exist in neither {} nor {}", tmpl, pth.display(), gpth.display());
        }
        debug!("Reading template in {}", gpth.display());
        gpth
    };
    // read the template - should work now
    let mut f = File::open(&found_pth)?;
    let mut data = String::new();
    f.read_to_string(&mut data)?;
    Ok(data)
}

/// Render convenience function that also trims whitespace
///
/// Takes a template to render either in the service folder or the templates folder.
/// The first takes precendense if it exists.
pub fn render_file_data(data: String, context: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("one_off", &data)?;
    tera.autoescape_on(vec!["html"]);
    tera.register_filter("indent", indent);
    tera.register_filter("as_secret", as_secret);

    let result = tera.render("one_off", context)?;
    let mut xs = vec![];
    for l in result.lines() {
        // trim whitespace (mostly to satisfy linters)
        xs.push(l.trim_right());
    }
    Ok(xs.join("\n"))
}

/// One off template
pub fn one_off(tpl: &str, ctx: &Context) -> Result<String> {
    let mut tera = Tera::default();
    tera.add_raw_template("one_off", tpl)?;
    tera.register_filter("as_secret", as_secret);
    let res = tera.render("one_off", ctx)?;
    Ok(res)
}


// main helpers for the manifest
use super::{Manifest};
use super::config::Region;
impl Manifest {
    // This function defines what variables are available within .j2 templates and evars
    fn make_template_context(&self, reg: &Region) -> Result<Context> {
        // same context as normal templates + base_urls
        let mut ctx = Context::new();

        // not great: pass env & secrets in a single btree for backwards compatibility
        // TODO: switch to a bespoke `secrets` struct in manifests
        let mut full_env = self.env.clone();
        full_env.append(&mut self.secrets.clone());

        ctx.insert("env", &full_env);
        ctx.insert("service", &self.name.clone());
        ctx.insert("region", &reg.name.clone());
        ctx.insert("kafka", &self.kafka.clone());
        ctx.insert("base_urls", &reg.base_urls);
        ctx.insert("kong", &reg.kong);
        Ok(ctx)
    }

    /// Read templates from disk and put them into value for ConfigMappedFile
    #[cfg(feature = "filesystem")]
    pub fn read_configs_files(&mut self) -> Result<()> {
        if let Some(ref mut cfg) = self.configs {
            for f in &mut cfg.files {
                f.value = Some(read_template_file(&self.name, &f.name)?);
            }
        }
        Ok(())
    }

    /// Replace template in values with template result inplace
    pub fn template_configs(&mut self, reg: &Region) -> Result<()> {
        let ctx = self.make_template_context(reg)?;
        if let Some(ref mut cfg) = self.configs {
            for f in &mut cfg.files {
                if let Some(ref mut v) = f.value {
                    let data : String = v.clone();
                    let svc = self.name.clone();
                    *v = render_file_data(data, &ctx).chain_err(|| {
                        ErrorKind::InvalidTemplate(svc)
                    })?;
                } else {
                    bail!("configs must be read first - missing {}", f.name); // internal error
                }
            }
        }
        Ok(())
    }

    /// Template evars - must happen before inline templates!
    pub fn template_evars(&mut self, reg: &Region) -> Result<()> {
        let ctx = self.make_template_context(reg)?;
        for (_, v) in &mut self.env {
            *v = one_off(v, &ctx)?;
        }
        Ok(())
    }
}
