#![allow(clippy::unwrap_used, clippy::expect_used)]
use clipsync_core::{InitOptions, Profile, State};
use clipsync_proto::ModuleSet;
use std::fs::{self, OpenOptions};

fn fixture() -> (tempfile::TempDir, Profile) {
    let dir = tempfile::Builder::new().prefix("csm-").tempdir().unwrap();
    let p = Profile::resolve(Some(dir.path().join("p"))).unwrap();
    p.init(InitOptions {
        device_name: "teste-modulos".into(),
        modules: ModuleSet {
            input: true,
            clipboard: true,
        },
        listen: "127.0.0.1:0".parse().unwrap(),
        fake_adapters: true,
    })
    .unwrap();
    (dir, p)
}

// Detecta o escritor que trunca estado inválido em vez de preservar a evidência.
#[test]
fn pausa_recusa_estado_desconhecido_sem_reescrever() {
    let (_dir, p) = fixture();
    for invalid in [
        "version = 2\npaused = false\n",
        "version = 1\npaused = false\nfuturo = 7\n",
    ] {
        fs::write(p.state_path(), invalid).unwrap();
        assert!(
            p.save_state(&State {
                version: 1,
                paused: true
            })
            .is_err()
        );
        assert_eq!(fs::read_to_string(p.state_path()).unwrap(), invalid);
    }
}

// Detecta set_clipboard_mode escrevendo enquanto outra transação possui a configuração.
#[test]
fn modo_clipboard_respeita_reserva_de_configuracao() {
    let (_dir, p) = fixture();
    let before = fs::read(p.config_path()).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(p.dir.join("config.toml.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(p.set_clipboard_mode("manual").is_err());
    assert_eq!(fs::read(p.config_path()).unwrap(), before);
}

// Detecta um init que substitui a identidade durante a reserva de reconfiguração.
#[test]
fn init_respeita_reserva_exclusiva_do_perfil() {
    let (_dir, p) = fixture();
    let before = fs::read(p.dir.join("identity.key.pem")).unwrap();
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(p.dir.join("profile-operation.lock"))
        .unwrap();
    lock.lock().unwrap();
    assert!(
        p.init(InitOptions {
            device_name: "outro".into(),
            modules: ModuleSet {
                input: false,
                clipboard: true
            },
            listen: "127.0.0.1:0".parse().unwrap(),
            fake_adapters: true,
        })
        .is_err()
    );
    assert_eq!(fs::read(p.dir.join("identity.key.pem")).unwrap(), before);
}

#[test]
fn modulos_preservam_tabelas_e_arquivos_do_perfil() {
    let (_dir, p) = fixture();
    for name in ["layout.toml", "ui.toml", "token.fake"] {
        fs::write(p.dir.join(name), format!("fixture {name}\n")).unwrap();
    }
    let names = [
        "identity.key.pem",
        "identity.cert.pem",
        "peers.toml",
        "layout.toml",
        "ui.toml",
        "token.fake",
    ];
    let bytes: Vec<_> = names
        .iter()
        .map(|n| fs::read(p.dir.join(n)).unwrap())
        .collect();
    let before = p.load_config().unwrap();
    let op = p.reserve_operation(true).unwrap();
    let mut edit = p.begin_config_edit(&op, before.modules).unwrap();
    assert!(
        edit.set_modules(ModuleSet {
            input: false,
            clipboard: true
        })
        .unwrap()
    );
    let after = p.load_config().unwrap();
    assert!(!after.modules.input && after.modules.clipboard);
    assert_eq!(
        (
            after.version,
            after.device_name,
            after.listen,
            after.input,
            after.clipboard
        ),
        (
            before.version,
            before.device_name,
            before.listen,
            before.input,
            before.clipboard
        )
    );
    for (name, original) in names.iter().zip(bytes) {
        assert_eq!(fs::read(p.dir.join(name)).unwrap(), original);
    }
    assert!(!p.state_path().exists());
}

#[test]
fn modulos_noop_preserva_bytes_e_tabela_ausente() {
    let (_dir, p) = fixture();
    let mut doc: toml::Value =
        toml::from_str(&fs::read_to_string(p.config_path()).unwrap()).unwrap();
    doc.as_table_mut().unwrap().remove("input");
    let original = format!("# preservar comentario\n{}", toml::to_string(&doc).unwrap());
    fs::write(p.config_path(), &original).unwrap();
    let set = ModuleSet {
        input: true,
        clipboard: true,
    };
    let op = p.reserve_operation(true).unwrap();
    let mut edit = p.begin_config_edit(&op, set).unwrap();
    assert!(!edit.set_modules(set).unwrap());
    assert_eq!(fs::read_to_string(p.config_path()).unwrap(), original);
    assert!(!p.state_path().exists());
}

#[test]
fn modulos_recusam_expected_antigo_e_conflito_em_outra_tabela() {
    let (_dir, p) = fixture();
    let op = p.reserve_operation(true).unwrap();
    let next = ModuleSet {
        input: false,
        clipboard: true,
    };
    assert!(p.begin_config_edit(&op, next).is_err());
    let mut edit = p
        .begin_config_edit(
            &op,
            ModuleSet {
                input: true,
                clipboard: true,
            },
        )
        .unwrap();
    let concurrent = fs::read_to_string(p.config_path())
        .unwrap()
        .replace("teste-modulos", "editor-externo");
    fs::write(p.config_path(), &concurrent).unwrap();
    assert!(edit.set_modules(next).is_err());
    assert_eq!(fs::read_to_string(p.config_path()).unwrap(), concurrent);
}

#[test]
fn modulos_recusam_reserva_de_outro_perfil_ou_compartilhada() {
    let (_a, p) = fixture();
    let (_b, q) = fixture();
    let set = ModuleSet {
        input: true,
        clipboard: true,
    };
    let other = q.reserve_operation(true).unwrap();
    assert!(p.begin_config_edit(&other, set).is_err());
    let shared = p.reserve_operation(false).unwrap();
    assert!(p.begin_config_edit(&shared, set).is_err());
}

#[test]
fn processo_concorrente_modo_clipboard() {
    let Some(path) = std::env::var_os("C01_TEST_PROFILE") else {
        return;
    };
    let p = Profile::resolve(Some(path.into())).unwrap();
    assert!(p.set_clipboard_mode("manual").is_err());
}

#[test]
fn reserva_impede_outro_processo_e_modo_posterior_preserva_modulos() {
    let (_dir, p) = fixture();
    let op = p.reserve_operation(true).unwrap();
    let mut edit = p
        .begin_config_edit(
            &op,
            ModuleSet {
                input: true,
                clipboard: true,
            },
        )
        .unwrap();
    let result = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "processo_concorrente_modo_clipboard",
            "--nocapture",
        ])
        .env("C01_TEST_PROFILE", &p.dir)
        .status()
        .unwrap();
    assert!(result.success());
    edit.set_modules(ModuleSet {
        input: false,
        clipboard: true,
    })
    .unwrap();
    drop(edit);
    drop(op);
    p.set_clipboard_mode("manual").unwrap();
    let cfg = p.load_config().unwrap();
    assert!(!cfg.modules.input && cfg.modules.clipboard);
    assert_eq!(cfg.clipboard.unwrap()["mode"].as_str(), Some("manual"));
}

