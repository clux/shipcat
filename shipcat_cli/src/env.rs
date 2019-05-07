use super::{Result, Config, Region};

/// Print exports to source from a shell
pub fn print_bash(svc: &str, conf: &Config, reg: &Region) -> Result<()> {
    let mf = shipcat_filebacked::load_manifest(svc, conf, reg)?.complete(&reg)?;
    for (k,s) in mf.secrets {
        println!("export {}={}", k, s);
    }
    for (k,v) in mf.env.plain {
        println!("export {}={}", k,v);
    }
    Ok(())
}
