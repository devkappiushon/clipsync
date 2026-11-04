//! Verificação offline: cada conclusão guarda a linha que a sustenta.
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fmt,
    path::{Path, PathBuf},
};
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const NO_ATTEMPT: &str = "nenhuma tentativa registrada: a origem nunca viu o cursor na barreira";

#[derive(clap::Args)]
pub(crate) struct Entrada {
    #[arg(long)]
    pub origem: PathBuf,
    #[arg(long)]
    pub destino: Option<PathBuf>,
    #[arg(long, conflicts_with = "ultima", value_parser = clap::value_parser!(u32).range(1..))]
    pub sessao: Option<u32>,
    #[arg(long)]
    pub ultima: bool,
    #[arg(long, value_parser = validar_desde)]
    pub desde: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub(crate) struct Saida {
    pub tentativa: Option<u32>,
    pub sessao: Option<u32>,
    pub passos: Vec<Passo>,
    pub quebra: Option<String>,
    pub parada: Option<String>,
    houve_tentativa: bool,
}

#[derive(Serialize)]
pub(crate) struct Passo {
    pub nome: &'static str,
    pub estado: Estado,
    pub linha: Option<String>,
    pub motivo: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Estado {
    Ok,
    Falhou,
    SemDado,
}

#[derive(Debug)]
pub(crate) enum TipoFalha {
    Arquivo,
    Esquema,
}

#[derive(Debug)]
pub(crate) struct FalhaVerificar {
    pub tipo: TipoFalha,
    detalhe: String,
}

impl fmt::Display for FalhaVerificar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tipo = match self.tipo {
            TipoFalha::Arquivo => "arquivo",
            TipoFalha::Esquema => "esquema",
        };
        write!(f, "erro de {tipo}: {}", self.detalhe)
    }
}
impl std::error::Error for FalhaVerificar {}

fn schema(detalhe: impl Into<String>) -> FalhaVerificar {
    FalhaVerificar {
        tipo: TipoFalha::Esquema,
        detalhe: detalhe.into(),
    }
}

struct Registro {
    ts: OffsetDateTime,
    indice: usize,
    linha: String,
    evento: String,
    dados: Value,
}
impl Registro {
    fn numero(&self, key: &str) -> Option<u32> {
        self.dados.get(key)?.as_u64()?.try_into().ok()
    }
    fn texto(&self, key: &str) -> Option<&str> {
        self.dados.get(key)?.as_str()
    }
}

fn ler(path: &Path) -> Result<Vec<Registro>, FalhaVerificar> {
    let text = std::fs::read_to_string(path).map_err(|error| FalhaVerificar {
        tipo: TipoFalha::Arquivo,
        detalhe: format!("{}: {error}", path.display()),
    })?;
    let mut old = false;
    let mut records = Vec::new();
    for (indice, line) in text.lines().enumerate() {
        let location = format!("{}:{}", path.display(), indice + 1);
        let dados: Value = serde_json::from_str(line)
            .map_err(|_| schema(format!("JSON inválido em {location}")))?;
        let version = dados
            .get("v")
            .and_then(Value::as_u64)
            .ok_or_else(|| schema(format!("versão ausente ou inválida em {location}")))?;
        if version > 1 {
            return Err(schema(format!(
                "versão {version} ainda não suportada em {location}"
            )));
        }
        old |= version < 1;
        let ts = dados
            .get("ts")
            .and_then(Value::as_str)
            .and_then(|v| OffsetDateTime::parse(v, &Rfc3339).ok())
            .ok_or_else(|| schema(format!("instante inválido em {location}")))?;
        let evento = dados
            .get("evento")
            .and_then(Value::as_str)
            .ok_or_else(|| schema(format!("evento ausente em {location}")))?
            .to_owned();
        records.push(Registro {
            ts,
            indice,
            linha: line.into(),
            evento,
            dados,
        });
    }
    if old {
        eprintln!(
            "aviso: esquema anterior a v1 em {}; usando os eventos conhecidos",
            path.display()
        );
    }
    records.sort_by_key(|r| (r.ts, r.indice));
    Ok(records)
}

fn ultimo<'a>(records: &[&'a Registro], names: &[&str]) -> Option<&'a Registro> {
    records
        .iter()
        .rev()
        .copied()
        .find(|r| names.contains(&r.evento.as_str()))
}

