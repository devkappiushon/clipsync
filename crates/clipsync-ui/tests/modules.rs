#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod common;
use common::*;
use serde_json::{Value, json};
fn request(input: bool, clipboard: bool) -> Value {
    json!({"cmd":"modules_set","args":{"modules":{"input":input,"clipboard":clipboard},"expected":{"input":true,"clipboard":true}}})
}
#[tokio::test(flavor = "multi_thread")]
async fn noop_nao_pausa_nem_regrava_perfil() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let config = std::fs::read(p.config_path()).unwrap();
    let r = ok(&ui, request(true, true)).await;
    assert_eq!(r["changed"], false);
    assert_eq!(r["effects"]["pause"], "not_requested");
    assert_eq!(r["effects"]["config_saved"], false);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), config);
    assert!(!p.state_path().exists());
    assert_eq!(app_state(&ui).await["paused"], false);
    ui.shutdown().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn muda_modulos_preserva_identidade_e_reinicia_pausado() {
    let d = tempfile::tempdir().unwrap();
    let (p, id) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let r = ok(&ui, request(false, true)).await;
    assert_eq!(r["changed"], true);
    assert_eq!(r["effects"]["start"], "owned_ready");
    assert_eq!(r["observed"]["paused"], true);
    assert_eq!(r["observed"]["agent"], "in_app");
    assert_eq!(p.load_identity().unwrap().id(), id);
    assert!(p.load_state().unwrap().paused);
    assert_eq!(app_state(&ui).await["paused"], true);
    ui.shutdown().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn parado_permanece_parado_sem_criar_pausa() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    ok(&ui, json!({"cmd":"agent_stop"})).await;
    let r = ok(&ui, request(false, true)).await;
    assert_eq!(r["effects"]["start"], "not_requested");
    assert_eq!(r["observed"]["agent"], "stopped");
    assert!(!p.state_path().exists());
    assert!(!p.socket_path().exists());
    ui.shutdown().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn externo_recusado_sem_comandos_nem_gravacao() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let other = fake_agent(&p, |_| panic!("não enviar mutação ao externo")).await;
    let ui = start(d.path(), "p").await;
    let before = std::fs::read(p.config_path()).unwrap();
    let e = err(&ui, request(false, true)).await;
    assert_eq!(e.code, "agent_external");
    assert_eq!(e.details.unwrap()["effects"]["config_saved"], false);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
    ui.shutdown().await;
    other.stop().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn conflito_inicial_e_ocupado_nao_pausam() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let mut bad = request(false, true);
    bad["args"]["expected"]["input"] = json!(false);
    assert_eq!(err(&ui, bad).await.code, "stale_config");
    let guard = p.reserve_operation(false).unwrap();
    assert_eq!(err(&ui, request(false, true)).await.code, "profile_busy");
    assert!(!p.state_path().exists());
    drop(guard);
    ui.shutdown().await;
}
#[tokio::test(flavor = "multi_thread")]
async fn falha_persistencia_reporta_pausa_aplicada_sem_salvar_modulos() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    std::fs::write(p.state_path(), "version=2\npaused=false\n").unwrap();
    let before = std::fs::read(p.config_path()).unwrap();
    let e = err(&ui, request(false, true)).await;
    let details = e.details.unwrap();
    assert_eq!(details["effects"]["pause"], "requested");
    assert_eq!(details["effects"]["pause_persisted"], false);
    assert_eq!(details["effects"]["config_saved"], false);
    assert_eq!(details["observed"]["paused"], true);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn reserva_impede_onboarding_antes_de_parar_ou_limpar_pares() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let before = std::fs::read(p.peers_path()).unwrap();
    let guard = p.reserve_operation(true).unwrap();
    let e=err(&ui,json!({"cmd":"onboarding_create","args":{"name":"novo","modules":{"input":false,"clipboard":true},"port":47800,"recreate":true}})).await;
    assert_eq!(e.code, "profile_busy");
    assert_eq!(app_state(&ui).await["agent"], "in_app");
    assert_eq!(std::fs::read(p.peers_path()).unwrap(), before);
    drop(guard);
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn reserva_recusa_join_ui_antes_de_retornar_tentativa() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let guard = p.reserve_operation(true).unwrap();
    let e = err(
        &ui,
        json!({"cmd":"pair_join_start","args":{"addr":"127.0.0.1:9"}}),
    )
    .await;
    assert_eq!(e.code, "profile_busy");
    drop(guard);
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn restaura_trava_true_e_false_antes_de_confirmar_restart() {
    for locked in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (_p, _) = init_profile(d.path(), "p", BOTH);
        let ui = start(d.path(), "p").await;
        ok(
            &ui,
            json!({"cmd":"lock_to_screen","args":{"locked":locked}}),
        )
        .await;
        let r = ok(&ui, request(true, false)).await;
        assert_eq!(r["effects"]["lock_restore"], "confirmed");
        assert_eq!(r["observed"]["screen_locked"], locked);
        assert_eq!(r["observed"]["paused"], true);
        ui.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn schema_recusa_campos_incompletos_desconhecidos_e_vazio() {
    let d = tempfile::tempdir().unwrap();
    let (_p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    for bad in [
        json!({"input":true}),
        json!({"input":true,"clipboard":true,"extra":1}),
        json!({"input":1,"clipboard":true}),
    ] {
        let mut req = request(false, true);
        req["args"]["modules"] = bad;
        assert_eq!(err(&ui, req).await.code, "bad_request");
    }
    let e = err(&ui, request(false, false)).await;
    assert_eq!(e.details.unwrap()["effects"]["pause"], "not_requested");
    ui.shutdown().await;
}

#[tokio::test(flavor = "multi_thread")]
async fn preflight_invalido_nao_pausa_nem_grava_proprio_ou_parado() {
    for stopped in [false, true] {
        let d = tempfile::tempdir().unwrap();
        let (p, _) = init_profile(d.path(), "p", BOTH);
        let operation = p.reserve_operation(true).unwrap();
        let mut edit = p.begin_config_edit(&operation, BOTH).unwrap();
        edit.set_modules(CLIPBOARD_ONLY).unwrap();
        drop(edit);
        drop(operation);
        let text = std::fs::read_to_string(p.config_path())
            .unwrap()
            .replace("capture = \"fake\"", "capture = \"invalido-fake-privado\"");
        std::fs::write(p.config_path(), text).unwrap();
        let ui = start(d.path(), "p").await;
        if stopped {
            ok(&ui, json!({"cmd":"agent_stop"})).await;
        }
        let before = std::fs::read(p.config_path()).unwrap();
        let e = err(&ui,json!({"cmd":"modules_set","args":{"modules":{"input":true,"clipboard":true},"expected":{"input":false,"clipboard":true}}})).await;
        let message = e.message;
        let details = e.details.unwrap();
        for action in ["pause", "stop", "start"] {
            assert_eq!(details["effects"][action], "not_requested");
        }
        assert_eq!(details["effects"]["config_saved"], false);
        assert!(!message.contains("invalido-fake-privado"));
        assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
        assert!(!p.state_path().exists());
        assert_eq!(
            app_state(&ui).await["agent"],
            if stopped { "stopped" } else { "in_app" }
        );
        ui.shutdown().await;
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn falha_runtime_depois_de_save_reporta_config_salva_e_pausa() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    // Configuração válida; a porta ocupada só impede o bind real do novo runtime.
    let occupied = std::net::UdpSocket::bind("127.0.0.1:0").unwrap();
    let text = std::fs::read_to_string(p.config_path()).unwrap().replace(
        &p.load_config().unwrap().listen.to_string(),
        &occupied.local_addr().unwrap().to_string(),
    );
    std::fs::write(p.config_path(), text).unwrap();
    let e = err(&ui, request(false, true)).await;
    let details = e.details.unwrap();
    assert_eq!(details["effects"]["config_saved"], true);
    assert_eq!(details["effects"]["stop"], "confirmed");
    assert_eq!(details["effects"]["start"], "failed");
    assert_eq!(details["observed"]["paused"], true);
    assert!(!p.load_config().unwrap().modules.input);
    assert!(p.reserve_operation(true).is_ok());
    ui.shutdown().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn resposta_malformada_nao_autoriza_gravar_ou_iniciar() {
    use tokio::io::AsyncWriteExt;
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let listener = tokio::net::UnixListener::bind(p.socket_path()).unwrap();
    let server = tokio::spawn(async move {
        while let Ok((mut socket, _)) = listener.accept().await {
            let _ = socket.write_all(&[0, 0, 0, 3, b'x', b'x', b'x']).await;
        }
    });
    let ui = start(d.path(), "p").await;
    let before = std::fs::read(p.config_path()).unwrap();
    let e = err(&ui, request(false, true)).await;
    let details = e.details.unwrap();
    assert_eq!(details["effects"]["config_saved"], false);
    assert_eq!(details["effects"]["pause"], "not_requested");
    assert_eq!(details["observed"]["agent"], "unknown");
    assert_eq!(std::fs::read(p.config_path()).unwrap(), before);
    ui.shutdown().await;
    server.abort();
    let _ = server.await;
}

#[tokio::test(flavor = "multi_thread")]
async fn tentativa_host_reserva_perfil_ate_cancelamento_real() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let attempt = ok(
        &ui,
        json!({"cmd":"pair_host_start","args":{"port":0,"window_secs":120}}),
    )
    .await;
    let error = err(&ui, request(false, true)).await;
    assert_eq!(error.code, "profile_busy");
    assert!(!p.state_path().exists());
    ok(
        &ui,
        json!({"cmd":"pair_cancel","args":{"attempt_id":attempt["attempt_id"]}}),
    )
    .await;
    wait_until("reserva de pairing liberada", || async {
        p.reserve_operation(true).is_ok()
    })
    .await;
    ok(&ui, request(false, true)).await;
    ui.shutdown().await;
}

#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn acesso_negado_ao_controle_nao_prova_ausencia() {
    use std::os::unix::fs::PermissionsExt;
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let listener = tokio::net::UnixListener::bind(p.socket_path()).unwrap();
    std::fs::set_permissions(p.socket_path(), std::fs::Permissions::from_mode(0o000)).unwrap();
    let ui = start(d.path(), "p").await;
    let original = std::fs::read(p.config_path()).unwrap();
    let error = err(&ui, request(false, true)).await;
    let details = error.details.unwrap();
    assert_eq!(details["observed"]["agent"], "unknown");
    assert_eq!(details["effects"]["config_saved"], false);
    assert_eq!(std::fs::read(p.config_path()).unwrap(), original);
    ui.shutdown().await;
    drop(listener);
}

#[tokio::test(flavor = "multi_thread")]
async fn preflight_modo_valida_outros_modulos_antes_de_pausar() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", BOTH);
    let ui = start(d.path(), "p").await;
    let text = std::fs::read_to_string(p.config_path())
        .unwrap()
        .replace("capture = \"fake\"", "capture = \"invalido-privado\"");
    std::fs::write(p.config_path(), &text).unwrap();
    let error = err(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"manual"}}),
    )
    .await;
    let details = error.details.unwrap();
    assert_eq!(details["effects"]["pause"], "not_requested");
    assert_eq!(details["effects"]["config_saved"], false);
    assert!(!error.message.contains("invalido-privado"));
    assert_eq!(std::fs::read_to_string(p.config_path()).unwrap(), text);
    assert!(!p.state_path().exists());
    ui.shutdown().await;
}

