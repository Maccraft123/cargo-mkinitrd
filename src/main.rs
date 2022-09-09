use std::process::Command;
use std::io::Result;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Stdio;
use std::io::Write;
use std::io;
use std::fs::File;

fn mkcpio(input: &PathBuf) -> Result<()> {
    let out = File::create(format!("./target/initrd/{}.cpio", input.file_name().unwrap().to_string_lossy()))?;
    let mut cpio = Command::new("cpio")
        .arg("-D./target/initrd")
        .arg("--no-absolute-filenames")
        .arg("-ov") // forgot what it does
        .arg("--format=newc")
        .stdin(Stdio::piped())
        .stdout(out)
        .stderr(Stdio::piped())
        .spawn()?;

    if PathBuf::from("./target/initrd/init").exists() {
        fs::remove_file("./target/initrd/init")?;
    }
    fs::copy(input, "./target/initrd/init")?;

    let cpio_stdin = cpio.stdin.as_mut().unwrap();
    cpio_stdin.write_all(b"init\n")?;
    drop(cpio_stdin);

    let out = cpio.wait_with_output()?;
    if !out.status.success() {
        io::stderr().write_all(&out.stderr)?;
    }

    Ok(())
}

fn main() -> Result<()> {
    Command::new("cargo")
        .arg("build")
        .arg("--release")
        .args(vec!["--target", "x86_64-unknown-linux-musl"])
        .args(vec!["--config", "profile.release.opt-level='z'"])
        .args(vec!["--config", "profile.release.strip=true"])
        .args(vec!["--config", "profile.release.lto=true"])
        .args(vec!["--config", "profile.release.codegen-units=1"])
        .status()
        .map(|_| ())?;

    let target_dir = PathBuf::from("./target/x86_64-unknown-linux-musl/release");
    let mut executables = Vec::new();

    for maybe_file in fs::read_dir(target_dir)? {
        if let Ok(file) = maybe_file {
            // check if executable
            if let Ok(metadata) = fs::metadata(file.path()) {
                let perms = metadata.permissions();
                // executable dirs exist
                if metadata.is_file() && perms.mode() & 0o111 != 0 {
                    executables.push(file.path());
                }
            }
        }
    };

    if !PathBuf::from("./target/initrd/").exists() {
        fs::create_dir("./target/initrd/")?;
    }

    for exec in executables {
        mkcpio(&exec)?;

        let name = exec.file_name().unwrap().to_string_lossy();
        println!("Built {} to {}", name, format!("./target/initrd/{}.cpio", name));
    }


    Ok(())
}