fn passo(
    nome: &'static str,
    estado: Estado,
    registro: Option<&Registro>,
    motivo: Option<&str>,
) -> Passo {
    Passo {
        nome,
        estado,
        linha: registro.map(|r| r.linha.clone()),
        motivo: motivo.map(str::to_owned),
    }
}

fn inventario(nome: &'static str, registro: Option<&Registro>) -> Passo {
    match registro {
        Some(r) if r.numero("monitores") == Some(0) => passo(
            nome,
            Estado::Falhou,
            Some(r),
            Some("nenhum monitor no inventário"),
        ),
        Some(r) if r.numero("monitores").is_some() => passo(nome, Estado::Ok, Some(r), None),
        other => passo(
            nome,
            Estado::SemDado,
            other,
            Some("inventário de monitores não registrado"),
        ),
    }
}

const GAP: &str = "sem registro: passo não alcançado ou log truncado";

fn validar_desde(value: &str) -> Result<String, String> {
    if OffsetDateTime::parse(value, &Rfc3339).is_ok()
        || humantime::parse_duration(value)
            .ok()
            .and_then(|v| time::Duration::try_from(v).ok())
            .is_some()
    {
        Ok(value.to_owned())
    } else {
        Err("use um instante RFC 3339 com fuso ou uma duração, como 30m ou 2h".into())
    }
}

fn corte(
    value: Option<&str>,
    records: &[Registro],
) -> Result<Option<OffsetDateTime>, FalhaVerificar> {
    let Some(value) = value else { return Ok(None) };
    if let Ok(ts) = OffsetDateTime::parse(value, &Rfc3339) {
        return Ok(Some(ts));
    }
    let duration = humantime::parse_duration(value)
        .ok()
        .and_then(|v| time::Duration::try_from(v).ok())
        .ok_or_else(|| schema("--desde inválido"))?;
    Ok(records.last().and_then(|r| r.ts.checked_sub(duration)))
}

fn inicio(r: &Registro) -> bool {
    matches!(r.evento.as_str(), "attempt.begin" | "barrier.activated")
}
fn evento_sessao(r: &Registro) -> bool {
    matches!(
        r.evento.as_str(),
        "session.open"
            | "session.sent"
            | "session.active"
            | "session.refused"
            | "session.closed"
            | "datagram.first"
            | "return.received"
            | "return.failed"
    ) && r.numero("session").is_some_and(|s| s != 0)
}

struct Selecao<'a> {
    marco: Option<&'a Registro>,
    inicio: Option<&'a Registro>,
    eventos: Vec<&'a Registro>,
    eventos_sessao: Vec<&'a Registro>,
    fim: Option<&'a Registro>,
    tentativa: Option<u32>,
    sessao: Option<u32>,
    houve_tentativa: bool,
}

