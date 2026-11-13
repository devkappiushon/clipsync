//! Sonda manual do emulador EIS. Compilar não abre portal; executar abre o diálogo do GNOME.
//! Restauração é só uma tentativa: morte da sessão ou interrupção entre toques pode deixar
//! NumLock trocado. Confira o estado e ajuste manualmente ao final.
#[cfg(target_os = "linux")]
mod linux {
    use clipsync_input::{EmulateAdapter, PortalEmulate, TokenStore};
    use clipsync_proto::InputEvent;
    use std::error::Error;
    use std::path::PathBuf;
    use std::time::Duration;
    use tokio::time::Instant;

    type Result<T> = std::result::Result<T, Box<dyn Error>>;
    const NUM_LOCK: u32 = 69;
    const SECOND_TAP: Duration = Duration::from_secs(2);
    const LED_SETTLE: Duration = Duration::from_millis(300);

    #[derive(Clone, PartialEq, Eq)]
    struct Led {
        path: PathBuf,
        value: u8,
    }

    fn snapshot() -> Result<Vec<Led>> {
        let mut leds = Vec::new();
        for entry in std::fs::read_dir("/sys/class/leds")? {
            let entry = entry?;
            if !entry
                .file_name()
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("numlock")
            {
                continue;
            }
            let path = entry.path().join("brightness");
            let value = std::fs::read_to_string(&path)?.trim().parse::<u8>()?;
            if value > 1 {
                return Err(std::io::Error::other("indicador NumLock não binário").into());
            }
            leds.push(Led { path, value });
        }
        leds.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(leds)
    }

    fn show(stage: &str, leds: &[Led]) {
        println!("{stage}");
        for led in leds {
            println!("  {}: {}", led.path.display(), led.value);
        }
    }

    async fn tap(emu: &mut PortalEmulate) -> Result<()> {
        emu.emit(&[
            InputEvent::Key {
                code: NUM_LOCK,
                pressed: true,
            },
            InputEvent::Key {
                code: NUM_LOCK,
                pressed: false,
            },
        ])
        .await?;
        Ok(())
    }

    async fn restore(emu: &mut PortalEmulate, initial: &[Led]) -> Result<()> {
        let current = snapshot()?;
        let mut same = true;
        let mut opposite = true;
        for before in initial {
            let Some(after) = current.iter().find(|led| led.path == before.path) else {
                return Err(std::io::Error::other(
                    "indicador inicial desapareceu; restauração não verificável",
                )
                .into());
            };
            same &= before.value == after.value;
            opposite &= before.value != after.value;
        }
        if !same {
            if !opposite {
                return Err(std::io::Error::other(
                    "indicadores divergentes; não há restauração única verificável",
                )
                .into());
            }
            tap(emu).await?;
            tokio::time::sleep(LED_SETTLE).await;
        }
        let final_state = snapshot()?;
        show("estado final", &final_state);
        if !initial
            .iter()
            .all(|before| final_state.iter().any(|after| after == before))
        {
            return Err(std::io::Error::other("NumLock não voltou ao estado inicial").into());
        }
        println!("estado inicial restaurado e conferido nos indicadores iniciais");
        Ok(())
    }

    pub async fn run() -> Result<()> {
        tracing_subscriber::fmt()
            .with_target(false)
            .with_writer(std::io::stderr)
            .try_init()
            .map_err(|_| std::io::Error::other("não foi possível iniciar o log da sonda"))?;
        let initial = snapshot()?;
        if initial.is_empty() {
            return Err(std::io::Error::other(
                "nenhum indicador NumLock observável; sonda cancelada antes do portal",
            )
            .into());
        }
        if initial.iter().any(|led| led.value != initial[0].value) {
            return Err(std::io::Error::other(
                "indicadores iniciais divergentes; sonda cancelada antes do portal",
            )
            .into());
        }
        let profile = tempfile::Builder::new()
            .prefix("clipsync-eis-probe-")
            .tempdir()?;
        let mut emu = PortalEmulate::new(TokenStore::new(profile.path()));
        println!(
            "se a sessão morrer ou o processo for interrompido, NumLock pode ficar trocado; a restauração é só uma tentativa: confira e ajuste manualmente ao final"
        );
        println!("aceite o diálogo do GNOME; mantenha NumLock sem uso durante a sonda");
        show("antes do toque 1 (estado inicial)", &initial);
        if let Err(e) = emu.start().await {
            let _ = emu.stop().await;
            return Err(e.into());
        }
        let first = Instant::now();
        // Nenhuma espera ou leitura de arquivo entre start e o primeiro pedido de tecla.
        let sequence: Result<()> = async {
            let first_result = tap(&mut emu).await;
            if let Err(e) = &first_result {
                eprintln!("toque 1 não enviado: {e}");
            }
            tokio::time::sleep(LED_SETTLE).await;
            show("depois do toque 1", &snapshot()?);
            tokio::time::sleep_until(first + SECOND_TAP).await;
            show("antes do toque 2", &snapshot()?);
            let second_result = tap(&mut emu).await;
            if let Err(e) = &second_result {
                eprintln!("toque 2 não enviado: {e}");
            }
            tokio::time::sleep(LED_SETTLE).await;
            show("depois do toque 2", &snapshot()?);
            first_result?;
            second_result?;
            Ok(())
        }
        .await;
        let released = emu.release_all().await;
        let restored = restore(&mut emu, &initial).await;
        if let Err(e) = &restored {
            eprintln!("restauração não confirmada: {e}");
        }
        let stopped = emu.stop().await;
        sequence?;
        released?;
        restored?;
        stopped?;
        Ok(())
    }
}

#[cfg(target_os = "linux")]
#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    linux::run().await
}

#[cfg(not(target_os = "linux"))]
fn main() {
    eprintln!("esta sonda exige Linux com GNOME e portal RemoteDesktop");
}
