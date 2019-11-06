use shipcat_definitions::math::ResourceTotals;
use super::{Config, Region, Manifest, Result};

use rayon::prelude::*;

use generic_array::{typenum::U4, GenericArray};
use size_format::{PointSeparated, PrefixType, SizeFormatter, SizeFormatterBinary};

// Nice size formatting of millicores.
struct Millicores;

// milli is default unit, then stop sub-dividing.
impl PrefixType for Millicores {
    type N = U4;
    const PREFIX_SIZE: u32 = 1000;
    fn prefixes() -> GenericArray<&'static str, Self::N> {
        ["m", "", "k", "M"].into()
    }
}

/// What to sort resources by (for main)
pub enum ResourceOrder {
    Cpu,
    Memory,
}
impl ResourceOrder {
    pub fn from_str(input: &str) -> Result<Self> {
        match input {
            "cpu" => Ok(ResourceOrder::Cpu),
            "memory" => Ok(ResourceOrder::Memory),
            _ => bail!("Resource type must be cpu or memory"),
        }
    }
}

/// Resource top for a single region
///
/// This presents an analytical solution to resource utilisation across the cluster.
/// It does NOT talk to kubernetes.
///
/// It works out ResourceTotals based on Manifest properties analytically.
pub fn resources(order: ResourceOrder, upper_bounds: bool, conf: &Config, reg: &Region)
    -> Result<Vec<(Manifest, ResourceTotals)>>
{
    let available = shipcat_filebacked::available(conf, &reg)?;

    let mfs_res : Result<Vec<(Manifest, ResourceTotals)>> = available.par_iter()
        .map(|mf| {
            let mf = shipcat_filebacked::load_manifest(&mf.base.name, &conf, &reg)?
                .stub(&reg)?;
            let res = mf.compute_resource_totals()?;
            Ok((mf, res))
        })
        .collect();
    let mut mfs = mfs_res?;
    match order {
        ResourceOrder::Cpu => {
            mfs.sort_by(|(_, r1), (_, r2)| {
                if upper_bounds {
                    (r2.base.requests.cpu + r2.extra.requests.cpu)
                        .partial_cmp(&(r1.base.requests.cpu + r1.extra.requests.cpu)).unwrap()
                } else {
                    r2.base.requests.cpu.partial_cmp(&r1.base.requests.cpu).unwrap()
                }
            });
        },
        ResourceOrder::Memory => {
            mfs.sort_by(|(_, r1), (_, r2)| {
                if upper_bounds {
                    (r2.base.requests.memory + r2.extra.requests.memory)
                        .partial_cmp(&(r1.base.requests.memory + r1.extra.requests.memory)).unwrap()
                } else {
                    r2.base.requests.memory.partial_cmp(&r1.base.requests.memory).unwrap()
                }
            });
        }
    }
    println!("{0:<50} {1:<8} {2:<8} {3:40}", "SERVICE", "CPU", "MEMORY", "TEAM");
    mfs.iter().for_each(|(mf, r)| {
        debug!("{}: cpu {} + {}, mem: {} + {}", mf.name, r.base.requests.cpu,
            r.extra.requests.cpu, r.base.requests.memory, r.extra.requests.memory);
        if upper_bounds {
            let ub_cpu = (1000.0*(r.base.requests.cpu + r.extra.requests.cpu)) as u64;
            let ub_memory = (r.base.requests.memory + r.extra.requests.memory) as u64;
            println!("{0:<50} {1:width$} {2:width$} {3:<40}", mf.name,
                format!("{:.0}", SizeFormatter::<u64, Millicores, PointSeparated>::new(ub_cpu)),
                format!("{:.0}B", SizeFormatterBinary::new(ub_memory)),
                mf.metadata.as_ref().unwrap().team,
                width = 8,
            );
        } else {
            let lb_cpu = (1000.0*r.base.requests.cpu) as u64;
            let lb_memory = r.base.requests.memory as u64;
            println!("{0:<50} {1:width$} {2:width$} {3:<40}", mf.name,
                format!("{:.0}", SizeFormatter::<u64, Millicores, PointSeparated>::new(lb_cpu)),
                format!("{:.0}B", SizeFormatterBinary::new(lb_memory)),
                mf.metadata.as_ref().unwrap().team,
                width = 8,
            );
        }
    });
    Ok(mfs)
}