fn selecionar<'a>(records: &[&'a Registro], e: &Entrada) -> Selecao<'a> {
    let current_boot = records
        .iter()
        .rposition(|r| r.evento == "agent.up")
        .unwrap_or(0);
    let anchor = if !e.ultima
        && let Some(session) = e.sessao
    {
        records
            .iter()
            .rposition(|r| r.evento == "session.open" && r.numero("session") == Some(session))
            .or_else(|| {
                records
                    .iter()
                    .position(|r| evento_sessao(r) && r.numero("session") == Some(session))
            })
    } else {
        records
            .iter()
            .enumerate()
            .rev()
            .find(|(i, r)| {
                *i >= current_boot
                    && (inicio(r)
                        || matches!(r.evento.as_str(), "attempt.filtered" | "session.open"))
            })
            .map(|(i, _)| i)
            .or_else(|| {
                records
                    .iter()
                    .enumerate()
                    .rev()
                    .find(|(i, r)| *i >= current_boot && evento_sessao(r))
                    .map(|(i, _)| i)
            })
    };
    let Some(anchor) = anchor else {
        return Selecao {
            marco: None,
            inicio: None,
            eventos: vec![],
            eventos_sessao: vec![],
            fim: None,
            tentativa: None,
            sessao: e.sessao,
            houve_tentativa: e.sessao.is_some(),
        };
    };
    let chosen = records[anchor];
    let mut start = anchor;
    // O contador reinicia com o processo. Nunca procurar por id através de outra
    // tentativa, fechamento ou inicialização; a correlação inclui o intervalo.
    if chosen.evento != "attempt.begin" {
        for i in (0..anchor).rev() {
            let r = records[i];
            if r.evento == "agent.up"
                || (r.evento == "session.closed"
                    && chosen.numero("session").map_or_else(
                        || {
                            r.numero("attempt")
                                .is_none_or(|a| Some(a) == chosen.numero("attempt"))
                        },
                        |session| r.numero("session") == Some(session),
                    ))
            {
                break;
            }
            if inicio(r) || r.evento == "attempt.filtered" {
                if chosen.numero("attempt").is_some()
                    && r.numero("attempt") == chosen.numero("attempt")
                {
                    start = i;
                    if r.evento == "attempt.begin" {
                        break;
                    }
                } else {
                    break;
                }
            }
            if r.evento == "session.open"
                && chosen.numero("session").is_some()
                && r.numero("session") != chosen.numero("session")
            {
                break;
            }
        }
    }
    let end = records
        .iter()
        .enumerate()
        .skip(anchor + 1)
        .find(|(_, r)| r.evento == "attempt.begin" || r.evento == "agent.up")
        .map_or(records.len(), |(i, _)| i);
    let tentativa = records[start]
        .numero("attempt")
        .or_else(|| chosen.numero("attempt"));
    // Um fechamento por switch pode entrar nesta janela depois do begin novo.
    // O attempt explícito de outro ciclo não serve para inferir sua sessão.
    let eventos: Vec<_> = records[start..end]
        .iter()
        .copied()
        .filter(|r| {
            tentativa.is_none_or(|wanted| r.numero("attempt").is_none_or(|actual| actual == wanted))
        })
        .collect();
    let begin = eventos.iter().copied().find(|r| inicio(r));
    let opening = ultimo(&eventos, &["session.open"]);
    let sessao = e
        .sessao
        .or_else(|| opening.and_then(|r| r.numero("session")))
        .or_else(|| {
            eventos
                .iter()
                .find(|r| evento_sessao(r))
                .and_then(|r| r.numero("session"))
        });
    // A próxima tentativa pode preceder o fechamento por switch. Ela limita
    // abertura e filtros da tentativa, mas não a vida da sessão selecionada.
    let peer = opening.and_then(|r| r.texto("peer")).or_else(|| {
        eventos
            .iter()
            .find(|r| r.numero("session") == sessao && sessao.is_some())
            .and_then(|r| r.texto("peer"))
    });
    let mut eventos_sessao = Vec::new();
    let mut fim = None;
    for r in records[start..].iter().copied() {
        if r.evento == "agent.up" {
            fim = Some(r);
            break;
        }
        if sessao.is_some()
            && r.numero("session") == sessao
            && peer.is_none_or(|p| r.texto("peer").is_none_or(|v| v == p))
        {
            eventos_sessao.push(r);
            if r.evento == "session.closed" {
                fim = Some(r);
                break;
            }
        }
    }
    Selecao {
        marco: Some(records[start]),
        inicio: begin,
        tentativa: begin.or(opening).unwrap_or(chosen).numero("attempt"),
        sessao,
        eventos,
        eventos_sessao,
        fim,
        houve_tentativa: true,
    }
}

fn exigir(nome: &'static str, records: &[&Registro], events: &[&str], missing: &str) -> Passo {
    if events
        .iter()
        .all(|event| ultimo(records, &[event]).is_some())
    {
        passo(nome, Estado::Ok, ultimo(records, events), None)
    } else {
        passo(nome, Estado::SemDado, None, Some(missing))
    }
}