#[test]
fn escritor_em_processo_separado() {
    let Some(path) = std::env::var_os("C01_WRITER_PROFILE") else {
        return;
    };
    let p = Profile::resolve(Some(path.into())).unwrap();
    let action = std::env::var("C01_WRITER_ACTION").unwrap();
    let id = std::env::var("C01_WRITER_ID").unwrap();
    fs::write(p.dir.join(format!("ready-{id}")), b"ready").unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !p.dir.join("go").exists() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let outcome = loop {
        let result = if action == "mode" {
            p.set_clipboard_mode("manual").map(|()| "saved")
        } else {
            p.reserve_operation(true).and_then(|op| {
                p.begin_config_edit(
                    &op,
                    ModuleSet {
                        input: true,
                        clipboard: true,
                    },
                )
                .and_then(|mut edit| {
                    edit.set_modules(ModuleSet {
                        input: false,
                        clipboard: true,
                    })
                    .map(|_| "saved")
                })
            })
        };
        match result {
            Ok(outcome) => break outcome,
            Err(clipsync_core::CoreError::Profile(s)) if s == "stale_config" => break "stale",
            Err(clipsync_core::CoreError::Profile(s)) if s == "profile_busy" => {
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            Err(error) => panic!("erro inesperado do escritor: {error}"),
        }
    };
    fs::write(p.dir.join(format!("result-{id}")), outcome).unwrap();
}

fn race_writers(actions: [&str; 2]) -> (tempfile::TempDir, Profile, Vec<String>) {
    let (dir, p) = fixture();
    let mut children: Vec<_> = actions
        .iter()
        .enumerate()
        .map(|(id, action)| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "escritor_em_processo_separado"])
                .env("C01_WRITER_PROFILE", &p.dir)
                .env("C01_WRITER_ACTION", action)
                .env("C01_WRITER_ID", id.to_string())
                .spawn()
                .unwrap()
        })
        .collect();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while !(0..2).all(|id| p.dir.join(format!("ready-{id}")).exists()) {
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    fs::write(p.dir.join("go"), b"go").unwrap();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    let outcomes = (0..2)
        .map(|id| fs::read_to_string(p.dir.join(format!("result-{id}"))).unwrap())
        .collect();
    (dir, p, outcomes)
}

