//! Snapshot selado → prlimit → bubblewrap → executável estático, sem mounts do host.
use crate::cake::{Result, fail};
use logos_core::{Budget, CheckOutcome, Exhausted};
use rustix::fs::{MemfdFlags, SealFlags, fcntl_add_seals, memfd_create};
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn memfile(bytes: &[u8], sealed: bool) -> Result<File> {
    let mut f = File::from(memfd_create(c"logos-cake", MemfdFlags::ALLOW_SEALING)?);
    f.write_all(bytes)?;
    f.rewind()?;
    if sealed {
        fcntl_add_seals(
            &f,
            SealFlags::WRITE | SealFlags::GROW | SealFlags::SHRINK | SealFlags::SEAL,
        )?;
    }
    Ok(f)
}

// RAII para erros/retornos antecipados; o caminho normal propaga kill/wait.
struct Running(Child);
impl Drop for Running {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

pub(crate) fn run(executable: &[u8], cnf: &[u8], proof: &[u8], b: &Budget) -> Result<CheckOutcome> {
    // ro-bind-data copia as entradas durante o setup; FSIZE precisa admitir a maior.
    let file_limit = executable.len().max(cnf.len()).max(proof.len()).max(65_536);
    let executable = memfile(executable, true)?;
    let cnf = memfile(cnf, true)?;
    let proof = memfile(proof, true)?;
    let mut stdout = memfile(&[], false)?;
    let mut stderr = memfile(&[], false)?;
    let cpu = b.wall_ms.div_ceil(1000).max(1);
    let mb = (b.mem_bytes / (3 * 1024 * 1024)).max(1);
    let mut command = Command::new("/usr/bin/prlimit");
    command
        .env_clear()
        .args([
            format!("--as={0}:{0}", b.mem_bytes),
            format!("--cpu={cpu}:{}", cpu + 1),
            format!("--fsize={file_limit}:{file_limit}"),
            "--core=0:0".into(),
        ])
        .args([
            "--",
            "/usr/bin/bwrap",
            "--unshare-user",
            "--unshare-pid",
            "--unshare-net",
            "--unshare-ipc",
            "--unshare-uts",
            "--new-session",
            "--die-with-parent",
            "--clearenv",
            "--cap-drop",
            "ALL",
        ]);
    for (file, path, mode) in [
        (&executable, "/checker", "0500"),
        (&cnf, "/input.cnf", "0400"),
        (&proof, "/proof.lrat", "0400"),
    ] {
        command.args([
            "--perms",
            mode,
            "--ro-bind-data",
            &file.as_raw_fd().to_string(),
            path,
        ]);
    }
    command
        .args(["--remount-ro", "/", "--", "/checker"])
        .arg(format!("--CML_HEAP_SIZE={mb}"))
        .arg(format!("--CML_STACK_SIZE={mb}"))
        .args(["/input.cnf", "/proof.lrat"])
        .stdin(Stdio::null())
        .stdout(stdout.try_clone()?)
        .stderr(stderr.try_clone()?);
    let start = Instant::now();
    let mut child = Running(command.spawn()?);
    let status = loop {
        let finished = child.0.try_wait()?;
        if start.elapsed() >= Duration::from_millis(b.wall_ms) {
            if finished.is_none() {
                child.0.kill()?;
            }
            child.0.wait()?;
            return Ok(CheckOutcome::Unknown(Exhausted::Time));
        }
        if let Some(status) = finished {
            break status;
        }
        if stdout.metadata()?.len() > 65_536 || stderr.metadata()?.len() > 65_536 {
            child.0.kill()?;
            child.0.wait()?;
            return Ok(CheckOutcome::Unknown(Exhausted::Size));
        }
        std::thread::sleep(Duration::from_millis(2));
    };
    let code = status
        .code()
        .ok_or_else(|| fail("launcher terminated by signal"))?;
    if code == 152 {
        return Ok(CheckOutcome::Unknown(Exhausted::Time));
    }
    if code == 153 {
        return Ok(CheckOutcome::Unknown(Exhausted::Size));
    }
    if matches!(code, 128..=255) {
        return Err(fail("checker terminated by an unexpected signal"));
    }
    stdout.rewind()?;
    stderr.rewind()?;
    let mut out = Vec::new();
    let mut err = Vec::new();
    stdout.take(65_537).read_to_end(&mut out)?;
    stderr.take(65_537).read_to_end(&mut err)?;
    crate::decode_cake_output(code, &out, &err).map_err(|e| crate::cake::CakeError {
        message: format!(
            "{e}; status={code} stdout={:?} stderr={:?}",
            String::from_utf8_lossy(&out),
            String::from_utf8_lossy(&err)
        ),
        exhausted: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    fn probe() -> Vec<u8> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../spec/vectors/cake-lpr")
            .join(format!("probe-{}.xz", std::env::consts::ARCH));
        let output=Command::new("python3").args(["-c","import lzma,sys;sys.stdout.buffer.write(lzma.decompress(open(sys.argv[1],'rb').read()))"])
            .arg(path).output().unwrap();
        assert!(output.status.success());
        output.stdout
    }
    #[test]
    fn actual_namespace_network_filesystem_environment_and_rlimits() {
        let exe = probe();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let _reachable = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let b = Budget::new(1_000_000, 128 * 1024 * 1024, 1_000_000, 10_000);
        let proof = format!("{} {}", b.mem_bytes, listener.local_addr().unwrap().port());
        assert_eq!(
            run(&exe, b"I", proof.as_bytes(), &b).unwrap(),
            CheckOutcome::Accepted
        );
        // Fora do sandbox o mesmo probe precisa detectar o ambiente/host sem restrições.
        let p = std::env::temp_dir().join(format!("logos-cake-probe-{}", std::process::id()));
        std::fs::create_dir(&p).unwrap();
        std::fs::write(p.join("exe"), exe).unwrap();
        std::fs::set_permissions(p.join("exe"), std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(p.join("cnf"), b"I").unwrap();
        std::fs::write(p.join("proof"), proof).unwrap();
        let outside = Command::new(p.join("exe"))
            .arg(p.join("cnf"))
            .arg(p.join("proof"))
            .env("PATH", "visible")
            .output()
            .unwrap();
        assert!(!outside.status.success());
        std::fs::remove_dir_all(p).unwrap();
    }
    #[test]
    fn unexpected_process_crash_is_an_error_not_resource_exhaustion() {
        let b = Budget::new(0, 128 * 1024 * 1024, 1_000_000, 10_000);
        let error = run(&probe(), b"S", b"", &b).unwrap_err();
        assert!(error.to_string().contains("unexpected signal"));
    }
    #[test]
    fn sealed_inputs_cannot_be_written_or_resized() {
        let mut file = memfile(b"snapshot", true).unwrap();
        assert!(file.write_all(b"changed").is_err());
        assert!(file.set_len(0).is_err());
        assert!(file.set_len(9999).is_err());
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, b"snapshot");
    }
    fn timeout_processes() -> usize {
        std::fs::read_dir("/proc")
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter_map(|p| std::fs::read_to_string(p.path().join("comm")).ok())
            .filter(|name| name.trim() == "logos-cake-tmo")
            .count()
    }
    #[test]
    fn timeout_kills_namespace_with_descendants_and_output_is_bounded() {
        let exe = probe();
        let b = Budget::new(0, 128 * 1024 * 1024, 1_000_000, 500);
        let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
        let flag = stop.clone();
        let monitor = std::thread::spawn(move || {
            let mut observed = false;
            while !flag.load(std::sync::atomic::Ordering::Relaxed) {
                observed |= timeout_processes() > 0;
                std::thread::sleep(Duration::from_millis(5));
            }
            observed
        });
        let start = Instant::now();
        assert_eq!(
            run(&exe, b"T", b"", &b).unwrap(),
            CheckOutcome::Unknown(Exhausted::Time)
        );
        assert!(start.elapsed() < Duration::from_secs(5));
        stop.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(
            monitor.join().unwrap(),
            "probe did not start before deadline"
        );
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(
            timeout_processes(),
            0,
            "namespace descendant survived timeout"
        );
        let outcome = run(
            &exe,
            b"O",
            b"",
            &Budget::new(0, b.mem_bytes, 1_000_000, 10_000),
        );
        assert!(!matches!(outcome, Ok(CheckOutcome::Accepted)));
    }
}