/// Uma coincidência do número não identifica a sessão de outro par/execução.
fn remotos_da_sessao<'a>(
    s: &Selecao<'_>,
    records: &[&'a Registro],
    origem: Option<&str>,
    destino: Option<&str>,
) -> (Vec<&'a Registro>, bool) {
    let (Some(start), Some(origem), Some(destino)) = (s.marco, origem, destino) else {
        return (Vec::new(), false);
    };
    let Some((boot_at, boot)) = records
        .iter()
        .enumerate()
        .rev()
        .find(|(_, r)| r.evento == "agent.up" && r.ts <= start.ts)
    else {
        return (Vec::new(), false);
    };
    if origem.is_empty() || destino.is_empty() || boot.texto("id") != Some(destino) {
        return (Vec::new(), false);
    }
    let end = records
        .iter()
        .enumerate()
        .skip(boot_at + 1)
        .find(|(_, r)| r.evento == "agent.up")
        .map_or(records.len(), |(i, _)| i);
    let events = records[boot_at..end]
        .iter()
        .copied()
        .filter(|r| {
            s.sessao.is_some()
                && r.numero("session") == s.sessao
                && r.texto("peer") == Some(origem)
                && r.ts >= start.ts
                && s.fim.is_none_or(|end| {
                    if end.evento == "agent.up" {
                        r.ts < end.ts
                    } else {
                        r.ts <= end.ts
                    }
                })
        })
        .collect();
    (events, true)
}

fn cadeia(
    s: &Selecao<'_>,
    remote: &[&Registro],
    destino_informado: bool,
    execucao_comprovada: bool,
) -> Vec<Passo> {
    let local = &s.eventos_sessao;
    let missing_remote = if !destino_informado {
        "arquivo do destino não informado"
    } else if !execucao_comprovada {
        "execução não comprovada: faltam identidades de agent.up compatíveis com o par selecionado"
    } else {
        "evento do par e da sessão ausente na janela do destino: pode não ter sido alcançado, o horário pode divergir ou o arquivo ter sido rotacionado"
    };
    let closed = ultimo(local, &["session.closed"]);
    let session = match closed.filter(|r| r.texto("motivo") == Some("error")) {
        Some(r) => passo(
            "sessao",
            Estado::Falhou,
            Some(r),
            r.texto("erro").or(Some("error")),
        ),
        None => exigir(
            "sessao",
            local,
            &["session.open", "session.sent"],
            "abertura e envio da sessão não registrados",
        ),
    };
    let refusal = ultimo(local, &["session.refused"])
        .or_else(|| ultimo(remote, &["enter.refused"]))
        .or_else(|| closed.filter(|r| r.texto("motivo") == Some("refused")));
    let entry = match refusal {
        Some(r) => passo(
            "entrada_destino",
            Estado::Falhou,
            Some(r),
            r.texto("motivo").or(Some("refused")),
        ),
        None => exigir(
            "entrada_destino",
            remote,
            &["enter.received"],
            missing_remote,
        ),
    };
    let emulation = match ultimo(remote, &["emulate.error"]) {
        Some(r) => passo(
            "emulacao",
            Estado::Falhou,
            Some(r),
            r.texto("motivo").or(r.texto("erro")),
        ),
        None => exigir(
            "emulacao",
            remote,
            &["emulate.start", "emulate.positioned"],
            missing_remote,
        ),
    };
    let both = |nome, here, there| match (ultimo(local, &[here]), ultimo(remote, &[there])) {
        (Some(r), Some(_)) => passo(nome, Estado::Ok, Some(r), Some("confirmado nos dois lados")),
        _ => passo(
            nome,
            Estado::SemDado,
            None,
            Some("evento não registrado nos dois lados"),
        ),
    };
    let finish = match closed {
        Some(r)
            if matches!(
                r.texto("motivo"),
                Some(
                    "return"
                        | "refused"
                        | "error"
                        | "pause"
                        | "lock"
                        | "stop"
                        | "peer_down"
                        | "switch"
                        | "inventory"
                )
            ) =>
        {
            passo("fechamento", Estado::Ok, Some(r), r.texto("motivo"))
        }
        r => passo(
            "fechamento",
            Estado::SemDado,
            r,
            Some("fechamento com motivo conhecido não registrado"),
        ),
    };
    let retorno =
        match ultimo(remote, &["return.failed"]).or_else(|| ultimo(local, &["return.failed"])) {
            Some(r) => passo(
                "retorno",
                Estado::Falhou,
                Some(r),
                r.texto("erro").or(Some("falha ao enviar retorno")),
            ),
            None => both("retorno", "return.received", "return.sent"),
        };
    let mut steps = vec![
        session,
        entry,
        emulation,
        both("datagramas", "datagram.first", "datagram.first"),
        retorno,
        finish,
    ];
    if let Some(reason) = closed.and_then(|r| r.texto("motivo")).filter(|reason| {
        matches!(
            *reason,
            "pause" | "lock" | "stop" | "peer_down" | "switch" | "inventory"
        )
    }) && let Some(step) = steps.iter_mut().find(|p| p.estado == Estado::SemDado)
    {
        step.motivo = Some(format!(
            "{}; sessão interrompida: {reason}",
            step.motivo.as_deref().unwrap_or("")
        ));
    }
    steps
}

