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

/// Resource top for a every
///
/// This presents an analytical solution to aggregate resource requests.
/// It does NOT talk to kubernetes.
///
/// It works out ResourceTotals based on Manifest properties analytically.
pub fn world_requests(order: ResourceOrder, upper_bounds: bool, conf: &Config)
    -> Result<Vec<(Manifest, ResourceTotals)>>
{
    let all = shipcat_filebacked::all(conf)?;
    let mfs_res : Result<Vec<Option<(Manifest, ResourceTotals)>>> = all.par_iter()
        .map(|base| {
            let mut res = ResourceTotals::default();
            let mut first_mf = None;
            debug!("{} looping over {:?}", base.name, base.regions);
            for r in &base.regions {
                if let Some(reg) = conf.get_region_unchecked(&r) {
                    trace!("valid region: {}", reg.name);
                    let mf = shipcat_filebacked::load_manifest(&base.name, &conf, &reg)?
                        .stub(&reg)?;
                    if !mf.disabled && !mf.external {
                        let ResourceTotals { base: rb, extra: se } = mf.compute_resource_totals()?;
                        debug!("{} in {}: adding reqs: {} {}", mf.name, r, rb.requests.cpu, rb.requests.memory);
                        res.base += rb.clone();
                        res.extra += se.clone();
                        first_mf = Some(mf);
                    }
                }
            }
            if let Some(mf) = first_mf {
                Ok(Some((mf, res)))
            } else {
                Ok(None)
            }
        })
        .collect();
    let mfs : Vec<(Manifest, ResourceTotals)> = mfs_res?.into_iter().filter_map(|x| x).collect();
    let mfs = sort_and_print_resources(mfs, order, upper_bounds)?;
    Ok(mfs)
}


/// Resource top for a single region
///
/// This presents an analytical solution to aggregate resource requests in a region.
/// It does NOT talk to kubernetes.
///
/// It works out ResourceTotals based on Manifest properties analytically.
pub fn region_requests(order: ResourceOrder, upper_bounds: bool, conf: &Config, reg: &Region)
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
    let mfs = sort_and_print_resources(mfs_res?, order, upper_bounds)?;
    Ok(mfs)
}


fn sort_and_print_resources(mfs_: Vec<(Manifest, ResourceTotals)>, order: ResourceOrder, upper_bounds: bool)
    -> Result<Vec<(Manifest, ResourceTotals)>>
{
    let mut mfs = mfs_;
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
