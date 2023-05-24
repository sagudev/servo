use std::{
    collections::BTreeSet,
    env::{current_dir, set_current_dir},
    num::NonZeroUsize,
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::Parser;
use lets_find_up::{find_up_with, FindUpKind, FindUpOptions};
use miette::{ensure, miette, Context, Diagnostic, IntoDiagnostic, Report, SourceSpan};
use regex::Regex;

use crate::{
    fs::{copy_dir, create_dir_all, existing_file, remove_file, FileRoot},
    path::join_path,
    process::{which, EasyCommand},
};

mod fs;
mod path;
mod process;

/// Vendor WebGPU CTS tests
///
/// WPT tests are generated into `tests/wpt/webgpu/tests/webgpu/`. If the set of tests
/// changes upstream, make sure that the generated output still matches up with test expectation
/// metadata in `tests/wpt/webgpu/meta/webgpu/`.
#[derive(Debug, Parser)]
struct CliArgs {
    /// Repo of cts
    #[clap(default_value = "https://github.com/gpuweb/cts")]
    cts_repo: String,
    /// Checkout branch or commit from cts repo
    #[clap(default_value = "main")]
    cts_checkout: String,
}

fn main() -> ExitCode {
    env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .parse_default_env()
        .init();

    let args = CliArgs::parse();

    match run(args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            log::error!("{e:?}");
            ExitCode::FAILURE
        },
    }
}