pub(crate) fn executar(e: &Entrada) -> Result<Saida, FalhaVerificar> {
    let origem = ler(&e.origem)?;
    let destino = e.destino.as_deref().map(ler).transpose()?;
    let desde = corte(e.desde.as_deref(), &origem)?;
    let origem: Vec<_> = origem
        .iter()
        .filter(|r| desde.is_none_or(|ts| r.ts >= ts))
        .collect();
    let destino: Vec<_> = destino
        .iter()
        .flatten()
        .filter(|r| desde.is_none_or(|ts| r.ts >= ts))
        .collect();
    let selection = selecionar(&origem, e);
    let chosen = selection.marco;
    let opening = ultimo(&selection.eventos, &["session.open"]);
    let boot = origem.iter().rev().copied().find(|r| {
        r.evento == "agent.up" && chosen.is_none_or(|c| (r.ts, r.indice) <= (c.ts, c.indice))
    });
    let prerequisites: Vec<_> = origem
        .iter()
        .copied()
        .filter(|r| {
            chosen.is_none_or(|c| (r.ts, r.indice) <= (c.ts, c.indice))
                && boot.is_none_or(|b| (r.ts, r.indice) >= (b.ts, b.indice))
        })
        .collect();
    let remote_boot = destino
        .iter()
        .rev()
        .copied()
        .find(|r| r.evento == "agent.up" && chosen.is_none_or(|c| r.ts <= c.ts));
    let remote: Vec<_> = destino
        .iter()
        .copied()
        .filter(|r| {
            chosen.is_none_or(|c| r.ts <= c.ts)
                && remote_boot.is_none_or(|b| (r.ts, r.indice) >= (b.ts, b.indice))
        })
        .collect();
    let peer = opening
        .and_then(|r| r.texto("peer"))
        .or_else(|| chosen.and_then(|r| r.texto("peer")))
        .or_else(|| ultimo(&prerequisites, &["inventory.sent"]).and_then(|r| r.texto("peer")));
    let peer_events: Vec<_> = prerequisites
        .iter()
        .copied()
        .filter(|r| peer.is_none_or(|p| r.texto("peer") == Some(p)))
        .collect();
    let received = ultimo(&peer_events, &["inventory.received", "inventory.missing"]);
    let local = inventario(
        "inventario_origem",
        ultimo(&prerequisites, &["inventory.prepared"]),
    );
    let remote_identified =
        peer.is_some_and(|p| !p.is_empty() && remote_boot.and_then(|r| r.texto("id")) == Some(p));
    let remote_prepared = remote_identified
        .then(|| ultimo(&remote, &["inventory.prepared"]))
        .flatten();
    let remote_inventory = match received {
        Some(r) if r.evento == "inventory.missing" => passo(
            "inventario_destino",
            Estado::Falhou,
            Some(r),
            Some("a origem não recebeu o inventário do par"),
        ),
        _ if e.destino.is_none() => passo(
            "inventario_destino",
            Estado::SemDado,
            None,
            Some("arquivo do destino não informado"),
        ),
        _ if !remote_identified => passo(
            "inventario_destino",
            Estado::SemDado,
            None,
            Some("execução não comprovada: agent.up do destino não identifica o par selecionado"),
        ),
        None if remote_prepared.is_some() => passo(
            "inventario_destino",
            Estado::SemDado,
            remote_prepared,
            Some("preparado no destino, não recebido na origem"),
        ),
        _ => inventario("inventario_destino", remote_prepared),
    };
    let routes = match ultimo(&prerequisites, &["routes.built", "routes.empty"]) {
        Some(r) if r.evento == "routes.empty" => passo(
            "rotas",
            Estado::Falhou,
            Some(r),
            r.texto("motivo").or(Some("nenhuma rota disponível")),
        ),
        Some(r)
            if r.numero("rotas").is_some_and(|n| n > 0)
                && r.numero("barreiras").is_some_and(|n| n > 0)
                && peer.is_none_or(|p| {
                    r.dados
                        .get("pares")
                        .and_then(Value::as_array)
                        .is_some_and(|peers| peers.iter().any(|v| v.as_str() == Some(p)))
                }) =>
        {
            passo("rotas", Estado::Ok, Some(r), None)
        }
        r => passo(
            "rotas",
            Estado::SemDado,
            r,
            Some("rotas e barreiras prontas para o par selecionado não registradas"),
        ),
    };
    let capture = match ultimo(&prerequisites, &["capture.ready", "capture.failed"]) {
        Some(r) if r.evento == "capture.failed" => passo(
            "captura",
            Estado::Falhou,
            Some(r),
            r.texto("motivo").or(Some("falha ao abrir captura")),
        ),
        Some(r)
            if r.dados
                .get("barreiras")
                .and_then(Value::as_array)
                .is_some_and(|barriers| {
                    barriers.iter().any(|b| {
                        peer.is_none_or(|p| b.get("peer").and_then(Value::as_str) == Some(p))
                    })
                }) =>
        {
            passo("captura", Estado::Ok, Some(r), None)
        }
        r => passo(
            "captura",
            Estado::SemDado,
            r,
            Some("captura com barreiras para o par selecionado não registrada"),
        ),
    };
    let filtered = ultimo(&selection.eventos, &["attempt.filtered"]);
    let attempt = match (selection.inicio, filtered) {
        (_, Some(r)) => passo("tentativa", Estado::Falhou, Some(r), r.texto("motivo")),
        (Some(r), None) => passo("tentativa", Estado::Ok, Some(r), None),
        _ => passo(
            "tentativa",
            Estado::SemDado,
            None,
            Some(if selection.marco.is_none() && e.sessao.is_some() {
                "sessão pedida não comprovada"
            } else if selection.houve_tentativa {
                "início da tentativa não registrado"
            } else {
                NO_ATTEMPT
            }),
        ),
    };
    let mut passos = vec![local, remote_inventory, routes, capture, attempt];
    if selection.houve_tentativa {
        let (session_remote, execucao) =
            remotos_da_sessao(&selection, &destino, boot.and_then(|r| r.texto("id")), peer);
        passos.extend(cadeia(
            &selection,
            &session_remote,
            e.destino.is_some(),
            execucao,
        ));
    }
    let quebra = passos
        .iter()
        .find(|p| p.estado == Estado::Falhou)
        .map(|p| p.nome.to_owned());
    let parada = passos
        .iter()
        .find(|p| p.estado == Estado::SemDado)
        .map(|p| p.nome.to_owned());
    for p in &mut passos {
        if p.estado == Estado::SemDado {
            p.motivo = Some(format!("{}; {GAP}", p.motivo.as_deref().unwrap_or("")));
        }
    }
    Ok(Saida {
        tentativa: selection.tentativa,
        sessao: selection.sessao,
        passos,
        quebra,
        parada,
        houve_tentativa: selection.houve_tentativa,
    })
}

impl Saida {
    pub fn incompleta(&self) -> bool {
        self.quebra.is_some() || (self.houve_tentativa && self.parada.is_some())
    }

    pub fn texto(&self) -> String {
        let mut lines = Vec::new();
        for p in &self.passos {
            let estado = match p.estado {
                Estado::Ok => "ok",
                Estado::Falhou => "falhou",
                Estado::SemDado => "sem dado",
            };
            let mut line = format!("[{estado}] {}", p.nome);
            if let Some(motivo) = &p.motivo {
                line.push_str(": ");
                line.push_str(motivo);
            }
            if let Some(proof) = &p.linha {
                line.push_str(" | ");
                line.push_str(proof);
            }
            lines.push(line);
        }
        if let Some(quebra) = &self.quebra {
            lines.push(format!("primeiro ponto de quebra: {quebra}"));
        }
        if let Some(parada) = &self.parada {
            lines.push(format!("parada: {parada}; {GAP}"));
        }
        lines.join("\n")
    }

    pub fn json(&self) -> Value {
        json!({"v":1,"tentativa":self.tentativa,"sessao":self.sessao,"passos":self.passos,"quebra":self.quebra,"parada":self.parada})
    }
}
