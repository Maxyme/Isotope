#![forbid(unsafe_code)]
mod artifacts;
mod atomic;
mod build;
mod common;
mod configure;
mod consumer;
mod downloads;
mod fonts;
mod frontend;
mod graphics;
mod process;
mod receipts;
mod release;
static REQUIRE_X86: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static NUCLEON: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use std::{num::NonZeroUsize, path::PathBuf, sync::atomic::Ordering};

#[derive(Parser)]
#[command(
    about = "Isotope: native ARM64 Wine/FEX builds, probes and source-inclusive packaging",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    /// Consumer executable used for package/assembly integration validation
    #[arg(long, global = true)]
    nucleon: Option<PathBuf>,
}
#[derive(Subcommand)]
enum Commands {
    /// Build or probe the pinned native Wine/FEX runtime
    Fex(FexArgs),
    /// Build/package the core runtime; x86 failures are temporarily optional
    Release(ReleaseArgs),
    /// Build, assemble or probe the isolated Vulkan/Metal integration
    Dx12(GraphicsArgs),
    /// Build and validate separately paired experimental graphics frontends
    Frontend(FrontendArgs),
}
#[derive(Args)]
struct Workspace {
    #[arg(long, default_value = "target/arm64-fex")]
    work: PathBuf,
    #[arg(long, default_value = "6")]
    jobs: NonZeroUsize,
}
#[derive(Clone, Copy, ValueEnum)]
enum FexAction {
    Doctor,
    Fetch,
    BuildFex,
    BuildWine,
    Probe,
    ProbeDxmt,
    CheckReference,
    ProbeUnixlib,
}
#[derive(Args)]
struct FexArgs {
    #[arg(value_enum)]
    action: FexAction,
    #[command(flatten)]
    workspace: Workspace,
    #[arg(long)]
    reference: bool,
    #[arg(long)]
    window: bool,
    #[arg(long)]
    library: Option<PathBuf>,
}
#[derive(Clone, Copy, ValueEnum)]
enum ReleaseAction {
    Build,
    Package,
    /// Relocate and execute the current built core, without creating release assets
    Validate,
}
#[derive(Args)]
struct ReleaseArgs {
    #[arg(value_enum)]
    action: ReleaseAction,
    #[command(flatten)]
    workspace: Workspace,
    #[arg(long)]
    version: String,
    #[arg(long)]
    output: PathBuf,
    /// Make a failed 32-bit test block publication (the test always runs)
    #[arg(long)]
    require_x86: bool,
}
#[derive(Args)]
struct GraphicsArgs {
    #[arg(value_enum)]
    action: GraphicsAction,
    #[command(flatten)]
    workspace: Workspace,
    #[arg(long)]
    sdk: Option<PathBuf>,
    #[arg(long)]
    debug: bool,
}
#[derive(Clone, Copy, ValueEnum)]
enum GraphicsAction {
    Fetch,
    BuildWine,
    BuildDriver,
    StageDxmt,
    PrepareRuntime,
    ProbeDriver,
    ProbeVulkan,
    ProbeD3d12,
}
#[derive(Args)]
struct FrontendArgs {
    #[arg(value_enum)]
    action: FrontendAction,
    #[command(flatten)]
    workspace: Workspace,
    #[arg(long)]
    sdk: Option<PathBuf>,
    #[arg(long)]
    debug: bool,
    #[arg(long,default_value="vkd3d",value_parser=["dxvk","vkd3d","dxmt"])]
    component: String,
    #[arg(long,default_value="arm64ec",value_parser=["arm64ec","x64"])]
    architecture: String,
    #[arg(long,default_value="dx12",value_parser=["dx11","dx11-vulkan","dx12"])]
    api: String,
    #[arg(long)]
    native_llvm: Option<PathBuf>,
    #[arg(long)]
    base_runtime: Option<PathBuf>,
}
#[derive(Clone, Copy, ValueEnum)]
enum FrontendAction {
    Fetch,
    Doctor,
    Build,
    PrepareRuntime,
    Probe,
    ProbeTls,
}
fn graphics_context(
    workspace: &Workspace,
    sdk: Option<&PathBuf>,
    debug: bool,
) -> Result<graphics::Graphics> {
    let sdk = sdk.cloned().unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .expect("HOME is required")
            .join("VulkanSDK/1.4.363.0/macOS")
    });
    graphics::Graphics::new(&workspace.work, &sdk, workspace.jobs.get(), debug)
}
fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();
    let _ = NUCLEON.set(
        cli.nucleon
            .or_else(|| std::env::var_os("ISOTOPE_NUCLEON").map(PathBuf::from))
            .or_else(|| std::env::var_os("WINE_FEX_NUCLEON").map(PathBuf::from))
            .unwrap_or_else(|| "nucleon".into()),
    );
    ctrlc::set_handler(|| common::CANCELLED.store(true, Ordering::SeqCst))?;
    match cli.command {
        Commands::Fex(args) => {
            let build = build::Build::new(&args.workspace.work, args.workspace.jobs.get())?;
            let _lease =
                common::FileLock::acquire(&build.workspace.join(".nucleon-devtools.lock"))?;
            match args.action {
                FexAction::Doctor => {
                    build.doctor()?;
                    Ok(())
                }
                FexAction::Fetch => build.fetch(args.reference),
                FexAction::BuildFex => {
                    common::require_native()?;
                    build.build_fex()
                }
                FexAction::BuildWine => {
                    common::require_native()?;
                    build.build_wine(&[], None)
                }
                FexAction::Probe => {
                    common::require_native()?;
                    build.probe(args.window)
                }
                FexAction::ProbeDxmt => {
                    common::require_native()?;
                    let graphics = graphics_context(&args.workspace, None, false)?;
                    graphics.stage_dxmt()?;
                    graphics.probe("dxmt")
                }
                FexAction::CheckReference => build.check_reference(),
                FexAction::ProbeUnixlib => {
                    common::require_native()?;
                    build.probe_unixlib(args.library.as_deref())
                }
            }
        }
        Commands::Release(args) => {
            REQUIRE_X86.store(args.require_x86, Ordering::SeqCst);
            common::require_native()?;
            let build = build::Build::new(&args.workspace.work, args.workspace.jobs.get())?;
            let _lease =
                common::FileLock::acquire(&build.workspace.join(".nucleon-devtools.lock"))?;
            let output = std::path::absolute(args.output)?;
            match args.action {
                ReleaseAction::Build => release::build_release(&build, &args.version, &output),
                ReleaseAction::Package => release::package(&build, &args.version, &output),
                ReleaseAction::Validate => release::validate_built(&build, &output),
            }
        }
        Commands::Dx12(args) => {
            let graphics = graphics_context(&args.workspace, args.sdk.as_ref(), args.debug)?;
            let _lease =
                common::FileLock::acquire(&graphics.base.workspace.join(".nucleon-devtools.lock"))?;
            match args.action {
                GraphicsAction::Fetch => graphics.fetch(),
                GraphicsAction::BuildWine => {
                    common::require_native()?;
                    graphics.build_wine()
                }
                GraphicsAction::BuildDriver => {
                    common::require_native()?;
                    graphics.build_driver()
                }
                GraphicsAction::StageDxmt => {
                    common::require_native()?;
                    println!("{}", graphics.stage_dxmt()?.display());
                    Ok(())
                }
                GraphicsAction::PrepareRuntime => {
                    println!("{}", graphics.prepare_runtime()?.display());
                    Ok(())
                }
                GraphicsAction::ProbeDriver => {
                    common::require_native()?;
                    graphics.probe_driver()
                }
                GraphicsAction::ProbeVulkan => {
                    common::require_native()?;
                    graphics.probe("vulkan")
                }
                GraphicsAction::ProbeD3d12 => {
                    common::require_native()?;
                    graphics.probe("d3d12")
                }
            }
        }
        Commands::Frontend(args) => {
            let frontend = frontend::Frontend::new(
                graphics_context(&args.workspace, args.sdk.as_ref(), args.debug)?,
                args.native_llvm,
            )?;
            let _lease = common::FileLock::acquire(
                &frontend
                    .graphics
                    .base
                    .workspace
                    .join(".nucleon-devtools.lock"),
            )?;
            let _legacy_lease = common::FileLock::configuration(&frontend.work.join("experiment"))?;
            match args.action {
                FrontendAction::Fetch => frontend.fetch(),
                FrontendAction::Doctor => {
                    common::require_native()?;
                    println!("{}", frontend.doctor()?);
                    Ok(())
                }
                FrontendAction::Build => {
                    common::require_native()?;
                    frontend.build(&args.component, &args.architecture)
                }
                FrontendAction::PrepareRuntime => {
                    let base = match args.base_runtime {
                        Some(path) => path,
                        None => frontend.graphics.prepare_runtime()?,
                    };
                    println!("{}", frontend.assemble(&base)?.display());
                    Ok(())
                }
                FrontendAction::Probe => {
                    common::require_native()?;
                    frontend.probe(&args.architecture, &args.api)
                }
                FrontendAction::ProbeTls => {
                    common::require_native()?;
                    frontend.probe_tls()
                }
            }
        }
    }
}