fn run(args: CliArgs) -> miette::Result<()> {
    let CliArgs {
        cts_repo,
        cts_checkout,
    } = args;
    //cts_checkout_path

    // It is expected to run this tool from the root of its Cargo project
    let orig_working_dir = current_dir().unwrap();

    let gecko_ckt = FileRoot::new(
        "servo",
        orig_working_dir.parent().unwrap().parent().unwrap(),
    )
    .unwrap();

    let cts_vendor_dir = gecko_ckt.child(join_path(["support", "vendor-webgpu-cts"]));

    let wpt_tests_dir = {
        let child = gecko_ckt.child(join_path(["tests", "wpt", "webgpu", "tests"]));
        ensure!(
            child.is_dir(),
            "WPT tests dir ({}) does not appear to exist",
            child,
        );
        child
    };

    let cts_ckt = gecko_ckt.child(join_path(["target", "cts-checkout"]));
    let cts_ckt_git_dir = cts_ckt.child(join_path([".git"]));

    /*
    git clone -n <repo_name>
    git checkout <commit_sha>
     */

    let git_bin = which("git", "Git binary")?;
    let npm_bin = which("npm", "NPM binary")?;

    log::info!("making a clone…",);

    if !(cts_ckt_git_dir.exists()) {
        // clone
        let mut git_clone = EasyCommand::new(&git_bin, |cmd| {
            cmd.args([
                "clone",
                "-n",
                cts_repo.as_str(),
                "../../target/cts-checkout",
            ])
        });
        log::info!(
            "  …clone with \
                {git_clone}…"
        );
        git_clone.spawn()?;
    }
    // chkout
    let mut git_chk = EasyCommand::new(&git_bin, |cmd| {
        cmd.args(["checkout", cts_checkout.as_str()])
        .envs([("GIT_DIR", &*cts_ckt_git_dir), ("GIT_WORK_TREE", &*cts_ckt)])
    });
    log::info!(
        "  …checkout with \
                {git_chk}…"
    );
    git_chk.spawn()?;

    log::info!("making a vendored copy of checked-in files from {cts_ckt}…",);
    gecko_ckt.regen_file(
        join_path([&*wpt_tests_dir, "checkout_commit.txt".as_ref()]),
        |checkout_commit_file| {
            let mut git_status_porcelain_cmd = EasyCommand::new(&git_bin, |cmd| {
                cmd.args(["status", "--porcelain"])
                    .envs([("GIT_DIR", &*cts_ckt_git_dir), ("GIT_WORK_TREE", &*cts_ckt)])
            });
            log::info!(
                "  …ensuring the working tree and index are clean with \
                    {git_status_porcelain_cmd}…"
            );
            let git_status_porcelain_output = git_status_porcelain_cmd.just_stdout_utf8()?;
            ensure!(
                git_status_porcelain_output.is_empty(),
                "expected a clean CTS working tree and index, but {}'s output was not empty; \
                    for reference, it was:\n\n{}",
                git_status_porcelain_cmd,
                git_status_porcelain_output,
            );

            /*gecko_ckt.regen_dir(&cts_ckt, |vendored_ckt_dir| {
                log::info!("  …copying files tracked by Git to {vendored_ckt_dir}…");
                let files_to_vendor = {
                    let mut git_ls_files_cmd = EasyCommand::new(&git_bin, |cmd| {
                        cmd.arg("ls-files").env("GIT_DIR", &cts_ckt_git_dir)
                    });
                    log::debug!("  …getting files to vendor from {git_ls_files_cmd}…");
                    let output = git_ls_files_cmd.just_stdout_utf8()?;
                    let mut files = output
                        .split_terminator('\n')
                        .map(PathBuf::from)
                        .collect::<BTreeSet<_>>();
                    log::trace!("  …files from {git_ls_files_cmd}: {files:#?}");

                    log::trace!("  …validating that files from Git repo still exist…");
                    let files_not_found = files
                        .iter()
                        .filter(|p| !cts_ckt.child(p).exists())
                        .collect::<Vec<_>>();
                    ensure!(
                        files_not_found.is_empty(),
                        "the following files were returned by `git ls-files`, but do not \
                        exist on disk: {:#?}",
                        files_not_found,
                    );

                    log::trace!("  …stripping files we actually don't want to vendor…");
                    let files_to_actually_not_vendor = [
                        // There's no reason to bring this over, and lots of reasons to not bring in
                        // security-sensitive content unless we have to.
                        "deploy_key.enc",
                    ]
                    .map(Path::new);
                    log::trace!("    …files we don't want: {files_to_actually_not_vendor:?}");
                    for path in files_to_actually_not_vendor {
                        ensure!(
                            files.remove(path),
                            "failed to remove {} from list of files to vendor; does it still \
                                exist?",
                            cts_ckt.child(path)
                        );
                    }
                    files
                };

                log::debug!("  …now doing the copying…");
                for path in files_to_vendor {
                    let vendor_from_path = cts_ckt.child(&path);
                    let vendor_to_path = vendored_ckt_dir.child(&path);
                    if let Some(parent) = vendor_to_path.parent() {
                        create_dir_all(vendored_ckt_dir.child(parent))?;
                    }
                    log::trace!("    …copying {vendor_from_path} to {vendor_to_path}…");
                    fs::copy(&vendor_from_path, &vendor_to_path)?;
                }

                Ok(())
            })?;*/

            log::info!("  …writing commit ref pointed to by `HEAD` to {checkout_commit_file}…");
            let mut git_rev_parse_head_cmd = EasyCommand::new(&git_bin, |cmd| {
                cmd.args(["rev-parse", "HEAD"])
                    .env("GIT_DIR", &cts_ckt_git_dir)
            });
            log::trace!("    …getting output of {git_rev_parse_head_cmd}…");
            fs::write(
                checkout_commit_file,
                git_rev_parse_head_cmd.just_stdout_utf8()?,
            )
            .wrap_err_with(|| format!("failed to write HEAD ref to {checkout_commit_file}"))
        },
    )?;

    set_current_dir(&*cts_ckt)
        .into_diagnostic()
        .wrap_err("failed to change working directory to CTS checkout")?;
    log::debug!("changed CWD to {cts_ckt}");

    let mut npm_ci_cmd = EasyCommand::new(&npm_bin, |cmd| cmd.arg("ci"));
    log::info!(
        "ensuring a clean {} directory with {npm_ci_cmd}…",
        cts_ckt.child("node_modules"),
    );
    npm_ci_cmd.spawn()?;

    /*let out_dir = cts_ckt.regen_dir("out", |out_dir| {
        let mut npm_run_standalone_cmd =
            EasyCommand::new(&npm_bin, |cmd| cmd.args(["run", "standalone"]));
        log::info!(
            "generating standalone runner files into {out_dir} with {npm_run_standalone_cmd}…"
        );
        npm_run_standalone_cmd.spawn()
    })?;*/

    let out_wpt_dir = cts_ckt.regen_dir("out-wpt", |out_wpt_dir| {
        let mut npm_run_wpt_cmd = EasyCommand::new(&npm_bin, |cmd| cmd.args(["run", "wpt"]));
        log::info!("generating WPT test cases into {out_wpt_dir} with {npm_run_wpt_cmd}…");
        npm_run_wpt_cmd.spawn()
    })?;

    let cts_https_html_path = out_wpt_dir.child("cts.https.html");
    log::info!("refining the output of {cts_https_html_path} with `npm run gen_wpt_cts_html …`…");
    EasyCommand::new(&npm_bin, |cmd| {
        cmd.args(["run", "gen_wpt_cts_html"])
            .arg(existing_file(&cts_https_html_path))
            .args([
                existing_file(cts_ckt.child(join_path([
                    "src",
                    "common",
                    "templates",
                    "cts.https.html",
                ]))),
                existing_file(cts_vendor_dir.child("arguments.txt")),
                existing_file(cts_vendor_dir.child("myexpectations.txt")),
            ])
            .arg("")
    })
    .spawn()?;

    /*log::info!("stealing standalone runtime files from {out_dir} for {out_wpt_dir}…");
    for subdir in [
        &["external"] as &[_],
        &["common", "internal"],
        &["common", "util"],
    ]
    .map(join_path)
    {
        let out_subdir = out_dir.child(&subdir);
        let out_wpt_subdir = out_wpt_dir.child(subdir);
        log::info!("  …copying from {out_subdir} to {out_wpt_subdir}…");
        copy_dir(out_subdir, out_wpt_subdir)?
    }
    log::info!("  …done stealing!");*/

    log::info!("analyzing {cts_https_html_path}…");
    let cts_https_html_content = fs::read_to_string(&*cts_https_html_path)?;
    let cts_boilerplate;
    let cts_cases;
    {
        {
            let (boilerplate, cases_start) = {
                let cases_start_idx = cts_https_html_content
                    .find("<meta name=variant")
                    .ok_or_else(|| miette!("no test cases found; this is unexpected!"))?;
                cts_https_html_content.split_at(cases_start_idx)
            };

            cts_boilerplate = {
                if !boilerplate.is_empty() {
                    #[derive(Debug, Diagnostic, thiserror::Error)]
                    #[error("last character before test cases was not a newline; bug, or weird?")]
                    #[diagnostic(severity("warning"))]
                    struct Oops {
                        #[label(
                            "this character ({:?}) was expected to be a newline, so that the test \
                            spec. following it is on its own line",
                            source_code.chars().last().unwrap()
                        )]
                        span: SourceSpan,
                        #[source_code]
                        source_code: String,
                    }
                    ensure!(
                        boilerplate.ends_with('\n'),
                        Oops {
                            span: SourceSpan::from(0..boilerplate.len()),
                            source_code: cts_https_html_content,
                        }
                    );
                }
                // NOTE: Adding `_mozilla` is necessary because [that's how it's mounted][source].
                //
                // [source]: https://searchfox.org/mozilla-central/rev/cd2121e7d83af1b421c95e8c923db70e692dab5f/testing/web-platform/mozilla/README#1-4]
                log::info!(
                    "  …fixing `script` paths in WPT boilerplate so they work as Mozilla-private \
                    WPT tests…"
                );
                let expected_wpt_script_tag =
                    "<script type=module src=/webgpu/common/runtime/wpt.js></script>";
                ensure!(
                    boilerplate.contains(expected_wpt_script_tag),
                    "failed to find expected `script` tag for `wpt.js` \
                    ({:?}); did something change upstream?",
                    expected_wpt_script_tag
                );
                boilerplate.replacen(
                    expected_wpt_script_tag,
                    "<script type=module src=../webgpu/common/runtime/wpt.js></script>",
                    1,
                )
            };

            log::info!("  …parsing test variants in {cts_https_html_path}…");
            cts_cases = cases_start.split_terminator('\n').collect::<Vec<_>>();
            let mut parsing_failed = false;
            let meta_variant_regex =
                Regex::new("^<meta name=variant content='([^']*?)'>$").unwrap();
            cts_cases.iter().for_each(|line| {
                if !meta_variant_regex.is_match(line) {
                    parsing_failed = true;
                    log::error!("line is not a test case: {line:?}");
                }
            });
            ensure!(
                !parsing_failed,
                "one or more test case lines failed to parse, fix it and try again"
            );
        };
        log::trace!("\"original\" HTML boilerplate:\n\n{}", cts_boilerplate);

        ensure!(
            !cts_cases.is_empty(),
            "no test cases found; this is unexpected!"
        );
        log::info!("  …found {} test cases", cts_cases.len());
    }

    gecko_ckt.regen_dir(wpt_tests_dir.join("webgpu"), |wpt_webgpu_tests_dir| {
        log::info!("copying contents of {out_wpt_dir} to {wpt_webgpu_tests_dir}…");
        copy_dir(&out_wpt_dir, wpt_webgpu_tests_dir)
    })?;

    log::info!("All done! Now get your CTS _ON_! :)");

    Ok(())
}
