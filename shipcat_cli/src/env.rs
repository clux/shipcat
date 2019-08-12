use super::{Result, Config, Region};

/// Print exports to source from a shell
pub fn print_bash(svc: &str, conf: &Config, reg: &Region, mock: bool) -> Result<()> {
    let mf = if mock {
        warn!("Using mocked values for secrets. Use `-s` to resolve secrets.");
        shipcat_filebacked::load_manifest(&svc, &conf, &reg)?.stub(&reg)?
    } else {
        shipcat_filebacked::load_manifest(&svc, &conf, &reg)?.complete(&reg)?
    };

    for (k,s) in mf.secrets {
        println!("export {}={}", k, s);
    }
    for (k,v) in mf.env.plain {
        println!("export {}={}", k,v);
    }
    Ok(())
}