#[test]
fn dois_processos_modulos_e_clipboard_nao_perdem_alteracoes() {
    let (_d, p, outcomes) = race_writers(["modules", "mode"]);
    assert_eq!(outcomes, ["saved", "saved"]);
    let cfg = p.load_config().unwrap();
    assert!(!cfg.modules.input && cfg.modules.clipboard);
    assert_eq!(
        cfg.clipboard.as_ref().unwrap()["mode"].as_str(),
        Some("manual")
    );
    assert_eq!(cfg.clipboard.unwrap()["adapter"].as_str(), Some("fake"));
    assert_eq!(cfg.input.unwrap()["capture"].as_str(), Some("fake"));
}

#[test]
fn dois_processos_mesmo_expected_so_um_salva() {
    let (_d, p, mut outcomes) = race_writers(["modules", "modules"]);
    outcomes.sort();
    assert_eq!(outcomes, ["saved", "stale"]);
    assert!(!p.load_config().unwrap().modules.input);
}

#[cfg(unix)]
#[test]
fn falha_de_gravacao_nao_altera_configuracao_nem_identidade() {
    use std::os::unix::fs::PermissionsExt;
    let (_d, p) = fixture();
    let original = fs::read(p.config_path()).unwrap();
    let identity = fs::read(p.dir.join("identity.key.pem")).unwrap();
    let op = p.reserve_operation(true).unwrap();
    let mut edit = p
        .begin_config_edit(
            &op,
            ModuleSet {
                input: true,
                clipboard: true,
            },
        )
        .unwrap();
    fs::set_permissions(&p.dir, fs::Permissions::from_mode(0o500)).unwrap();
    let result = edit.set_modules(ModuleSet {
        input: false,
        clipboard: true,
    });
    fs::set_permissions(&p.dir, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(matches!(result, Err(clipsync_core::CoreError::Io(_))));
    assert_eq!(fs::read(p.config_path()).unwrap(), original);
    assert!(fs::read(p.dir.join("identity.key.pem")).unwrap() == identity);
}

#[test]
fn clone_da_reserva_so_libera_no_ultimo_guard() {
    let (_d, p) = fixture();
    let first = p.reserve_operation(true).unwrap();
    let last = first.clone();
    drop(first);
    assert!(p.reserve_operation(false).is_err());
    drop(last);
    assert!(p.reserve_operation(true).is_ok());
}

#[test]
fn erros_de_configuracao_e_estado_nao_expoem_conteudo() {
    let (_d, p) = fixture();
    let marker = "marcador-privado-fake";
    let invalid = format!("version = \"{marker}\"\n");
    fs::write(p.config_path(), &invalid).unwrap();
    for error in [
        p.load_config().unwrap_err(),
        p.set_clipboard_mode("manual").unwrap_err(),
    ] {
        assert!(!error.to_string().contains(marker));
    }
    fs::write(p.state_path(), &invalid).unwrap();
    assert!(!p.set_paused(true).unwrap_err().to_string().contains(marker));
    assert_eq!(fs::read_to_string(p.config_path()).unwrap(), invalid);
    assert_eq!(fs::read_to_string(p.state_path()).unwrap(), invalid);
}

#[test]
fn edicao_compartilhada_de_modo_nao_autoriza_modulos() {
    let (_d, p) = fixture();
    let operation = p.reserve_operation(false).unwrap();
    let mut edit = p.begin_clipboard_edit(&operation).unwrap();
    let before = fs::read(p.config_path()).unwrap();
    assert!(
        edit.set_modules(ModuleSet {
            input: false,
            clipboard: true
        })
        .is_err()
    );
    assert_eq!(fs::read(p.config_path()).unwrap(), before);
}
