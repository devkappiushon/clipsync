//! Roteiros do usuário como testes: init, pair, mesh, pause (contrato §9 S1/S4) e clip (§9 I).
//! Cada teste sobe processos `clipsync` de verdade, em perfis temporários, com adaptadores falsos.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)] // helpers outside #[test] fns

use std::io::{BufRead, BufReader, Write};
use std::net::UdpSocket;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_clipsync");
/// Generous on purpose: under a loaded `cargo test --workspace` the mesh needs the dialing delay of the
/// larger PeerId (2 s) plus backoff retries before a peer shows as connected.
const WAIT: Duration = Duration::from_secs(60);

/// These tests start real agents and share the CPU with the rest of the workspace: run them one at a time.
fn serial() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn cmd(profile: &Path, args: &[&str]) -> Command {
    let mut c = Command::new(BIN);
    c.arg("--profile")
        .arg(profile)
        .args(args)
        .env("CLIPSYNC_LOG", "info");
    c
}

/// Runs to completion with `stdin` fed in; returns (exit code, stdout, stderr).
fn run(profile: &Path, args: &[&str], stdin: &str) -> (i32, String, String) {
    let mut child = cmd(profile, args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(stdin.as_bytes())
        .unwrap();
    let Output {
        status,
        stdout,
        stderr,
    } = child.wait_with_output().unwrap();
    (
        status.code().unwrap(),
        String::from_utf8_lossy(&stdout).into_owned(),
        String::from_utf8_lossy(&stderr).into_owned(),
    )
}

fn free_udp_port() -> u16 {
    UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

/// A long-running `clipsync` child with its stdout read on a thread. Killed on drop.
struct Proc {
    child: Child,
    lines: Receiver<String>,
    seen: Vec<String>,
    stderr_path: PathBuf,
}

impl Proc {
    fn spawn(profile: &Path, args: &[&str]) -> Self {
        let stderr_path = profile.with_extension(format!("{}.err", args.join("_")));
        let stderr = std::fs::File::create(&stderr_path).unwrap();
        let mut child = cmd(profile, args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(stderr)
            .spawn()
            .unwrap();
        let out = child.stdout.take().unwrap();
        let (tx, lines) = mpsc::channel();
        std::thread::spawn(move || {
            for l in BufReader::new(out).lines().map_while(Result::ok) {
                if tx.send(l).is_err() {
                    break;
                }
            }
        });
        Self {
            child,
            lines,
            seen: Vec::new(),
            stderr_path,
        }
    }

    /// Waits for a stdout line containing `needle` and returns it.
    fn wait_line(&mut self, needle: &str) -> String {
        if let Some(l) = self.seen.iter().find(|l| l.contains(needle)) {
            return l.clone();
        }
        let end = Instant::now() + WAIT;
        while let Some(left) = end.checked_duration_since(Instant::now()) {
            match self.lines.recv_timeout(left) {
                Ok(l) => {
                    self.seen.push(l.clone());
                    if l.contains(needle) {
                        return l;
                    }
                }
                Err(_) => break,
            }
        }
        panic!(
            "timeout waiting for {needle:?}; saw {:?}; stderr: {}",
            self.seen,
            self.stderr()
        );
    }

    fn say(&mut self, text: &str) {
        let stdin = self.child.stdin.as_mut().unwrap();
        stdin.write_all(text.as_bytes()).unwrap();
        stdin.flush().unwrap();
    }

    fn finish(mut self) -> i32 {
        drop(self.child.stdin.take());
        let end = Instant::now() + WAIT;
        loop {
            if let Some(st) = self.child.try_wait().unwrap() {
                return st.code().unwrap_or(-1);
            }
            assert!(
                Instant::now() < end,
                "process did not exit; stderr: {}",
                self.stderr()
            );
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn stderr(&self) -> String {
        std::fs::read_to_string(&self.stderr_path).unwrap_or_default()
    }

    fn kill(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for Proc {
    fn drop(&mut self) {
        self.kill();
    }
}

fn init(dir: &Path, name: &str, modules: &str, port: u16) {
    let (code, _, err) = run(
        dir,
        &[
            "init",
            "--name",
            name,
            "--modules",
            modules,
            "--port",
            &port.to_string(),
            "--fake-adapters",
        ],
        "",
    );
    assert_eq!(code, 0, "{err}");
}

fn code_of(line: &str) -> String {
    line.split("Código: ").nth(1).unwrap().trim().to_owned()
}

/// Pairs `host` and `join` (both answer the code with "s"). `host_allow`/`join_allow` pre-answer permissions.
/// Returns (host exit, join exit, host code, join code).
fn pair(
    host: &Path,
    join: &Path,
    host_allow: &str,
    join_allow: &str,
    host_ans: &str,
    join_ans: &str,
) -> (i32, i32, String, String) {
    let mut h = Proc::spawn(host, &["pair", "host", "--allow", host_allow]);
    let waiting = h.wait_line("Aguardando em");
    let port = waiting
        .split("Aguardando em ")
        .nth(1)
        .unwrap()
        .split_whitespace()
        .next()
        .unwrap()
        .rsplit(':')
        .next()
        .unwrap()
        .to_owned();
    let mut j = Proc::spawn(
        join,
        &[
            "pair",
            "join",
            &format!("127.0.0.1:{port}"),
            "--allow",
            join_allow,
        ],
    );
    let (hc, jc) = (
        code_of(&h.wait_line("Código:")),
        code_of(&j.wait_line("Código:")),
    );
    h.say(host_ans);
    j.say(join_ans);
    (h.finish(), j.finish(), hc, jc)
}

fn peers_text(dir: &Path) -> String {
    std::fs::read_to_string(dir.join("peers.toml")).unwrap()
}

fn wait_status(dir: &Path, needle: &str) -> String {
    let end = Instant::now() + WAIT;
    loop {
        let (code, out, _) = run(dir, &["status"], "");
        if code == 0 && out.contains(needle) {
            return out;
        }
        assert!(
            Instant::now() < end,
            "status never showed {needle:?}; last: {out}"
        );
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// The peer left: `status` stops showing it as connected (it is retrying or disconnected).
fn wait_not_connected(dir: &Path) {
    let end = Instant::now() + WAIT;
    loop {
        let (_, out, _) = run(dir, &["status"], "");
        if !out.contains("· conectado ·") {
            return;
        }
        assert!(Instant::now() < end, "peer still shown as connected: {out}");
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Two paired, running agents `a` (notebook-a) and `b` (notebook-b). `b_modules` is what B runs.
struct Mesh {
    _tmp: tempfile::TempDir,
    a: PathBuf,
    b: PathBuf,
    port_a: u16,
    port_b: u16,
    procs: Vec<Proc>,
}

fn mesh(a_modules: &str, b_modules: &str, a_allow: &str, b_allow: &str) -> Mesh {
    let tmp = tempfile::tempdir().unwrap();
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    let (port_a, port_b) = (free_udp_port(), free_udp_port());
    init(&a, "notebook-a", a_modules, port_a);
    init(&b, "notebook-b", b_modules, port_b);
    let (hx, jx, hc, jc) = pair(&a, &b, a_allow, b_allow, "s\n", "s\n");
    assert_eq!((hx, jx), (0, 0));
    assert_eq!(hc, jc);
    let mut m = Mesh {
        _tmp: tmp,
        a,
        b,
        port_a,
        port_b,
        procs: Vec::new(),
    };
    m.procs.push(Proc::spawn(&m.a.clone(), &["run"]));
    m.procs.push(Proc::spawn(&m.b.clone(), &["run"]));
    wait_status(&m.a, "· conectado ·");
    wait_status(&m.b, "· conectado ·");
    m
}

// ---------------- §9 S1 item 2: init ----------------

#[test]
fn init_creates_profile_files_with_private_modes() {
    let _serial = serial();
    use std::os::unix::fs::PermissionsExt;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("cs-a");
    let (code, out, err) = run(&dir, &["init"], "notebook-a\nn\n");
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("Nada foi transmitido"), "{out}");
    let mode = |p: &str| std::fs::metadata(dir.join(p)).unwrap().permissions().mode() & 0o777;
    assert_eq!(
        (
            mode("identity.key.pem"),
            mode("config.toml"),
            mode("peers.toml")
        ),
        (0o600, 0o600, 0o600)
    );
    let cfg = std::fs::read_to_string(dir.join("config.toml")).unwrap();
    assert!(
        cfg.contains("clipboard = false") && cfg.contains("notebook-a"),
        "{cfg}"
    );
    assert!(!peers_text(&dir).contains("[[peer]]"));
}

#[test]
fn init_without_a_clipboard_answer_writes_false_and_flags_variant_works() {
    let _serial = serial();
    let tmp = tempfile::tempdir().unwrap();
    let (code, ..) = run(&tmp.path().join("x"), &["init"], "so-o-nome\n");
    assert_eq!(code, 0);
    assert!(
        std::fs::read_to_string(tmp.path().join("x/config.toml"))
            .unwrap()
            .contains("clipboard = false")
    );
    let (code, ..) = run(
        &tmp.path().join("y"),
        &[
            "init",
            "--name",
            "y",
            "--modules",
            "input,clipboard",
            "--fake-adapters",
        ],
        "",
    );
    assert_eq!(code, 0);
    let cfg = std::fs::read_to_string(tmp.path().join("y/config.toml")).unwrap();
    assert!(
        cfg.contains("clipboard = true") && cfg.contains("adapter = \"fake\""),
        "{cfg}"
    );
}

#[test]
fn init_refuses_a_config_with_both_modules_off_and_says_why() {
    let _serial = serial();
    let tmp = tempfile::tempdir().unwrap();
    let (code, _, err) = run(
        &tmp.path().join("z"),
        &["init", "--name", "z", "--modules", ""],
        "",
    );
    assert_eq!(code, 1);
    assert!(err.contains("pelo menos um módulo"), "{err}");
    assert!(!tmp.path().join("z/config.toml").exists());
}

// ---------------- §9 S1 item 3: pair ----------------

#[test]
fn pair_shows_the_same_code_and_saves_permissions_on_both_sides() {
    let _serial = serial();
    let tmp = tempfile::tempdir().unwrap();
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    init(&a, "notebook-a", "input,clipboard", 47810);
    init(&b, "notebook-b", "input,clipboard", 47811);
    let (hx, jx, hc, jc) = pair(
        &a,
        &b,
        "clipboard-in,clipboard-out",
        "clipboard-out",
        "s\n",
        "s\n",
    );
    assert_eq!((hx, jx), (0, 0));
    assert_eq!(hc, jc, "same numeric code on both ends");
    assert!(
        hc.len() == 7 && hc.as_bytes()[3] == b' ',
        "NNN NNN, got {hc:?}"
    );
    let (pa, pb) = (peers_text(&a), peers_text(&b));
    assert!(
        pa.contains("name = \"notebook-b\"")
            && pa.contains("clipboard_in = true")
            && pa.contains("clipboard_out = true")
            && pa.contains("input_in = false"),
        "{pa}"
    );
    assert!(
        pb.contains("name = \"notebook-a\"")
            && pb.contains("clipboard_in = false")
            && pb.contains("clipboard_out = true"),
        "{pb}"
    );
}

#[test]
fn pair_declined_on_one_side_saves_nothing_on_either() {
    let _serial = serial();
    let tmp = tempfile::tempdir().unwrap();
    let (a, b) = (tmp.path().join("a"), tmp.path().join("b"));
    init(&a, "notebook-a", "clipboard", 47812);
    init(&b, "notebook-b", "clipboard", 47813);
    let (hx, jx, ..) = pair(&a, &b, "clipboard-in", "clipboard-in", "s\n", "n\n");
    assert_eq!(hx, 3, "pair host exits 3 when the attempt fails");
    assert_eq!(jx, 3);
    assert!(!peers_text(&a).contains("[[peer]]") && !peers_text(&b).contains("[[peer]]"));
}

// ---------------- §9 S1 item 5: mesh ----------------

#[test]
fn mesh_connects_refuses_strangers_reconnects_and_flags_a_changed_key() {
    let _serial = serial();
    let mut m = mesh(
        "input,clipboard",
        "input,clipboard",
        "clipboard-out",
        "clipboard-in",
    );
    let status = wait_status(&m.a, "· conectado ·");
    assert!(
        status.contains("notebook-b")
            && status.contains("clipboard")
            && status.contains("permissões"),
        "{status}"
    );

    // a third identity that knows A's address and A's pin, but that A never paired with
    let c = m._tmp.path().join("c");
    init(&c, "notebook-c", "input,clipboard", free_udp_port());
    let a_entry = peers_text(&m.b); // B's peers.toml holds A's full id
    let a_id = a_entry
        .lines()
        .find(|l| l.starts_with("id = "))
        .unwrap()
        .to_owned();
    std::fs::write(
        c.join("peers.toml"),
        format!("version = 1\n[[peer]]\n{a_id}\nname = \"notebook-a\"\naddr = \"127.0.0.1:{}\"\npaired_at = \"2026-10-27T14:00:00Z\"\nclipboard_in = false\nclipboard_out = false\ninput_in = false\n", m.port_a),
    )
    .unwrap();
    let mut pc = Proc::spawn(&c, &["run"]);
    pc.wait_line("no ar");
    let end = Instant::now() + WAIT;
    while !m.procs[0].stderr().contains("par desconhecido recusado") {
        assert!(
            Instant::now() < end,
            "A never logged the refusal; stderr: {}",
            m.procs[0].stderr()
        );
        std::thread::sleep(Duration::from_millis(200));
    }
    let (_, c_status, _) = run(&c, &["status"], "");
    assert!(
        !c_status.contains("· conectado ·"),
        "stranger must not be connected: {c_status}"
    );
    drop(pc);

    // kill B and bring it back: reconnects
    m.procs[1].kill();
    wait_not_connected(&m.a);
    m.procs[1] = Proc::spawn(&m.b.clone(), &["run"]);
    wait_status(&m.a, "· conectado ·");

    // B gets a NEW identity at the same address: A flags the changed key
    m.procs[1].kill();
    let (code, ..) = run(
        &m.b,
        &[
            "init",
            "--name",
            "notebook-b",
            "--modules",
            "input,clipboard",
            "--port",
            &m.port_b.to_string(),
            "--fake-adapters",
        ],
        "s\n",
    );
    assert_eq!(code, 0);
    m.procs[1] = Proc::spawn(&m.b.clone(), &["run"]);
    let status = wait_status(&m.a, "chave do par mudou");
    assert!(status.contains("bloqueado"), "{status}");
}

// ---------------- §9 S1 item 8: pause ----------------

#[test]
fn pause_blocks_and_resume_returns_and_state_survives() {
    let _serial = serial();
    let m = mesh("clipboard", "clipboard", "clipboard-out", "clipboard-in");
    let (code, out, _) = run(&m.b, &["pause"], "");
    assert_eq!(code, 0);
    assert!(out.contains("pausado"));
    assert!(wait_status(&m.b, "PAUSADO").contains("PAUSADO"));
    assert!(
        std::fs::read_to_string(m.b.join("state.toml"))
            .unwrap()
            .contains("paused = true")
    );
    let (code, ..) = run(&m.b, &["resume"], "");
    assert_eq!(code, 0);
    assert!(wait_status(&m.b, "ativo").contains("ativo"));
    assert!(
        std::fs::read_to_string(m.b.join("state.toml"))
            .unwrap()
            .contains("paused = false")
    );
    // no agent: exit 4
    let tmp = tempfile::tempdir().unwrap();
    init(
        &tmp.path().join("solo"),
        "solo",
        "clipboard",
        free_udp_port(),
    );
    assert_eq!(run(&tmp.path().join("solo"), &["pause"], "").0, 4);
    assert_eq!(run(&tmp.path().join("solo"), &["status"], "").0, 4);
}

// ---------------- §9 I item 1/2: clipboard end to end ----------------

#[test]
fn clip_send_arrives_in_the_inbox_and_is_refused_when_the_module_is_off() {
    let _serial = serial();
    let m = mesh("clipboard", "clipboard", "clipboard-out", "clipboard-in");
    let (code, out, err) = run(&m.a, &["send", "--stdin"], "olá ção");
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("notebook-b: aplicado"), "{out}");
    let (code, inbox, _) = run(&m.b, &["inbox"], "");
    assert_eq!(code, 0);
    assert!(
        inbox.contains("bytes=10") && inbox.contains("olá ção"),
        "{inbox}"
    );

    // B pausado: send recusado com "pausado"; resume; nada reenviado
    assert_eq!(run(&m.b, &["pause"], "").0, 0);
    let (code, out, _) = run(&m.a, &["send", "--stdin"], "durante a pausa");
    assert_eq!(code, 3);
    assert!(out.contains("pausado"), "{out}");
    assert_eq!(run(&m.b, &["resume"], "").0, 0);
    std::thread::sleep(Duration::from_secs(1));
    let (_, after, _) = run(&m.b, &["inbox"], "");
    assert_eq!(
        after.lines().count(),
        inbox.lines().count(),
        "inbox unchanged: {after}"
    );

    // B sem o módulo clipboard: recusado com motivo
    let tmp = tempfile::tempdir().unwrap();
    let (a2, b2) = (tmp.path().join("a"), tmp.path().join("b"));
    init(&a2, "notebook-a", "clipboard", free_udp_port());
    init(&b2, "notebook-b", "input", free_udp_port());
    let (hx, jx, ..) = pair(&a2, &b2, "clipboard-out", "", "s\n", "s\n");
    assert_eq!((hx, jx), (0, 0));
    let _pa = Proc::spawn(&a2, &["run"]);
    let _pb = Proc::spawn(&b2, &["run"]);
    wait_status(&a2, "· conectado ·");
    let (code, out, _) = run(&a2, &["send", "--stdin"], "x");
    assert_eq!(code, 3, "{out}");
    assert!(
        out.contains("recusado: módulo clipboard desligado em notebook-b"),
        "{out}"
    );
}