#[cfg(unix)]
async fn externo_perde_status(failure: u8) {
    use clipsync_core::control::{read_message, write_message};
    use clipsync_proto::{AgentStatus, ControlRequest, ControlResponse};
    use std::sync::{
        Arc,
        atomic::{AtomicU8, Ordering},
    };
    use tokio::io::AsyncWriteExt;
    let d = tempfile::tempdir().unwrap();
    let (p, id) = init_profile(d.path(), "p", CLIPBOARD_ONLY);
    p.set_paused(false).unwrap();
    let status = AgentStatus {
        device_name: "externo-fake".into(),
        id,
        listen: p.load_config().unwrap().listen,
        paused: true,
        peers: vec![],
        modules: vec![],
        recent_blocks: vec![],
        recent_removals: vec![],
    };
    let listener = tokio::net::UnixListener::bind(p.socket_path()).unwrap();
    let mode = Arc::new(AtomicU8::new(0));
    let server_mode = mode.clone();
    let server = tokio::spawn(async move {
        let mut tasks = tokio::task::JoinSet::new();
        loop {
            tokio::select! {
                accepted=listener.accept()=>{
                    let (mut stream,_)=accepted.unwrap();let mode=server_mode.clone();let status=status.clone();
                    tasks.spawn(async move {
                        let Ok(req)=read_message::<ControlRequest,_>(&mut stream,1024*1024).await else {return};
                        if matches!(req,ControlRequest::Status) {
                            match mode.load(Ordering::SeqCst) {
                                1=>{let _=write_message(&mut stream,&ControlResponse::Error{message:"estado não observado".into()},1024*1024).await;}
                                2=>{let _=stream.write_all(&[0,0,0,3,b'x',b'x',b'x']).await;}
                                3=>{std::future::pending::<()>().await;}
                                _=>{let _=write_message(&mut stream,&ControlResponse::Status(status),1024*1024).await;}
                            }
                        } else {
                            let _=write_message(&mut stream,&ControlResponse::Error{message:clipsync_core::msg::STATE_NOT_SAVED.into()},1024*1024).await;
                        }
                    });
                }
                Some(_)=tasks.join_next(), if !tasks.is_empty()=>{}
            }
        }
    });
    let ui = start(d.path(), "p").await;
    assert_eq!(app_state(&ui).await["agent"], "external");
    err(&ui, json!({"cmd":"pause_set","args":{"paused":true}})).await;
    let valid = app_state(&ui).await;
    assert_eq!(valid["paused"], true);
    assert!(valid["agent_error"].is_null());
    assert!(valid["pause_warning"].as_str().is_some());
    mode.store(failure, Ordering::SeqCst);
    let unknown = app_state(&ui).await;
    let has_error = unknown["agent_error"].as_str().is_some();
    let has_warning = unknown["pause_warning"].as_str().is_some();
    mode.store(0, Ordering::SeqCst);
    let recovered = app_state(&ui).await;
    assert_eq!(recovered["agent"], "external");
    assert_eq!(recovered["paused"], true);
    assert!(recovered["agent_error"].is_null());
    assert!(recovered["pause_warning"].as_str().is_some());
    ui.shutdown().await;
    server.abort();
    let _ = server.await;
    assert!(
        has_error,
        "falha externa não pode confirmar pausa persistida: {unknown}"
    );
    assert!(
        has_warning,
        "a falha de consulta não pode apagar aviso de persistência"
    );
}
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn externo_status_erro_preserva_desconhecido() {
    externo_perde_status(1).await;
}
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn externo_status_malformado_preserva_desconhecido() {
    externo_perde_status(2).await;
}
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn externo_status_timeout_preserva_desconhecido() {
    externo_perde_status(3).await;
}

#[tokio::test(flavor = "multi_thread")]
async fn preflight_modo_valida_candidato_e_nao_confunde_valor_invalido_com_default() {
    let d = tempfile::tempdir().unwrap();
    let (p, _) = init_profile(d.path(), "p", CLIPBOARD_ONLY);
    let ui = start(d.path(), "p").await;
    let original = std::fs::read_to_string(p.config_path()).unwrap();
    let mut raw: toml::Value = toml::from_str(&original).unwrap();
    raw["clipboard"]["mode"] = toml::Value::Integer(17);
    std::fs::write(p.config_path(), toml::to_string(&raw).unwrap()).unwrap();
    let reply = ok(
        &ui,
        json!({"cmd":"clipboard_set_mode","args":{"mode":"auto"}}),
    )
    .await;
    assert_eq!(reply["effects"]["config_saved"], true);
    assert_eq!(reply["observed"]["paused"], true);
    let saved = p.load_config().unwrap();
    assert_eq!(saved.clipboard.unwrap()["mode"].as_str(), Some("auto"));
    ui.shutdown().await;
}
