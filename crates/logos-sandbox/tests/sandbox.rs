//! Etapa 5.11 (ADR0059): plano do sandbox e proxy do LLM.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use std::ffi::OsString;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};

use logos_sandbox::{CODE, KERNEL_SOCKET, LLM_SOCKET, LlmProxy, Plan, STAGING, SandboxError};

struct Temp(PathBuf);
impl Temp {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(format!(
            "logos-sandbox-{}-{name}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&path).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        Self(path)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn plan(root: &Path) -> (Plan, UnixListener) {
    let socket = root.join("kernel.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    for dir in ["staging", "code"] {
        std::fs::create_dir(root.join(dir)).unwrap();
    }
    (
        Plan {
            kernel_socket: socket,
            staging: root.join("staging"),
            code: root.join("code"),
            llm_socket: None,
            program: vec!["/usr/bin/true".into()],
        },
        listener,
    )
}

fn invalid(plan: &Plan) -> &'static str {
    match plan.arguments() {
        Err(SandboxError::Invalid(reason)) => reason,
        other => panic!("expected Invalid, got {other:?}"),
    }
}

#[test]
fn arguments_mount_only_the_granted_paths_and_isolate_every_namespace() {
    let temp = Temp::new("args");
    let (plan, _listener) = plan(&temp.0);
    let args: Vec<String> = plan
        .arguments()
        .unwrap()
        .into_iter()
        .map(|a| a.into_string().unwrap())
        .collect();
    for flag in [
        "--unshare-user",
        "--unshare-pid",
        "--unshare-net",
        "--unshare-ipc",
        "--unshare-uts",
        "--die-with-parent",
        "--clearenv",
    ] {
        assert!(args.contains(&flag.to_string()), "{flag}");
    }
    assert_eq!(
        args[args.iter().position(|a| a == "--cap-drop").unwrap() + 1],
        "ALL"
    );
    // Toda montagem com origem no host: só runtime, CA/loader e o concedido.
    let mut sources = Vec::new();
    for (i, arg) in args.iter().enumerate() {
        if matches!(arg.as_str(), "--bind" | "--ro-bind" | "--ro-bind-try") {
            sources.push((arg.as_str(), args[i + 1].as_str(), args[i + 2].as_str()));
        }
    }
    let root = temp.0.to_str().unwrap();
    assert_eq!(
        sources,
        vec![
            ("--ro-bind", "/usr", "/usr"),
            ("--ro-bind-try", "/etc/pki", "/etc/pki"),
            ("--ro-bind-try", "/etc/ssl", "/etc/ssl"),
            ("--ro-bind-try", "/etc/ld.so.cache", "/etc/ld.so.cache"),
            ("--bind", &*format!("{root}/kernel.sock"), KERNEL_SOCKET),
            ("--bind", &*format!("{root}/staging"), STAGING),
            ("--ro-bind", &*format!("{root}/code"), CODE),
        ]
    );
    assert!(
        !args
            .iter()
            .any(|a| a.contains("--share-net") || a == "--dev-bind")
    );
    assert_eq!(&args[args.len() - 2..], ["--", "/usr/bin/true"]);
}

#[test]
fn invalid_plans_are_refused_before_anything_runs() {
    let temp = Temp::new("invalid");
    let (base, _listener) = plan(&temp.0);
    let mut p = base.clone();
    p.program.clear();
    assert_eq!(invalid(&p), "empty program");
    let mut p = base.clone();
    p.kernel_socket = PathBuf::from("relative.sock");
    assert_eq!(invalid(&p), "kernel socket must be absolute");
    let mut p = base.clone();
    p.kernel_socket = temp.0.join("staging");
    assert_eq!(invalid(&p), "kernel socket is not a socket");
    let mut p = base.clone();
    p.staging = base.kernel_socket.clone();
    assert_eq!(invalid(&p), "staging is not a directory");
    // Link simbólico não é seguido: o alvo poderia ser a área do kernel.
    std::os::unix::fs::symlink(temp.0.join("code"), temp.0.join("link")).unwrap();
    let mut p = base.clone();
    p.code = temp.0.join("link");
    assert_eq!(invalid(&p), "code is not a directory");
    let mut p = base.clone();
    p.llm_socket = Some(temp.0.join("code"));
    assert_eq!(invalid(&p), "LLM socket is not a socket");
    let mut p = base;
    p.staging = temp.0.join("missing");
    assert!(matches!(p.arguments(), Err(SandboxError::Io(_))));
}

#[test]
fn llm_proxy_forwards_only_to_the_fixed_upstream_and_needs_a_private_directory() {
    let upstream = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = upstream.local_addr().unwrap();
    std::thread::spawn(move || {
        for connection in upstream.incoming() {
            let mut connection = connection.unwrap();
            let mut request = Vec::new();
            connection.read_to_end(&mut request).unwrap();
            connection.write_all(b"llm:").unwrap();
            connection.write_all(&request).unwrap();
        }
    });
    let public = Temp::new("public");
    std::fs::set_permissions(&public.0, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert!(matches!(
        LlmProxy::start(&public.0.join("llm.sock"), address),
        Err(SandboxError::Invalid("proxy directory must be private"))
    ));
    let private = Temp::new("private");
    let proxy = LlmProxy::start(&private.0.join("llm.sock"), address).unwrap();
    let mode = std::fs::metadata(proxy.path())
        .unwrap()
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    for message in [&b"ping"[..], b"GET / HTTP/1.1\r\nHost: outro\r\n\r\n"] {
        let mut client = UnixStream::connect(proxy.path()).unwrap();
        client.write_all(message).unwrap();
        client.shutdown(std::net::Shutdown::Write).unwrap();
        let mut reply = Vec::new();
        client.read_to_end(&mut reply).unwrap();
        // O cliente não escolhe destino: até um Host diferente vai ao upstream fixo.
        assert_eq!(reply, [&b"llm:"[..], message].concat());
    }
    let mut with_llm = plan(&private.0).0;
    with_llm.llm_socket = Some(proxy.path().to_path_buf());
    let args: Vec<OsString> = with_llm.arguments().unwrap();
    assert!(
        args.windows(3).any(|w| {
            w[0] == "--bind" && w[1] == proxy.path().as_os_str() && w[2] == LLM_SOCKET
        })
    );
}
