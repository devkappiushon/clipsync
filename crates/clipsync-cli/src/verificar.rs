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
    #[arg(long, conflicts_with = "ultima")]
    pub sessao: Option<u32>,
    #[arg(long)]
    pub ultima: bool,
    #[arg(long)]
    pub desde: Option<String>,
    #[arg(long)]
    pub json: bool,
}

pub(crate) struct Saida {
    pub tentativa: Option<u32>,
    pub sessao: Option<u32>,
    pub passos: Vec<Passo>,
    pub quebra: Option<String>,
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

pub(crate) fn executar(e: &Entrada) -> Result<Saida, FalhaVerificar> {
    let origem = ler(&e.origem)?;
    let destino = e.destino.as_deref().map(ler).transpose()?;
    let desde = e
        .desde
        .as_deref()
        .map(|v| {
            OffsetDateTime::parse(v, &Rfc3339)
                .map_err(|_| schema("--desde exige um instante RFC 3339 nesta versão"))
        })
        .transpose()?;
    let origem: Vec<_> = origem
        .iter()
        .filter(|r| desde.is_none_or(|ts| r.ts >= ts))
        .collect();
    let destino: Vec<_> = destino
        .iter()
        .flatten()
        .filter(|r| desde.is_none_or(|ts| r.ts >= ts))
        .collect();
    let session_open = origem.iter().rev().copied().find(|r| {
        r.evento == "session.open" && r.numero("session") == e.sessao && e.sessao.is_some()
    });
    let chosen = if e.ultima || e.sessao.is_none() {
        ultimo(&origem, &["attempt.begin", "barrier.activated"])
    } else {
        session_open.and_then(|s| {
            origem.iter().rev().copied().find(|r| {
                matches!(r.evento.as_str(), "attempt.begin" | "barrier.activated")
                    && r.numero("attempt") == s.numero("attempt")
            })
        })
    };
    let tentativa = chosen.and_then(|r| r.numero("attempt"));
    let opening = session_open.or_else(|| {
        tentativa.and_then(|n| {
            origem
                .iter()
                .rev()
                .copied()
                .find(|r| r.evento == "session.open" && r.numero("attempt") == Some(n))
        })
    });
    let sessao = e
        .sessao
        .or_else(|| opening.and_then(|r| r.numero("session")));
    let prerequisites: Vec<_> = origem
        .iter()
        .copied()
        .filter(|r| chosen.is_none_or(|c| (r.ts, r.indice) <= (c.ts, c.indice)))
        .collect();
    let remote: Vec<_> = destino
        .iter()
        .copied()
        .filter(|r| chosen.is_none_or(|c| r.ts <= c.ts))
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
    let remote_prepared = ultimo(&remote, &["inventory.prepared"]);
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
                && r.numero("barreiras").is_some_and(|n| n > 0) =>
        {
            passo("rotas", Estado::Ok, Some(r), None)
        }
        r => passo(
            "rotas",
            Estado::SemDado,
            r,
            Some("rotas e barreiras prontas não registradas"),
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
                .is_some_and(|b| !b.is_empty()) =>
        {
            passo("captura", Estado::Ok, Some(r), None)
        }
        r => passo(
            "captura",
            Estado::SemDado,
            r,
            Some("captura com barreiras não registrada"),
        ),
    };
    let filtered = tentativa.and_then(|n| {
        origem
            .iter()
            .rev()
            .copied()
            .find(|r| r.evento == "attempt.filtered" && r.numero("attempt") == Some(n))
    });
    let attempt = match (chosen, filtered) {
        (_, Some(r)) => passo("tentativa", Estado::Falhou, Some(r), r.texto("motivo")),
        (Some(r), None) => passo("tentativa", Estado::Ok, Some(r), None),
        _ => passo("tentativa", Estado::SemDado, None, Some(NO_ATTEMPT)),
    };
    let passos = vec![local, remote_inventory, routes, capture, attempt];
    let quebra = passos
        .iter()
        .find(|p| p.estado == Estado::Falhou)
        .map(|p| p.nome.to_owned());
    Ok(Saida {
        tentativa,
        sessao,
        passos,
        quebra,
    })
}

impl Saida {
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
        lines.join("\n")
    }

    pub fn json(&self) -> Value {
        json!({"v":1,"tentativa":self.tentativa,"sessao":self.sessao,"passos":self.passos,"quebra":self.quebra})
    }
}
